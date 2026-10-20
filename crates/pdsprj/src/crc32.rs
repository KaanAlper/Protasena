//! CRC-32 (IEEE 802.3, reflected, polynomial 0xEDB88320) as used by ZIP.

const fn make_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        // Evaluated at compile time: an out-of-range index would be a build error, never a
        // runtime panic.
        #[allow(clippy::indexing_slicing)]
        {
            table[i] = c;
        }
        i += 1;
    }
    table
}

static TABLE: [u32; 256] = make_table();

/// The CRC-32 of `data`.
pub fn crc32(data: &[u8]) -> u32 {
    !data.iter().fold(!0u32, |crc, &b| {
        let entry = TABLE.get(usize::from((crc as u8) ^ b)).copied().unwrap_or(0);
        entry ^ (crc >> 8)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_values() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b"The quick brown fox jumps over the lazy dog"), 0x414F_A339);
    }
}
