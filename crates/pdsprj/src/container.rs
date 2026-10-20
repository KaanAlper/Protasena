//! The `.pdsprj` container: a plain ZIP archive (stored or deflated members, no ZIP64).
//!
//! The reader works from the central directory at the end of the archive, checks every offset
//! and size against the data and the [`limits`](crate::limits), inflates on demand with an
//! output cap and verifies each member's CRC-32.

use std::io::Read;
use std::path::Path;

use crate::crc32::crc32;
use crate::cursor::Cursor;
use crate::error::Error;
use crate::limits::{MAX_ARCHIVE_BYTES, MAX_MEMBER_BYTES, MAX_MEMBERS, MAX_RATIO, RATIO_FREE_BYTES};

const EOCD_SIG: [u8; 4] = *b"PK\x05\x06";
const CENTRAL_SIG: [u8; 4] = *b"PK\x01\x02";
const LOCAL_SIG: [u8; 4] = *b"PK\x03\x04";
const EOCD_LEN: usize = 22;
const LOCAL_HEADER_LEN: usize = 30;
const MAX_COMMENT: usize = 0xffff;
const FLAG_ENCRYPTED: u16 = 1;

/// How a member's data is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Uncompressed.
    Stored,
    /// Deflate (raw, as ZIP uses it).
    Deflated,
    /// Any other method (the reader refuses it).
    Other(u16),
}

impl Method {
    fn from_code(code: u16) -> Self {
        match code {
            0 => Self::Stored,
            8 => Self::Deflated,
            other => Self::Other(other),
        }
    }
}

/// One member as the archive's directory describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// Path inside the archive (`ROOT.DSN`, `SCRIPTS/PWRRAILS.DAT`).
    pub name: String,
    /// Storage method.
    pub method: Method,
    /// Size in the archive.
    pub compressed_size: u64,
    /// Size once extracted.
    pub size: u64,
    /// CRC-32 of the extracted data.
    pub crc32: u32,
    flags: u16,
    local_offset: u64,
}

/// An opened archive: its bytes and its member list.
#[derive(Debug, Clone)]
pub struct Container {
    bytes: Vec<u8>,
    members: Vec<Member>,
}

impl Container {
    /// Reads an archive from disk (at most [`MAX_ARCHIVE_BYTES`]).
    ///
    /// # Errors
    /// [`Error::Io`], [`Error::TooLarge`] and everything [`Container::from_bytes`] returns.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let io = |e: std::io::Error| Error::Io { path: path.to_owned(), message: e.to_string() };
        let file = std::fs::File::open(path).map_err(io)?;
        let mut bytes = Vec::new();
        file.take(MAX_ARCHIVE_BYTES.saturating_add(1)).read_to_end(&mut bytes).map_err(io)?;
        Self::from_bytes(bytes)
    }

    /// Opens an archive held in memory.
    ///
    /// # Errors
    /// [`Error::TooLarge`], [`Error::NotZip`], [`Error::Unsupported`],
    /// [`Error::TooManyMembers`], [`Error::DuplicateMember`].
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if size > MAX_ARCHIVE_BYTES {
            return Err(Error::TooLarge { what: "archive".into(), size, limit: MAX_ARCHIVE_BYTES });
        }
        let members = read_directory(&bytes)?;
        Ok(Self { bytes, members })
    }

    /// The members in directory order.
    pub fn members(&self) -> &[Member] {
        &self.members
    }

    /// The member called `name` (compared without case).
    pub fn member(&self, name: &str) -> Option<&Member> {
        self.members.iter().find(|m| m.name.eq_ignore_ascii_case(name))
    }

    /// The extracted data of the member called `name` (compared without case).
    ///
    /// # Errors
    /// [`Error::MissingMember`], [`Error::Unsupported`] (encrypted, unknown method),
    /// [`Error::TooLarge`], [`Error::SuspiciousRatio`], [`Error::Corrupt`].
    pub fn read(&self, name: &str) -> Result<Vec<u8>, Error> {
        let member = self.member(name).ok_or_else(|| Error::MissingMember(name.to_owned()))?;
        extract(&self.bytes, member)
    }
}

fn not_zip(message: &str) -> Error {
    Error::NotZip(message.to_owned())
}

/// Finds the end-of-central-directory record: the last signature whose comment length reaches
/// exactly the end of the data.
fn find_eocd(bytes: &[u8]) -> Option<usize> {
    let lowest = bytes.len().saturating_sub(EOCD_LEN.saturating_add(MAX_COMMENT));
    let last = bytes.len().checked_sub(EOCD_LEN)?;
    (lowest..=last).rev().find(|&pos| {
        let mut c = Cursor::new(bytes, pos);
        c.array::<4>() == Some(EOCD_SIG)
            && c.skip(16).is_some()
            && c.u16().map(usize::from).and_then(|n| pos.checked_add(EOCD_LEN)?.checked_add(n)) == Some(bytes.len())
    })
}

fn read_directory(bytes: &[u8]) -> Result<Vec<Member>, Error> {
    let eocd = find_eocd(bytes).ok_or_else(|| not_zip("no end-of-directory record"))?;
    let mut c = Cursor::new(bytes, eocd.saturating_add(4));
    let short = || not_zip("truncated end-of-directory record");
    let (disk, cd_disk) = (c.u16().ok_or_else(short)?, c.u16().ok_or_else(short)?);
    let (on_disk, total) = (c.u16().ok_or_else(short)?, c.u16().ok_or_else(short)?);
    let (cd_size, cd_offset) = (c.u32().ok_or_else(short)?, c.u32().ok_or_else(short)?);
    if total == 0xffff || cd_size == u32::MAX || cd_offset == u32::MAX {
        return Err(Error::Unsupported("ZIP64 archives".into()));
    }
    if disk != 0 || cd_disk != 0 || on_disk != total {
        return Err(Error::Unsupported("archives split over several disks".into()));
    }
    let count = usize::from(total);
    if count > MAX_MEMBERS {
        return Err(Error::TooManyMembers { count, limit: MAX_MEMBERS });
    }
    let start = usize::try_from(cd_offset).map_err(|_| not_zip("directory offset out of range"))?;
    let end = start.checked_add(usize::try_from(cd_size).unwrap_or(usize::MAX));
    if end.is_none_or(|e| e > eocd) {
        return Err(not_zip("the directory lies outside the archive"));
    }
    let mut c = Cursor::new(bytes, start);
    let mut members: Vec<Member> = Vec::with_capacity(count);
    for _ in 0..count {
        let member = read_entry(&mut c).ok_or_else(|| not_zip("damaged directory entry"))?;
        if members.iter().any(|m| m.name.eq_ignore_ascii_case(&member.name)) {
            return Err(Error::DuplicateMember(member.name));
        }
        members.push(member);
    }
    Ok(members)
}

fn read_entry(c: &mut Cursor<'_>) -> Option<Member> {
    if c.array::<4>()? != CENTRAL_SIG {
        return None;
    }
    c.skip(4)?; // version made by, version needed
    let flags = c.u16()?;
    let method = Method::from_code(c.u16()?);
    c.skip(4)?; // time, date
    let crc32 = c.u32()?;
    let compressed_size = u64::from(c.u32()?);
    let size = u64::from(c.u32()?);
    let (name_len, extra_len, comment_len) = (c.u16()?, c.u16()?, c.u16()?);
    c.skip(8)?; // disk, internal and external attributes
    let local_offset = u64::from(c.u32()?);
    let name = String::from_utf8_lossy(c.bytes(usize::from(name_len))?).into_owned();
    c.skip(usize::from(extra_len).checked_add(usize::from(comment_len))?)?;
    if name.is_empty() || name.contains('\0') {
        return None;
    }
    Some(Member { name, method, compressed_size, size, crc32, flags, local_offset })
}

fn corrupt(member: &Member, message: &str) -> Error {
    Error::Corrupt { name: member.name.clone(), message: message.to_owned() }
}

fn extract(bytes: &[u8], member: &Member) -> Result<Vec<u8>, Error> {
    if member.flags & FLAG_ENCRYPTED != 0 {
        return Err(Error::Unsupported(format!("{} is encrypted", member.name)));
    }
    if member.size > MAX_MEMBER_BYTES {
        return Err(Error::TooLarge { what: member.name.clone(), size: member.size, limit: MAX_MEMBER_BYTES });
    }
    if member.size > RATIO_FREE_BYTES && member.size / member.compressed_size.max(1) > MAX_RATIO {
        return Err(Error::SuspiciousRatio { name: member.name.clone(), compressed: member.compressed_size, size: member.size });
    }
    let data = member_data(bytes, member).ok_or_else(|| corrupt(member, "its data lies outside the archive"))?;
    let size = usize::try_from(member.size).map_err(|_| corrupt(member, "size out of range"))?;
    let out = match member.method {
        Method::Stored if data.len() == size => data.to_vec(),
        Method::Stored => return Err(corrupt(member, "stored size differs from the declared size")),
        Method::Deflated => {
            miniz_oxide::inflate::decompress_to_vec_with_limit(data, size).map_err(|_| corrupt(member, "deflate data is invalid or too long"))?
        }
        Method::Other(code) => return Err(Error::Unsupported(format!("{} uses compression method {code}", member.name))),
    };
    if out.len() != size {
        return Err(corrupt(member, "extracted size differs from the declared size"));
    }
    if crc32(&out) != member.crc32 {
        return Err(corrupt(member, "checksum mismatch"));
    }
    Ok(out)
}

/// The compressed bytes of a member, found through its local header.
fn member_data<'a>(bytes: &'a [u8], member: &Member) -> Option<&'a [u8]> {
    let start = usize::try_from(member.local_offset).ok()?;
    let mut c = Cursor::new(bytes, start);
    if c.array::<4>()? != LOCAL_SIG {
        return None;
    }
    c.skip(22)?;
    let (name_len, extra_len) = (usize::from(c.u16()?), usize::from(c.u16()?));
    let data_start = start.checked_add(LOCAL_HEADER_LEN)?.checked_add(name_len)?.checked_add(extra_len)?;
    let len = usize::try_from(member.compressed_size).ok()?;
    bytes.get(data_start..data_start.checked_add(len)?)
}
