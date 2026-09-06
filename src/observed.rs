//! The observed-edges file at the docs repo root: traffic something measured.

use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::frontmatter::string_field;

pub(crate) const OBSERVED_FILE: &str = "observed-edges.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ObservedEdge {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) last_seen: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Observed {
    pub(crate) generated_at: Option<String>,
    pub(crate) edges: Vec<ObservedEdge>,
    pub(crate) warnings: Vec<String>,
}

/// `None` when the clone has no `observed-edges.json`; never fails, because a
/// file the exporter mangled is a warning beside the index, not a broken index.
pub(crate) fn read(clone: &Path) -> Option<Observed> {
    let path = clone.join(OBSERVED_FILE);
    if !path.is_file() {
        return None;
    }
    match fs::read_to_string(&path) {
        Ok(text) => Some(parse(&text)),
        Err(e) => Some(Observed {
            warnings: vec![format!("{OBSERVED_FILE}: {e}")],
            ..Observed::default()
        }),
    }
}

pub(crate) fn parse(text: &str) -> Observed {
    let value: Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(e) => {
            return Observed {
                warnings: vec![format!("{OBSERVED_FILE}: not valid JSON: {e}")],
                ..Observed::default()
            };
        }
    };
    let Some(map) = value.as_object() else {
        return Observed {
            warnings: vec![format!("{OBSERVED_FILE}: top level is not an object")],
            ..Observed::default()
        };
    };
    // A string or nothing: `string_field` would stringify a JSON number here,
    // and the format says anything but a string reads as absent.
    let generated_at = map
        .get("generated_at")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|date| !date.is_empty())
        .map(str::to_string);
    let Some(items) = map.get("edges").and_then(Value::as_array) else {
        return Observed {
            generated_at,
            warnings: vec![format!("{OBSERVED_FILE}: 'edges' is missing or not a list")],
            ..Observed::default()
        };
    };
    let mut edges = Vec::new();
    let mut warnings = Vec::new();
    for (i, item) in items.iter().enumerate() {
        match edge_entry(item) {
            Ok(edge) => edges.push(edge),
            Err(reason) => warnings.push(format!("{OBSERVED_FILE}: edges[{i}] {reason}")),
        }
    }
    Observed {
        generated_at,
        edges,
        warnings,
    }
}

fn edge_entry(item: &Value) -> std::result::Result<ObservedEdge, String> {
    let Some(map) = item.as_object() else {
        return Err("is not a mapping".to_string());
    };
    let field = |key: &str| string_field(map, key).ok_or_else(|| format!("missing '{key}'"));
    let from = field("from")?;
    let to = field("to")?;
    let kind = field("kind")?;
    let name = field("name")?;
    Ok(ObservedEdge {
        from,
        to,
        kind: kind.to_ascii_lowercase(),
        name,
        last_seen: string_field(map, "last_seen"),
    })
}

/// The greatest `last_seen` wins when one route is reported twice, so ISO 8601
/// dates and timestamps sort correctly as strings and a dateless row never
/// displaces a dated one.
pub(crate) fn latest(current: Option<String>, candidate: &Option<String>) -> Option<String> {
    if *candidate > current {
        candidate.clone()
    } else {
        current
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::{Observed, latest, parse};

    const SAMPLE: &str = r#"{
  "generated_at": "2026-09-05",
  "edges": [
    {"from": "ingest-api", "to": "record-store", "kind": "HTTP", "name": "GET /records", "last_seen": "2026-09-05"}
  ]
}"#;

    #[test]
    fn s16_a_well_formed_file_parses_rows_and_lowercases_kind() {
        let observed = parse(SAMPLE);
        assert!(observed.warnings.is_empty(), "{:?}", observed.warnings);
        assert_eq!(observed.generated_at.as_deref(), Some("2026-09-05"));
        assert_eq!(observed.edges.len(), 1);
        let edge = &observed.edges[0];
        assert_eq!(edge.from, "ingest-api");
        assert_eq!(edge.to, "record-store");
        assert_eq!(edge.kind, "http");
        assert_eq!(edge.name, "GET /records");
        assert_eq!(edge.last_seen.as_deref(), Some("2026-09-05"));
    }

    #[test]
    fn s16_invalid_json_is_one_warning_and_no_rows() {
        let observed = parse("{not json");
        assert!(observed.edges.is_empty());
        assert_eq!(observed.warnings.len(), 1, "{:?}", observed.warnings);
        assert!(
            observed.warnings[0].starts_with("observed-edges.json: not valid JSON"),
            "{:?}",
            observed.warnings
        );
    }

    #[test]
    fn s16_a_row_missing_a_field_is_skipped_with_a_warning() {
        let observed = parse(
            r#"{"edges": [
                {"from": "a", "to": "b", "kind": "http", "name": "GET /x"},
                {"from": "a", "kind": "http", "name": "GET /y"}
            ]}"#,
        );
        assert_eq!(observed.edges.len(), 1);
        assert_eq!(observed.edges[0].name, "GET /x");
        assert_eq!(observed.warnings.len(), 1, "{:?}", observed.warnings);
        assert!(
            observed.warnings[0].contains("edges[1] missing 'to'"),
            "{:?}",
            observed.warnings
        );
    }

    #[test]
    fn s16_last_seen_and_generated_at_are_optional() {
        let observed =
            parse(r#"{"edges": [{"from": "a", "to": "b", "kind": "sqs", "name": "n"}]}"#);
        assert!(observed.warnings.is_empty(), "{:?}", observed.warnings);
        assert_eq!(observed.generated_at, None);
        assert_eq!(observed.edges.len(), 1);
        assert_eq!(observed.edges[0].last_seen, None);
    }

    #[test]
    fn s16_a_non_string_generated_at_reads_as_absent() {
        for text in [
            r#"{"generated_at": 20260905, "edges": []}"#,
            r#"{"generated_at": ["2026-09-05"], "edges": []}"#,
            r#"{"generated_at": "   ", "edges": []}"#,
        ] {
            let observed = parse(text);
            assert_eq!(observed.generated_at, None, "{text}");
            assert!(observed.warnings.is_empty(), "{text}");
        }
    }

    #[test]
    fn s16_a_top_level_list_is_one_warning() {
        let observed = parse("[]");
        assert_eq!(
            observed,
            Observed {
                warnings: vec!["observed-edges.json: top level is not an object".to_string()],
                ..Observed::default()
            }
        );
    }

    #[test]
    fn s16_a_file_without_an_edges_list_is_one_warning() {
        for text in ["{}", r#"{"edges": 1}"#] {
            let observed = parse(text);
            assert!(observed.edges.is_empty(), "{text}");
            assert_eq!(
                observed.warnings,
                ["observed-edges.json: 'edges' is missing or not a list"],
                "{text}"
            );
        }
    }

    #[test]
    fn s16_the_later_last_seen_wins() {
        let older = Some("2026-09-04".to_string());
        let newer = Some("2026-09-05".to_string());
        assert_eq!(latest(older.clone(), &newer), newer);
        assert_eq!(latest(newer.clone(), &older), newer);
        assert_eq!(latest(None, &older), older);
        assert_eq!(latest(older.clone(), &None), older);
    }
}
