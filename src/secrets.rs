//! Secret-shaped strings: the six patterns the docs repo must never hold.

use std::sync::LazyLock;

use regex::Regex;

/// Name and pattern, spelled exactly as Capstone's map-check.sh spells them.
pub(crate) const PATTERNS: &[(&str, &str)] = &[
    ("aws-access-key", r"AKIA[0-9A-Z]{16}"),
    ("github-token", r"gh[pousr]_[A-Za-z0-9]{36,}"),
    ("slack-token", r"xox[abprs]-[A-Za-z0-9-]{10,}"),
    ("stripe-key", r"sk_(live|test)_[A-Za-z0-9]{16,}"),
    ("google-api-key", r"AIza[0-9A-Za-z_-]{35}"),
    ("private-key", r"-----BEGIN [A-Z ]*PRIVATE KEY-----"),
];

// A pattern that fails to compile drops out rather than taking the binary with
// it; s19_every_pattern_compiles is what keeps the list whole.
static COMPILED: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
    PATTERNS
        .iter()
        .filter_map(|(name, pattern)| Regex::new(pattern).ok().map(|re| (*name, re)))
        .collect()
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SecretHit {
    /// Repo-relative: the unit's docs dir plus the name inside it.
    pub(crate) file: String,
    pub(crate) pattern: &'static str,
}

/// The name of every pattern with at least one match, in table order, one entry
/// per pattern however many times it matched. Never the matched text.
pub(crate) fn scan(text: &str) -> Vec<&'static str> {
    COMPILED
        .iter()
        .filter(|(_, re)| re.is_match(text))
        .map(|(name, _)| *name)
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    const AWS: &str = "AKIAAAAAAAAAAAAAAAAA";
    const GITHUB: &str = "ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SLACK: &str = "xoxb-0000000000-aaaaaaaaaa";
    const STRIPE: &str = "sk_test_aaaaaaaaaaaaaaaa";
    const GOOGLE: &str = "AIzaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const PRIVATE: &str = "-----BEGIN RSA PRIVATE KEY-----";

    #[test]
    fn s19_every_pattern_compiles() {
        assert_eq!(COMPILED.len(), PATTERNS.len());
        assert_eq!(PATTERNS.len(), 6);
    }

    // The literals are duplicated on purpose: editing one copy of a pattern
    // without the other fails here, and the bash half is pinned the same way.
    #[test]
    fn s19_pattern_table_is_p8_verbatim() {
        assert_eq!(
            PATTERNS,
            &[
                ("aws-access-key", r"AKIA[0-9A-Z]{16}"),
                ("github-token", r"gh[pousr]_[A-Za-z0-9]{36,}"),
                ("slack-token", r"xox[abprs]-[A-Za-z0-9-]{10,}"),
                ("stripe-key", r"sk_(live|test)_[A-Za-z0-9]{16,}"),
                ("google-api-key", r"AIza[0-9A-Za-z_-]{35}"),
                ("private-key", r"-----BEGIN [A-Z ]*PRIVATE KEY-----"),
            ]
        );
    }

    #[test]
    fn s19_each_pattern_is_found_by_name() {
        assert_eq!(GOOGLE.len(), 39);
        for (name, sample) in [
            ("aws-access-key", AWS),
            ("github-token", GITHUB),
            ("slack-token", SLACK),
            ("stripe-key", STRIPE),
            ("google-api-key", GOOGLE),
            ("private-key", PRIVATE),
        ] {
            assert_eq!(scan(sample), vec![name], "{name}");
        }
    }

    #[test]
    fn s19_clean_text_has_no_hits() {
        assert!(scan("generated_date: 2026-09-05\n| DATABASE_URL | <redacted> |\n").is_empty());
        assert!(scan("ghp_short").is_empty());
        assert!(scan("AKIA1234").is_empty());
        assert!(scan("").is_empty());
    }

    #[test]
    fn s19_one_hit_per_pattern_in_table_order() {
        let text = format!("{PRIVATE}\nkey one {AWS}\nkey two AKIABBBBBBBBBBBBBBBB\n");
        assert_eq!(scan(&text), vec!["aws-access-key", "private-key"]);
    }

    #[test]
    fn s19_fenced_text_still_counts() {
        let text = format!("before\n\n```sh\nexport TOKEN={GITHUB}\n```\n\nafter\n");
        assert_eq!(scan(&text), vec!["github-token"]);
    }
}
