//! A bounds-checked little-endian byte reader. Every read returns `None` past the end instead
//! of panicking, so parsers built on it can treat any input as hostile.

/// Reads from `data`, starting at `pos`.
#[derive(Debug, Clone)]
pub(crate) struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    /// A reader at `pos` (past the end is allowed: every read then fails).
    pub(crate) fn new(data: &'a [u8], pos: usize) -> Self {
        Self { data, pos }
    }

    /// The current position.
    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    /// The byte `ahead` bytes after the current position, without moving.
    pub(crate) fn peek(&self, ahead: usize) -> Option<u8> {
        self.data.get(self.pos.checked_add(ahead)?).copied()
    }

    /// The next `n` bytes.
    pub(crate) fn bytes(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let slice = self.data.get(self.pos..end)?;
        self.pos = end;
        Some(slice)
    }

    /// Skips `n` bytes.
    pub(crate) fn skip(&mut self, n: usize) -> Option<()> {
        self.bytes(n).map(|_| ())
    }

    /// The next `N` bytes as an array.
    pub(crate) fn array<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.bytes(N)?.try_into().ok()
    }

    pub(crate) fn u8(&mut self) -> Option<u8> {
        self.array::<1>().map(|[b]| b)
    }

    pub(crate) fn u16(&mut self) -> Option<u16> {
        self.array().map(u16::from_le_bytes)
    }

    pub(crate) fn u32(&mut self) -> Option<u32> {
        self.array().map(u32::from_le_bytes)
    }

    pub(crate) fn i32(&mut self) -> Option<i32> {
        self.array().map(i32::from_le_bytes)
    }

    /// A string with a one-byte length prefix.
    pub(crate) fn pstr(&mut self) -> Option<String> {
        let len = self.u8()?;
        self.bytes(usize::from(len)).map(latin1)
    }

    /// A NUL-terminated string of at most `max` bytes (the NUL is consumed, not returned).
    pub(crate) fn cstr(&mut self, max: usize) -> Option<String> {
        let rest = self.data.get(self.pos..)?;
        let len = rest.iter().take(max.saturating_add(1)).position(|&b| b == 0)?;
        let text = latin1(rest.get(..len)?);
        self.pos = self.pos.checked_add(len)?.checked_add(1)?;
        Some(text)
    }
}

/// Bytes as text, one character per byte (ISO 8859-1): never fails, keeps every byte.
pub(crate) fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}

/// Whether `s` is a plausible name: 1..=`max` printable ASCII characters.
pub(crate) fn is_name(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && s.bytes().all(|b| (0x20..0x7f).contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_little_endian_and_stops_at_the_end() {
        let data = [1, 0, 2, 0, 0, 0, 0xff, 3, b'a', b'b', b'c', b'x', 0, 9];
        let mut c = Cursor::new(&data, 0);
        assert_eq!(c.u16(), Some(1));
        assert_eq!(c.u32(), Some(2));
        assert_eq!(c.u8(), Some(0xff));
        assert_eq!(c.pstr().as_deref(), Some("abc"));
        assert_eq!(c.cstr(4).as_deref(), Some("x"));
        assert_eq!(c.peek(0), Some(9));
        assert_eq!(c.u16(), None);
        assert_eq!(c.pos(), 13);
        assert_eq!(Cursor::new(&data, usize::MAX).u8(), None);
        assert_eq!(Cursor::new(&data, 0).skip(usize::MAX), None);
    }

    #[test]
    fn cstr_respects_its_limit_and_needs_a_terminator() {
        assert_eq!(Cursor::new(b"abcd\0", 0).cstr(3), None);
        assert_eq!(Cursor::new(b"abc\0", 0).cstr(3).as_deref(), Some("abc"));
        assert_eq!(Cursor::new(b"abc", 0).cstr(10), None);
        assert_eq!(latin1(&[0x41, 0xe9]), "Aé");
        assert!(is_name("WIRE", 8) && !is_name("", 8) && !is_name("a\u{1}", 8) && !is_name("abc", 2));
    }
}
