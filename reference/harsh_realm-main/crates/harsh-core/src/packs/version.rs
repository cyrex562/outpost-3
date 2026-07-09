//! SemVer-like version parsing and constraint checks for packs.
//!
//! Ported from `src/harsh_realm/packs/version.py`.

/// Parse a `MAJOR.MINOR.PATCH` version string into three integers.
///
/// Returns an error if the string is not a valid pack version.
pub fn parse_version(version: &str) -> Result<(u32, u32, u32), String> {
    let parts: Vec<&str> = version.trim().split('.').collect();
    let err = || format!("Invalid pack version: {version:?}");
    if parts.len() != 3 {
        return Err(err());
    }
    let nums: Result<Vec<u32>, _> = parts.iter().map(|p| p.parse::<u32>()).collect();
    match nums {
        Ok(v) if v.len() == 3 && parts.iter().all(|p| !p.is_empty()) => Ok((v[0], v[1], v[2])),
        _ => Err(err()),
    }
}

/// Whether `version` satisfies a pack dependency `constraint`.
///
/// Operators: `>=`, `<=`, `==`, `<`, `>`, and `~=` (compatible-release:
/// `~=1.2.3` means `>=1.2.3, <1.3.0`).
pub fn satisfies(version: &str, constraint: &str) -> Result<bool, String> {
    let v = parse_version(version)?;
    let c = constraint.trim();

    let (op, target_str) = if let Some(rest) = c.strip_prefix(">=") {
        (">=", rest)
    } else if let Some(rest) = c.strip_prefix("<=") {
        ("<=", rest)
    } else if let Some(rest) = c.strip_prefix("==") {
        ("==", rest)
    } else if let Some(rest) = c.strip_prefix("~=") {
        ("~=", rest)
    } else if let Some(rest) = c.strip_prefix('<') {
        ("<", rest)
    } else if let Some(rest) = c.strip_prefix('>') {
        (">", rest)
    } else {
        return Err(format!("Invalid pack version constraint: {constraint:?}"));
    };

    let target = parse_version(target_str)
        .map_err(|_| format!("Invalid pack version constraint: {constraint:?}"))?;

    Ok(match op {
        ">=" => v >= target,
        "<=" => v <= target,
        "==" => v == target,
        "<" => v < target,
        ">" => v > target,
        "~=" => {
            let (maj, min, _) = target;
            v >= target && v < (maj, min + 1, 0)
        }
        _ => unreachable!(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_rejects() {
        assert_eq!(parse_version("1.2.3").unwrap(), (1, 2, 3));
        assert!(parse_version("1.2").is_err());
        assert!(parse_version("1.2.x").is_err());
        assert!(parse_version("v1.2.3").is_err());
    }

    #[test]
    fn constraint_operators() {
        assert!(satisfies("1.2.3", ">=1.2.0").unwrap());
        assert!(!satisfies("1.2.3", "<1.2.0").unwrap());
        assert!(satisfies("1.2.3", "==1.2.3").unwrap());
        assert!(satisfies("1.2.9", "~=1.2.3").unwrap());
        assert!(!satisfies("1.3.0", "~=1.2.3").unwrap());
        assert!(satisfies("1.2.3", "~=1.2.3").unwrap());
        assert!(satisfies("invalid", ">=1.0.0").is_err());
        assert!(satisfies("1.0.0", "!!1.0.0").is_err());
    }
}
