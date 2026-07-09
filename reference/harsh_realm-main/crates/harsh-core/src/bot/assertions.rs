//! Assertion helpers for bot goal validation.
//!
//! Ported from `src/harsh_realm/bot/assertions.py`.

/// Return true if `substring` appears in `response` (case-insensitive).
pub fn assert_contains(response: &str, substring: &str) -> bool {
    response.to_lowercase().contains(&substring.to_lowercase())
}

/// Return true if `response` exactly equals `expected`.
pub fn assert_exact(response: &str, expected: &str) -> bool {
    response == expected
}

/// Return true if the fraction of true values in `results` meets `threshold`.
/// Returns false if `results` is empty.
pub fn assert_threshold(results: &[bool], threshold: f64) -> bool {
    if results.is_empty() {
        return false;
    }
    let ratio = results.iter().filter(|&&v| v).count() as f64 / results.len() as f64;
    ratio >= threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_case_insensitive() {
        assert!(assert_contains("You enter the FOREST", "forest"));
        assert!(!assert_contains("plains", "forest"));
    }

    #[test]
    fn exact_match() {
        assert!(assert_exact("ok", "ok"));
        assert!(!assert_exact("OK", "ok"));
    }

    #[test]
    fn threshold_empty_fails() {
        assert!(!assert_threshold(&[], 0.5));
    }

    #[test]
    fn threshold_all_true() {
        assert!(assert_threshold(&[true, true, true], 1.0));
    }

    #[test]
    fn threshold_partial() {
        assert!(assert_threshold(&[true, true, false], 0.6));
        assert!(!assert_threshold(&[true, false, false], 0.6));
    }
}
