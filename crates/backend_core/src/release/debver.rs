//! Pure Rust Debian version ordering (Debian Policy §5.6.12).

use std::cmp::Ordering;

/// Split a Debian version string into `(epoch, upstream_version, debian_revision)`.
pub fn parse(v: &str) -> (u64, &str, &str) {
    let v = v.trim();
    let (epoch, rest) = if let Some((epoch_str, rest)) = v.split_once(':') {
        let epoch = if !epoch_str.is_empty() && epoch_str.bytes().all(|b| b.is_ascii_digit()) {
            epoch_str.parse::<u64>().unwrap_or(0)
        } else {
            0
        };
        (epoch, rest)
    } else {
        (0, v)
    };

    let (upstream, revision) = if let Some((upstream, revision)) = rest.rsplit_once('-') {
        (upstream, revision)
    } else {
        (rest, "0")
    };

    (epoch, upstream, revision)
}

/// Compare two Debian version strings per Debian Policy §5.6.12.
pub fn compare(a: &str, b: &str) -> Ordering {
    if a.trim() == b.trim() {
        return Ordering::Equal;
    }
    let (e1, u1, r1) = parse(a);
    let (e2, u2, r2) = parse(b);
    e1.cmp(&e2)
        .then_with(|| compare_part(u1, u2))
        .then_with(|| compare_part(r1, r2))
}

fn char_weight(ch: Option<u8>) -> i32 {
    match ch {
        Some(b'~') => -2,
        None => -1,
        Some(c) if c.is_ascii_alphabetic() => i32::from(c),
        Some(c) => i32::from(c) + 256,
    }
}

fn strip_leading_zeros(digits: &[u8]) -> &[u8] {
    let first_non_zero = digits
        .iter()
        .position(|&b| b != b'0')
        .unwrap_or(digits.len());
    &digits[first_non_zero..]
}

fn compare_digit_runs(a: &[u8], b: &[u8]) -> Ordering {
    let sa = strip_leading_zeros(a);
    let sb = strip_leading_zeros(b);
    sa.len().cmp(&sb.len()).then_with(|| sa.cmp(sb))
}

fn compare_part(part_a: &str, part_b: &str) -> Ordering {
    let a = part_a.as_bytes();
    let b = part_b.as_bytes();
    let mut i = 0;
    let mut j = 0;

    while i < a.len() || j < b.len() {
        while (i < a.len() && !a[i].is_ascii_digit()) || (j < b.len() && !b[j].is_ascii_digit()) {
            let ca = if i < a.len() && !a[i].is_ascii_digit() {
                Some(a[i])
            } else {
                None
            };
            let cb = if j < b.len() && !b[j].is_ascii_digit() {
                Some(b[j])
            } else {
                None
            };
            let wa = char_weight(ca);
            let wb = char_weight(cb);
            if wa != wb {
                return wa.cmp(&wb);
            }
            if ca.is_some() {
                i += 1;
            }
            if cb.is_some() {
                j += 1;
            }
        }

        let start_i = i;
        while i < a.len() && a[i].is_ascii_digit() {
            i += 1;
        }
        let start_j = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }

        let cmp_digits = compare_digit_runs(&a[start_i..i], &b[start_j..j]);
        if cmp_digits != Ordering::Equal {
            return cmp_digits;
        }
    }

    Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEBVER_CASES: &str = include_str!("testdata/debver_cases.txt");

    #[test]
    fn parse_splits_epoch_upstream_and_revision() {
        assert_eq!(parse("1.0"), (0, "1.0", "0"));
        assert_eq!(parse("2:1.0-1"), (2, "1.0", "1"));
        assert_eq!(parse("1:2.3-foo-bar-4"), (1, "2.3-foo-bar", "4"));
        assert_eq!(
            parse("  0:7.3.3-x64v3-xanmod1  "),
            (0, "7.3.3-x64v3", "xanmod1")
        );
    }

    #[test]
    fn dpkg_verified_case_table_matches_and_swaps_symmetrically() {
        let mut count = 0usize;
        for (line_idx, raw_line) in DEBVER_CASES.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            assert_eq!(
                parts.len(),
                3,
                "invalid case format at line {}: {line}",
                line_idx + 1
            );
            let (a, op, b) = (parts[0], parts[1], parts[2]);
            let expected = match op {
                "lt" => Ordering::Less,
                "eq" => Ordering::Equal,
                "gt" => Ordering::Greater,
                other => panic!("unknown operator '{other}' at line {}", line_idx + 1),
            };

            let actual = compare(a, b);
            assert_eq!(
                actual,
                expected,
                "line {}: compare({a:?}, {b:?}) expected {expected:?}, got {actual:?}",
                line_idx + 1
            );

            let swapped = compare(b, a);
            let expected_swapped = expected.reverse();
            assert_eq!(
                swapped,
                expected_swapped,
                "line {}: swapped compare({b:?}, {a:?}) expected {expected_swapped:?}, got {swapped:?}",
                line_idx + 1
            );
            count += 1;
        }
        assert!(count >= 60, "expected at least 60 cases, found {count}");
    }
}
