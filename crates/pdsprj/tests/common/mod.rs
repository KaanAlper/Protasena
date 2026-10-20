//! Synthetic `.pdsprj` files for the tests: the structure Proteus writes, our own content.
#![allow(dead_code, clippy::indexing_slicing)]

pub mod dsn;
pub mod zip;

/// Our manifest. `TESTONLY` stands for the attributes the reader must skip without storing.
pub const MANIFEST: &str = "<?xml version='1.0' encoding='UTF-8'?>\n<PROJECT>\n <TIMESTAMP RELEASE=\"901\" FILEVER=\"902\" MODIFIED=\"2000\" \
                            TESTONLY=\"must-not-appear\" CREATED=\"1000\"/>\n <VARIANTS/>\n <SCRIPTS>\n  <STRING>RAILS.DAT</STRING>\n </SCRIPTS>\n</PROJECT>\n";

/// An archive with the four members Proteus writes, around `design`.
pub fn archive(design: &[u8]) -> Vec<u8> {
    zip::zip(
        &[
            zip::Entry::new("PROJECT.XML", MANIFEST.as_bytes(), true),
            zip::Entry::new("ROOT.CDB", &[5, 0, 0, 0, 1, 2, 3], true),
            zip::Entry::new("ROOT.DSN", design, true),
            zip::Entry::new("SCRIPTS/RAILS.DAT", b"*SECTION\n", false),
        ],
        b"",
    )
}

/// A small deterministic pseudo-random generator (xorshift64*).
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }

    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next() as u8).collect()
    }
}
