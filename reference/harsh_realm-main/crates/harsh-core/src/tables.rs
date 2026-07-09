//! Random-table selection — pure core of `harsh_realm.engine.tables`.
//!
//! The roll (RNG) stays with the host; this ports the deterministic *selection*
//! given a roll value: the ranged-table lookup and the weighted cumulative
//! selection. The weighted selection mirrors Python's `random.choices`
//! (cumulative weights + `bisect_right` over `[0, total)` with `hi = n-1`), so a
//! caller that supplies the same uniform draw gets the same index.

/// The maximum upper bound across ranged entries — the die size to roll, as in
/// the Python engine's `max_val` computation.
pub fn max_range(ranges: &[(i64, i64)]) -> i64 {
    ranges.iter().map(|(_, hi)| *hi).max().unwrap_or(0)
}

/// Index of the first entry whose inclusive `[min, max]` range contains `roll`.
pub fn select_by_range(ranges: &[(i64, i64)], roll: i64) -> Option<usize> {
    ranges
        .iter()
        .position(|(lo, hi)| *lo <= roll && roll <= *hi)
}

/// Weighted selection given a uniform draw `r` in `[0, total_weight)`.
///
/// Matches `random.choices`: build cumulative weights, then
/// `bisect_right(cum, r, 0, n-1)`.
pub fn weighted_select(weights: &[f64], r: f64) -> usize {
    if weights.is_empty() {
        return 0;
    }
    let mut cum = Vec::with_capacity(weights.len());
    let mut acc = 0.0;
    for w in weights {
        acc += *w;
        cum.push(acc);
    }
    bisect_right(&cum, r, 0, weights.len() - 1)
}

/// `bisect.bisect_right(a, x, lo, hi)` from the Python stdlib.
fn bisect_right(a: &[f64], x: f64, lo: usize, hi: usize) -> usize {
    let mut lo = lo;
    let mut hi = hi;
    while lo < hi {
        let mid = (lo + hi) / 2;
        if x < a[mid] {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    lo
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_range_finds_die_size() {
        assert_eq!(max_range(&[(1, 50), (51, 90), (91, 100)]), 100);
        assert_eq!(max_range(&[]), 0);
    }

    #[test]
    fn range_selection_first_match() {
        let ranges = [(1, 50), (51, 90), (91, 100)];
        assert_eq!(select_by_range(&ranges, 1), Some(0));
        assert_eq!(select_by_range(&ranges, 50), Some(0));
        assert_eq!(select_by_range(&ranges, 51), Some(1));
        assert_eq!(select_by_range(&ranges, 100), Some(2));
        assert_eq!(select_by_range(&[(1, 5)], 6), None);
    }

    #[test]
    fn weighted_selection_boundaries() {
        // cum = [1, 3, 6]; total 6.
        let w = [1.0, 2.0, 3.0];
        assert_eq!(weighted_select(&w, 0.0), 0); // < 1 → index 0
        assert_eq!(weighted_select(&w, 0.999), 0);
        assert_eq!(weighted_select(&w, 1.0), 1); // >= cum[0] → index 1
        assert_eq!(weighted_select(&w, 2.999), 1);
        assert_eq!(weighted_select(&w, 3.0), 2); // >= cum[1] → index 2
        assert_eq!(weighted_select(&w, 5.999), 2);
    }

    #[test]
    fn weighted_selection_clamps_to_last_via_hi() {
        // hi = n-1 means r at/over total still maps to the last index.
        assert_eq!(weighted_select(&[1.0, 1.0], 2.0), 1);
    }
}
