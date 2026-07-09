//! Effective-tag resolution — pure core of `harsh_realm.tags.service`.
//!
//! An entity's effective tags are the union of its static tags, the tags its
//! traits provide, and the tags its active statuses provide. (The Python service
//! does a plain union — no transitive `implies` processing — so this matches it.)

use std::collections::BTreeSet;

/// Union the static tags with all trait- and status-provided tags.
pub fn resolve_effective_tags<'a>(
    static_tags: impl IntoIterator<Item = &'a str>,
    trait_provided: impl IntoIterator<Item = &'a str>,
    status_provided: impl IntoIterator<Item = &'a str>,
) -> BTreeSet<String> {
    static_tags
        .into_iter()
        .chain(trait_provided)
        .chain(status_provided)
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unions_all_sources_and_dedups() {
        let tags = resolve_effective_tags(
            ["humanoid", "guard"],
            ["alert", "guard"], // 'guard' duplicated across sources
            ["poisoned"],
        );
        let expected: BTreeSet<String> = ["humanoid", "guard", "alert", "poisoned"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(tags, expected);
    }

    #[test]
    fn empty_sources_yield_empty_set() {
        let empty: [&str; 0] = [];
        assert!(resolve_effective_tags(empty, empty, empty).is_empty());
    }
}
