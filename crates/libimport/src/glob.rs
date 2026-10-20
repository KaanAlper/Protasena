//! Wildcard patterns: `*` matches any run of characters, `?` one character; everything else
//! matches itself. Used by the map's rules and by KiCad's footprint filters.

/// Whether `text` matches `pattern`; `ignore_case` compares ASCII letters without case.
pub fn matches(pattern: &str, text: &str, ignore_case: bool) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let eq = |a: char, b: char| if ignore_case { a.eq_ignore_ascii_case(&b) } else { a == b };
    // Iterative matcher with one backtrack point (the last `*`): linear in practice, never
    // recursive, so hostile patterns cannot exhaust the stack.
    let (mut pi, mut ti) = (0usize, 0usize);
    let mut star: Option<(usize, usize)> = None;
    while ti < t.len() {
        match (p.get(pi), t.get(ti)) {
            (Some('*'), _) => {
                star = Some((pi, ti));
                pi += 1;
            }
            (Some(&pc), Some(&tc)) if pc == '?' || eq(pc, tc) => {
                pi += 1;
                ti += 1;
            }
            _ => match star {
                Some((sp, st)) => {
                    pi = sp + 1;
                    ti = st + 1;
                    star = Some((sp, st + 1));
                }
                None => return false,
            },
        }
    }
    p.get(pi..).is_some_and(|rest| rest.iter().all(|&c| c == '*'))
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn wildcards() {
        assert!(matches("R_*", "R_0603_1608Metric", false));
        assert!(!matches("R_*", "C_0603", false));
        assert!(matches("*SOIC*3.9x4.9mm*P1.27mm*", "SOIC-8_3.9x4.9mm_P1.27mm", false));
        assert!(matches("DIP*W7.62mm*", "DIP-14_W7.62mm", false));
        assert!(matches("74LS??", "74LS00", false));
        assert!(!matches("74LS??", "74LS000", false));
        assert!(matches("*", "", false));
        assert!(matches("", "", false));
        assert!(!matches("", "a", false));
        assert!(matches("r_*", "R_1", true));
        assert!(!matches("r_*", "R_1", false));
        assert!(matches("a*b*c", "aXXbYYc", false));
        assert!(!matches("a*b*c", "aXXbYY", false));
    }

    #[test]
    fn hostile_patterns_finish() {
        let pattern = "*a".repeat(200) + "b";
        let text = "a".repeat(2000);
        assert!(!matches(&pattern, &text, false));
    }
}
