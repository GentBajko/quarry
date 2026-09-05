//! The YAML boundary, edge declarations, and heading sections.

use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Parsed {
    pub(crate) fields: Map<String, Value>,
    pub(crate) body: String,
    pub(crate) warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Section {
    pub(crate) heading: String,
    pub(crate) normalized: String,
    pub(crate) level: u8,
    pub(crate) body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EdgeDecl {
    pub(crate) other: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) produces: bool,
}

pub(crate) fn split(text: &str) -> (Option<&str>, &str) {
    let rest = match text.strip_prefix("---\n") {
        Some(rest) => rest,
        None => return (None, text),
    };
    let mut offset = 0usize;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\n', '\r']);
        if trimmed == "---" || trimmed == "..." {
            return (Some(&rest[..offset]), &rest[offset + line.len()..]);
        }
        offset += line.len();
    }
    (None, text)
}

pub(crate) fn parse(text: &str) -> Parsed {
    let (front, body) = split(text);
    let Some(front) = front else {
        return Parsed {
            fields: Map::new(),
            body: body.to_string(),
            warning: None,
        };
    };
    match serde_saphyr::from_str::<Value>(front) {
        Ok(Value::Object(fields)) => Parsed {
            fields,
            body: body.to_string(),
            warning: None,
        },
        Ok(Value::Null) => Parsed {
            fields: Map::new(),
            body: body.to_string(),
            warning: None,
        },
        Ok(_) => Parsed {
            fields: Map::new(),
            body: body.to_string(),
            warning: Some("frontmatter is not a mapping".to_string()),
        },
        Err(e) => Parsed {
            fields: Map::new(),
            body: body.to_string(),
            warning: Some(format!("frontmatter is not valid YAML: {e}")),
        },
    }
}

pub(crate) fn edges_of(fields: &Map<String, Value>) -> (Vec<EdgeDecl>, Vec<String>) {
    let mut edges = Vec::new();
    let mut warnings = Vec::new();
    for (key, produces) in [("produces", true), ("consumes", false)] {
        let Some(value) = fields.get(key) else {
            continue;
        };
        let Some(items) = value.as_array() else {
            warnings.push(format!("{key} is not a list"));
            continue;
        };
        for (i, item) in items.iter().enumerate() {
            match edge_entry(item, key, produces) {
                Ok(edge) => edges.push(edge),
                Err(reason) => warnings.push(format!("{key}[{i}] {reason}")),
            }
        }
    }
    (edges, warnings)
}

fn edge_entry(item: &Value, key: &str, produces: bool) -> std::result::Result<EdgeDecl, String> {
    let Some(map) = item.as_object() else {
        return Err("is not a mapping".to_string());
    };
    let other_key = if produces { "to" } else { "from" };
    let kind = string_field(map, "kind").ok_or_else(|| format!("missing '{}'", "kind"))?;
    let name = string_field(map, "name")
        .or_else(|| string_field(map, "endpoint"))
        .ok_or_else(|| "missing 'name'".to_string())?;
    let other =
        string_field(map, other_key).ok_or_else(|| format!("missing '{other_key}' in {key}"))?;
    Ok(EdgeDecl {
        other,
        kind: kind.to_ascii_lowercase(),
        name,
        produces,
    })
}

fn string_field(map: &Map<String, Value>, key: &str) -> Option<String> {
    match map.get(key) {
        Some(Value::String(s)) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

pub(crate) fn sections_of(body: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let mut fence: Option<String> = None;
    let mut current: Option<(String, u8, String)> = None;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if let Some(marker) = fence_marker(trimmed) {
            match &fence {
                Some(open) if trimmed.starts_with(open.as_str()) => fence = None,
                Some(_) => {}
                None => fence = Some(marker),
            }
        } else if fence.is_none()
            && let Some((level, heading)) = heading_of(trimmed)
        {
            if let Some((h, l, b)) = current.take() {
                sections.push(finish(h, l, b));
            }
            current = Some((heading, level, String::new()));
            continue;
        }
        if let Some((_, _, buffer)) = current.as_mut() {
            buffer.push_str(line);
            buffer.push('\n');
        }
    }
    if let Some((h, l, b)) = current.take() {
        sections.push(finish(h, l, b));
    }
    sections
}

fn finish(heading: String, level: u8, body: String) -> Section {
    Section {
        normalized: normalize_heading(&heading),
        heading,
        level,
        body: body.trim_end().to_string(),
    }
}

fn fence_marker(trimmed: &str) -> Option<String> {
    for marker in ["```", "~~~"] {
        if trimmed.starts_with(marker) {
            return Some(marker.to_string());
        }
    }
    None
}

fn heading_of(trimmed: &str) -> Option<(u8, String)> {
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.starts_with(' ') {
        return None;
    }
    let heading = rest.trim();
    if heading.is_empty() {
        return None;
    }
    Some((hashes as u8, heading.to_string()))
}

pub(crate) fn normalize_heading(heading: &str) -> String {
    let mut text = heading.trim();
    if let Some(open) = text.rfind("{#")
        && text.ends_with('}')
    {
        text = text[..open].trim_end();
    }
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    const PAGE: &str = "---\ngenerated_date: 2026-09-04\nproduces:\n  - kind: SQS\n    name: file-ingest\n    to: record-store\n    site: src/publish/sqs.py:57\nconsumes:\n  - kind: http\n    from: identity-api\n    endpoint: GET /customers/{id}\n---\n# Title\n\nintro\n\n## file-ingest (v2)\n\n| a | b |\n\n### deeper\n\nx\n\n## Next\n\ny\n";

    #[test]
    fn s6_frontmatter_parses_into_fields_and_body() {
        let parsed = parse(PAGE);
        assert_eq!(parsed.warning, None);
        assert_eq!(
            parsed.fields.get("generated_date").and_then(|v| v.as_str()),
            Some("2026-09-04")
        );
        assert!(parsed.body.starts_with("# Title"));
    }

    #[test]
    fn s6_edges_carry_direction_and_lowercased_kind() {
        let parsed = parse(PAGE);
        let (edges, warnings) = edges_of(&parsed.fields);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0].kind, "sqs");
        assert_eq!(edges[0].other, "record-store");
        assert!(edges[0].produces);
        assert_eq!(edges[1].name, "GET /customers/{id}");
        assert!(!edges[1].produces);
    }

    #[test]
    fn s6_entry_missing_a_required_field_is_skipped_with_a_warning() {
        let parsed = parse("---\nproduces:\n  - kind: sqs\n    name: x\n---\nbody\n");
        let (edges, warnings) = edges_of(&parsed.fields);
        assert!(edges.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("missing 'to'"), "{warnings:?}");
    }

    #[test]
    fn s6_invalid_yaml_becomes_a_warning_not_an_error() {
        let parsed = parse("---\na: [1,\n---\nbody\n");
        assert!(parsed.warning.is_some());
        assert_eq!(parsed.body, "body\n");
    }

    #[test]
    fn s8_sections_split_at_headings_and_stop_at_the_next_one() {
        let parsed = parse(PAGE);
        let sections = sections_of(&parsed.body);
        let headings: Vec<&str> = sections.iter().map(|s| s.heading.as_str()).collect();
        assert_eq!(headings, ["Title", "file-ingest (v2)", "deeper", "Next"]);
        let ingest = &sections[1];
        assert!(ingest.body.contains("| a | b |"));
        assert!(!ingest.body.contains("deeper"));
        assert_eq!(ingest.normalized, "file-ingest (v2)");
    }

    #[test]
    fn s8_headings_inside_fenced_blocks_are_not_sections() {
        let body = "# Real\n\n```\n# not a heading\n```\n\n## Also real\n";
        let headings: Vec<String> = sections_of(body).into_iter().map(|s| s.heading).collect();
        assert_eq!(headings, ["Real", "Also real"]);
    }

    #[test]
    fn s8_normalization_folds_case_and_whitespace() {
        assert_eq!(
            normalize_heading("  File-Ingest   (V2) {#anchor}"),
            "file-ingest (v2)"
        );
    }
}
