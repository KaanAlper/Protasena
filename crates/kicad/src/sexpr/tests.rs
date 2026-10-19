use super::*;
use crate::error::ErrorKind;

fn p(s: &str) -> Sexpr {
    parse(s.as_bytes()).unwrap()
}

fn err(s: &[u8]) -> crate::error::Error {
    parse(s).unwrap_err()
}

#[test]
fn parses_lists_atoms_and_strings_with_positions() {
    let t = p("(at 1.27 -2\n  \"a b\" (x))");
    assert_eq!(t.head(), Some("at"));
    let args = t.args();
    assert_eq!(args[0].as_atom(), Some("1.27"));
    assert_eq!(args[1].as_atom(), Some("-2"));
    assert!(matches!(&args[2], Sexpr::Str { text, .. } if text == "a b"));
    assert_eq!(args[2].pos(), Pos { line: 2, col: 3 });
    assert_eq!(args[3].head(), Some("x"));
    assert_eq!(args[3].pos(), Pos { line: 2, col: 9 });
}

#[test]
fn decodes_escapes_and_keeps_unknown_ones() {
    let t = p(r#"(s "q\"b\\n\n\t\x41\101\z")"#);
    assert_eq!(t.args()[0].text(), Some("q\"b\\n\n\tAA\\z"));
}

#[test]
fn columns_count_characters_not_bytes() {
    let t = p("(ä ö)");
    assert_eq!(t.args()[0].pos(), Pos { line: 1, col: 4 });
}

#[test]
fn skips_byte_order_mark() {
    assert_eq!(p("\u{feff}(a)").head(), Some("a"));
}

#[test]
fn display_round_trips() {
    let src = "(a \"x \\\"y\\\" \\\\ \n\" (b (c)) () d)";
    let t = p(src);
    let again = p(&t.to_string());
    assert_eq!(strip(&t), strip(&again));
    assert_eq!(t.to_string(), "(a \"x \\\"y\\\" \\\\ \\n\" (b (c)) () d)");
}

/// The tree without positions, for comparing round trips.
fn strip(t: &Sexpr) -> String {
    format!("{t}")
}

#[test]
fn errors_carry_line_and_column() {
    let e = err(b"(a\n  (b \"open)");
    assert_eq!(e.kind, ErrorKind::UnterminatedString);
    assert_eq!(e.pos, Pos { line: 2, col: 6 });
    assert_eq!(err(b"(a))").kind, ErrorKind::UnexpectedClose);
    assert_eq!(err(b"(a (b)").kind, ErrorKind::UnclosedList);
    assert_eq!(err(b"(a (b)").pos, Pos { line: 1, col: 1 });
    assert_eq!(err(b"").kind, ErrorKind::Empty);
    assert_eq!(err(b"(a) (b)").kind, ErrorKind::TrailingData);
    assert!(matches!(err(b"atom").kind, ErrorKind::Invalid(_)));
    let e = err(b"(a\n\xff)");
    assert_eq!((e.kind, e.pos), (ErrorKind::InvalidUtf8, Pos { line: 2, col: 1 }));
    assert!(e_display(b"(a))").contains("line 1, column 4"));
}

fn e_display(s: &[u8]) -> String {
    err(s).to_string()
}

#[test]
fn enforces_depth_limit() {
    let ok = format!("{}{}", "(".repeat(256), ")".repeat(256));
    assert!(parse(ok.as_bytes()).is_ok());
    let deep = format!("{}{}", "(".repeat(257), ")".repeat(257));
    assert_eq!(err(deep.as_bytes()).kind, ErrorKind::TooDeep { max: 256 });
    // Far deeper than any stack could recurse: still a clean error.
    let very = "(".repeat(1_000_000);
    assert_eq!(err(very.as_bytes()).kind, ErrorKind::TooDeep { max: 256 });
}

#[test]
fn enforces_size_limits() {
    let small = Limits { max_input: 10, max_depth: 4, max_atom: 3, max_nodes: 4 };
    let e = parse_with(b"(aaaaaaaaaaaa)", &small).unwrap_err();
    assert!(matches!(e.kind, ErrorKind::InputTooLarge { len: 14, max: 10 }));
    assert_eq!(parse_with(b"(abcd)", &small).unwrap_err().kind, ErrorKind::AtomTooLong { max: 3 });
    assert_eq!(parse_with(b"(\"abcd\")", &small).unwrap_err().kind, ErrorKind::AtomTooLong { max: 3 });
    assert_eq!(parse_with(b"(a b c d)", &small).unwrap_err().kind, ErrorKind::TooManyNodes { max: 4 });
    assert!(parse_with(b"(abc \"d\")", &small).is_ok());
    let big = vec![b' '; (64 << 20) + 1];
    assert!(matches!(err(&big).kind, ErrorKind::InputTooLarge { .. }));
    let long = format!("(\"{}\")", "x".repeat((1 << 20) + 1));
    assert!(matches!(err(long.as_bytes()).kind, ErrorKind::AtomTooLong { .. }));
}

/// A small deterministic generator (64-bit LCG), so failures reproduce.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

const ALPHABET: &[u8] = b"()()\"\"\\ \n\tab1.-_xyzhide\xc3\xa4\xff\x00";

#[test]
fn hostile_random_bytes_never_panic() {
    let mut rng = Lcg(0x5eed);
    let limits = Limits { max_input: 4096, max_depth: 16, max_atom: 64, max_nodes: 512 };
    for _ in 0..20_000 {
        let len = rng.below(200) as usize;
        let bytes: Vec<u8> =
            (0..len).map(|_| if rng.below(4) == 0 { rng.below(256) as u8 } else { ALPHABET[rng.below(ALPHABET.len() as u64) as usize] }).collect();
        if let Ok(trees) = parse_all_with(&bytes, &limits) {
            // Whatever parses must print and parse back to the same text.
            for t in trees {
                let s = t.to_string();
                assert_eq!(parse_all(s.as_bytes()).unwrap()[0].to_string(), s);
            }
        }
        let _ = crate::symbol::parse_symbol_lib(&bytes);
        let _ = crate::footprint::parse_footprint(&bytes);
    }
}

#[test]
fn hostile_garbled_kicad_never_panics() {
    let base = include_str!("../../tests/data/sample.kicad_sym").as_bytes().to_vec();
    let fp = include_str!("../../tests/data/sample.kicad_mod").as_bytes().to_vec();
    let mut rng = Lcg(42);
    for i in 0..4_000 {
        let mut b = if i % 2 == 0 { base.clone() } else { fp.clone() };
        for _ in 0..=rng.below(8) {
            let at = rng.below(b.len() as u64) as usize;
            match rng.below(3) {
                0 => b[at] = ALPHABET[rng.below(ALPHABET.len() as u64) as usize],
                1 => {
                    b.remove(at);
                }
                _ => b.insert(at, ALPHABET[rng.below(ALPHABET.len() as u64) as usize]),
            }
        }
        if let Ok(lib) = crate::symbol::parse_symbol_lib(&b) {
            let _ = lib.report();
            for s in &lib.symbols {
                let _ = lib.resolve(&s.name);
            }
        }
        if let Ok(f) = crate::footprint::parse_footprint(&b) {
            let _ = f.report();
        }
    }
}
