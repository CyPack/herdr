//! Natural ("human") ordering for file names: digit runs compare by value, so
//! `file2` sorts before `file10`.
//!
//! Vendored and adapted from yazi's `yazi-shared/src/natsort.rs`
//! (Copyright (c) 2023, sxyazi; MIT licensed), itself a Rust port of Martin
//! Pool's `strnatcmp.c` — <http://sourcefrog.net/projects/natsort/>.
//!
//! herdr adaptation: the two digit-run helpers were rewritten in safe Rust. The
//! upstream `unwrap_unchecked()` calls are replaced by an `if let` over the
//! already-checked `Some` values, so this module contains no `unsafe`. Behavior
//! is unchanged; the upstream test vectors are retained below as characterization
//! tests that prove the port stays faithful.

use std::cmp::Ordering;

/// Return early from the enclosing function unless the comparison is `Equal`.
macro_rules! return_unless_equal {
    ($ord:expr) => {
        match $ord {
            Ordering::Equal => {}
            ord => return ord,
        }
    };
}

/// Compare two leading digit runs when either has a leading zero, i.e. the
/// "fractional" case where a longer run is not automatically larger; digits are
/// compared position-by-position until they diverge or one run ends.
fn compare_left(left: &[u8], right: &[u8], li: &mut usize, ri: &mut usize) -> Ordering {
    loop {
        let l = left.get(*li);
        let r = right.get(*ri);
        match (
            l.is_some_and(|b| b.is_ascii_digit()),
            r.is_some_and(|b| b.is_ascii_digit()),
        ) {
            (true, true) => {
                // Both are `Some` here (`is_some_and` returned true), so this
                // `if let` always binds — no `unsafe` unwrap needed.
                if let (Some(&lb), Some(&rb)) = (l, r) {
                    return_unless_equal!(lb.cmp(&rb));
                }
            }
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            (false, false) => return Ordering::Equal,
        }
        *li += 1;
        *ri += 1;
    }
}

/// Compare two leading digit runs without leading zeros: the longer run is the
/// larger number, and among equal-length runs the first differing digit decides
/// (that first difference is remembered in `bias`).
fn compare_right(left: &[u8], right: &[u8], li: &mut usize, ri: &mut usize) -> Ordering {
    let mut bias = Ordering::Equal;
    loop {
        let l = left.get(*li);
        let r = right.get(*ri);
        match (
            l.is_some_and(|b| b.is_ascii_digit()),
            r.is_some_and(|b| b.is_ascii_digit()),
        ) {
            (true, true) => {
                if bias == Ordering::Equal {
                    // Both are `Some` here; safe direct comparison.
                    if let (Some(&lb), Some(&rb)) = (l, r) {
                        bias = lb.cmp(&rb);
                    }
                }
            }
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            (false, false) => return bias,
        }
        *li += 1;
        *ri += 1;
    }
}

/// Compare two byte strings in natural order. `insensitive` folds ASCII case.
pub fn natsort(left: &[u8], right: &[u8], insensitive: bool) -> Ordering {
    let mut li = 0;
    let mut ri = 0;

    let mut l = left.get(li);
    let mut r = right.get(ri);

    macro_rules! left_next {
        () => {{
            li += 1;
            l = left.get(li);
        }};
    }
    macro_rules! right_next {
        () => {{
            ri += 1;
            r = right.get(ri);
        }};
    }

    loop {
        while l.is_some_and(|c| c.is_ascii_whitespace()) {
            left_next!();
        }
        while r.is_some_and(|c| c.is_ascii_whitespace()) {
            right_next!();
        }

        match (l, r) {
            (Some(&ll), Some(&rr)) => {
                if ll.is_ascii_digit() && rr.is_ascii_digit() {
                    if ll == b'0' || rr == b'0' {
                        return_unless_equal!(compare_left(left, right, &mut li, &mut ri));
                    } else {
                        return_unless_equal!(compare_right(left, right, &mut li, &mut ri));
                    }

                    l = left.get(li);
                    r = right.get(ri);
                    continue;
                }

                if insensitive {
                    return_unless_equal!(ll.to_ascii_lowercase().cmp(&rr.to_ascii_lowercase()));
                } else {
                    return_unless_equal!(ll.cmp(&rr));
                }
            }
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (None, None) => return Ordering::Equal,
        }

        left_next!();
        right_next!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Assert `items` is already in natural (case-insensitive) order, i.e.
    /// sorting is a no-op. Mirrors yazi's upstream helper so the port stays
    /// faithful to the reference vectors.
    fn assert_sorted(items: &[&str]) {
        let mut sorted = items.to_vec();
        sorted.sort_by(|a, b| natsort(a.as_bytes(), b.as_bytes(), true));
        assert_eq!(items, sorted.as_slice());
    }

    // T-A1.1a: upstream reference vectors (dates / fractions / words).
    #[test]
    fn faithful_port_matches_upstream_vectors() {
        let dates = [
            "1999-3-3",
            "1999-12-25",
            "2000-1-2",
            "2000-1-10",
            "2000-3-23",
        ];
        let fractions = [
            "1.002.01", "1.002.03", "1.002.08", "1.009.02", "1.009.10", "1.009.20", "1.010.12",
            "1.011.02",
        ];
        let words = [
            "1-02",
            "1-2",
            "1-20",
            "10-20",
            "fred",
            "jane",
            "pic01",
            "pic02",
            "pic02a",
            "pic02000",
            "pic05",
            "pic2",
            "pic3",
            "pic4",
            "pic 4 else",
            "pic 5",
            "pic 5 ",
            "pic 5 something",
            "pic 6",
            "pic   7",
            "pic100",
            "pic100a",
            "pic120",
            "pic121",
            "tom",
            "x2-g8",
            "x2-y08",
            "x2-y7",
            "x8-y8",
        ];

        assert_sorted(&dates);
        assert_sorted(&fractions);
        assert_sorted(&words);
    }

    // T-A1.1b: edge inputs are total and panic-free.
    #[test]
    fn empty_and_singleton_inputs_are_total_and_panic_free() {
        assert_eq!(natsort(b"", b"", true), Ordering::Equal);
        assert_eq!(natsort(b"", b"a", true), Ordering::Less);
        assert_eq!(natsort(b"a", b"", true), Ordering::Greater);
        assert_eq!(natsort(b"a", b"a", true), Ordering::Equal);
    }

    // T-A1.1b: numeric runs compare by value, not lexically.
    #[test]
    fn numeric_runs_compare_by_value() {
        assert_eq!(natsort(b"file2", b"file10", true), Ordering::Less);
        assert_eq!(natsort(b"file10", b"file2", true), Ordering::Greater);
    }

    // T-A1.1b: case sensitivity flag behaves.
    #[test]
    fn case_folding_follows_the_insensitive_flag() {
        // Insensitive: identical when folded.
        assert_eq!(natsort(b"File", b"file", true), Ordering::Equal);
        // Sensitive: uppercase 'F' (0x46) sorts before lowercase 'f' (0x66).
        assert_eq!(natsort(b"File", b"file", false), Ordering::Less);
    }
}
