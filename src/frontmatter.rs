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
    pub(crate) site: Option<String>,
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

pub(crate) const INTERFACES_PAGE: &str = "09-interfaces.md";

pub(crate) const UNKNOWN_TARGET: &str = "unknown";

pub(crate) fn is_unknown_target(other: &str) -> bool {
    other.trim().eq_ignore_ascii_case(UNKNOWN_TARGET)
}

// The list as declared: trimmed, non-empty, first-occurrence deduped, original
// case. A bare scalar is a warning rather than a one-item list so a template
// bug upstream stays visible.
pub(crate) fn known_as_of(fields: &Map<String, Value>) -> (Vec<String>, Vec<String>) {
    let mut names: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let Some(value) = fields.get("known_as") else {
        return (names, warnings);
    };
    let Some(items) = value.as_array() else {
        warnings.push("known_as is not a list".to_string());
        return (names, warnings);
    };
    for (i, item) in items.iter().enumerate() {
        match item {
            Value::String(s) if !s.trim().is_empty() => {
                let s = s.trim().to_string();
                if !names.contains(&s) {
                    names.push(s);
                }
            }
            Value::String(_) => warnings.push(format!("known_as[{i}] is empty")),
            _ => warnings.push(format!("known_as[{i}] is not a string")),
        }
    }
    (names, warnings)
}

pub(crate) fn edges_of(
    path: &str,
    fields: &Map<String, Value>,
    body: &str,
) -> (Vec<EdgeDecl>, Vec<String>) {
    if fields.contains_key("produces") || fields.contains_key("consumes") {
        return edges_from_frontmatter(fields);
    }
    if path.rsplit('/').next() == Some(INTERFACES_PAGE) {
        return edges_from_tables(body);
    }
    (Vec::new(), Vec::new())
}

fn edges_from_frontmatter(fields: &Map<String, Value>) -> (Vec<EdgeDecl>, Vec<String>) {
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
        site: string_field(map, "site"),
    })
}

fn edges_from_tables(body: &str) -> (Vec<EdgeDecl>, Vec<String>) {
    let mut edges = Vec::new();
    let mut warnings = Vec::new();
    for section in sections_of(body) {
        let produces = match section.normalized.as_str() {
            "produces" => true,
            "consumes" => false,
            _ => continue,
        };
        let Some(table) = table_of(&section.body) else {
            continue;
        };
        let other_key = if produces { "to" } else { "from" };
        for (row, cells) in table.rows.iter().enumerate() {
            match table_entry(&table.headers, cells, other_key, produces) {
                Ok(edge) => edges.push(edge),
                Err(reason) => warnings.push(format!(
                    "{} table row {} {reason}",
                    section.heading,
                    row + 1
                )),
            }
        }
    }
    (edges, warnings)
}

struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

fn table_of(body: &str) -> Option<Table> {
    let unfenced = strip_fences(body);
    let mut lines = unfenced
        .into_iter()
        .skip_while(|l| !l.trim_start().starts_with('|'));
    let headers: Vec<String> = split_row(lines.next()?)
        .into_iter()
        .map(|c| c.to_lowercase())
        .collect();
    let separator = lines.next()?;
    if !separator.trim_start().starts_with('|') || !separator.contains('-') {
        return None;
    }
    let mut rows = Vec::new();
    for line in lines {
        if !line.trim_start().starts_with('|') {
            break;
        }
        rows.push(split_row(line));
    }
    Some(Table { headers, rows })
}

fn strip_fences(body: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut fence: Option<&str> = None;
    for line in body.lines() {
        let trimmed = line.trim_start();
        let marker = if trimmed.starts_with("```") {
            Some("```")
        } else if trimmed.starts_with("~~~") {
            Some("~~~")
        } else {
            None
        };
        match (marker, fence) {
            (Some(m), None) => fence = Some(m),
            (Some(m), Some(open)) if m == open => fence = None,
            _ => {
                if fence.is_none() {
                    out.push(line);
                }
            }
        }
    }
    out
}

fn split_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_start_matches('|')
        .trim_end_matches('|')
        .replace("\\|", "\u{0}")
        .split('|')
        .map(|cell| clean_cell(&cell.replace('\u{0}', "|")))
        .collect()
}

/// Removes one matched pair of surrounding backticks only, so a cell holding
/// two separate inline-code spans keeps the backticks that separate them.
fn unbacktick(text: &str) -> &str {
    text.strip_prefix('`')
        .and_then(|rest| rest.strip_suffix('`'))
        .unwrap_or(text)
}

fn clean_cell(cell: &str) -> String {
    let mut text = unbacktick(cell.trim()).trim().to_string();
    if let Some(open) = text.find('[')
        && let Some(close) = text[open..].find("](")
    {
        text = text[open + 1..open + close].to_string();
    }
    unbacktick(text.trim()).trim().to_string()
}

fn table_entry(
    headers: &[String],
    cells: &[String],
    other_key: &str,
    produces: bool,
) -> std::result::Result<EdgeDecl, String> {
    let cell = |name: &str| -> Option<String> {
        let index = headers.iter().position(|h| h == name)?;
        cells
            .get(index)
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
    };
    let kind = cell("kind").ok_or_else(|| "has no 'kind'".to_string())?;
    let name = cell("name")
        .or_else(|| cell("endpoint"))
        .ok_or_else(|| "has no 'name'".to_string())?;
    let other = cell(other_key)
        .or_else(|| cell("repo"))
        .ok_or_else(|| format!("has no '{other_key}'"))?;
    Ok(EdgeDecl {
        other,
        kind: kind.to_ascii_lowercase(),
        name,
        produces,
        site: cell("site"),
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

pub(crate) fn site_path(site: &str) -> &str {
    let site = site.trim();
    let Some((path, suffix)) = site.rsplit_once(':') else {
        return site;
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let is_line = digits(suffix)
        || suffix
            .split_once('-')
            .is_some_and(|(a, b)| digits(a) && digits(b));
    if is_line && !path.is_empty() {
        path
    } else {
        site
    }
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
        let (edges, warnings) = edges_of("09-interfaces.md", &parsed.fields, &parsed.body);
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
        let (edges, warnings) = edges_of("09-interfaces.md", &parsed.fields, &parsed.body);
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

    const TABLE_PAGE: &str = "---\ngenerated_date: 2026-09-04\n---\n\n## Produces\n\n| Kind | Name | To | Site |\n|---|---|---|---|\n| SQS | file-ingest | [record-store](../record-store/09-interfaces.md) | `src/publish/sqs.py:57` |\n\n## Consumes\n\n| Kind | Name | From | Site |\n|---|---|---|---|\n| http | GET /customers/{id} | identity-api | `src/clients/customers.py:12` |\n";

    #[test]
    fn s6_tables_declare_edges_when_frontmatter_does_not() {
        let parsed = parse(TABLE_PAGE);
        let (edges, warnings) = edges_of("09-interfaces.md", &parsed.fields, &parsed.body);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0].kind, "sqs");
        assert_eq!(edges[0].other, "record-store");
        assert_eq!(edges[0].name, "file-ingest");
        assert!(edges[0].produces);
        assert_eq!(edges[1].other, "identity-api");
        assert_eq!(edges[1].name, "GET /customers/{id}");
        assert!(!edges[1].produces);
    }

    #[test]
    fn s6_frontmatter_wins_when_a_page_has_both() {
        let both = format!(
            "---\nproduces:\n  - kind: kafka\n    name: only-this\n    to: warehouse\n---\n{}",
            parse(TABLE_PAGE).body
        );
        let parsed = parse(&both);
        let (edges, _) = edges_of("09-interfaces.md", &parsed.fields, &parsed.body);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].name, "only-this");
    }

    #[test]
    fn s6_tables_outside_the_interfaces_page_are_prose() {
        let parsed = parse(TABLE_PAGE);
        let (edges, warnings) = edges_of("01-architecture.md", &parsed.fields, &parsed.body);
        assert!(edges.is_empty(), "{edges:?}");
        assert!(warnings.is_empty());
    }

    #[test]
    fn s6_a_renamed_column_is_a_warning_not_a_silent_zero() {
        let page = "---\ngenerated_date: 2026-09-04\n---\n\n## Produces\n\n| Kind | Name | Target |\n|---|---|---|\n| sqs | file-ingest | record-store |\n";
        let parsed = parse(page);
        let (edges, warnings) = edges_of("09-interfaces.md", &parsed.fields, &parsed.body);
        assert!(edges.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("has no 'to'"), "{warnings:?}");
    }

    #[test]
    fn s6_an_escaped_pipe_survives_the_split() {
        let page = "---\ngenerated_date: 2026-09-04\n---\n\n## Produces\n\n| Kind | Name | To |\n|---|---|---|\n| http | GET /a\\|b | other |\n";
        let parsed = parse(page);
        let (edges, _) = edges_of("09-interfaces.md", &parsed.fields, &parsed.body);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].name, "GET /a|b");
    }

    #[test]
    fn s6_a_table_in_a_fenced_block_is_not_read() {
        let page = "---\ngenerated_date: 2026-09-04\n---\n\n## Produces\n\n```\n| Kind | Name | To |\n|---|---|---|\n| sqs | example | somewhere |\n```\n";
        let parsed = parse(page);
        let (edges, warnings) = edges_of("09-interfaces.md", &parsed.fields, &parsed.body);
        assert!(edges.is_empty(), "{edges:?}");
        assert!(warnings.is_empty(), "{warnings:?}");
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

    #[test]
    fn s15_known_as_reads_a_list_of_strings() {
        let parsed = parse("---\nknown_as: [records-svc, records.internal, records-svc]\n---\n");
        let (names, warnings) = known_as_of(&parsed.fields);
        assert_eq!(names, ["records-svc", "records.internal"]);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn s15_known_as_not_a_list_is_a_warning() {
        let parsed = parse("---\nknown_as: records-svc\n---\n");
        let (names, warnings) = known_as_of(&parsed.fields);
        assert!(names.is_empty(), "{names:?}");
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].contains("known_as is not a list"),
            "{warnings:?}"
        );
    }

    #[test]
    fn s15_known_as_non_string_items_are_warned_and_skipped() {
        let parsed = parse("---\nknown_as: [a, 3, \"\"]\n---\n");
        let (names, warnings) = known_as_of(&parsed.fields);
        assert_eq!(names, ["a"]);
        assert_eq!(
            warnings,
            ["known_as[1] is not a string", "known_as[2] is empty"]
        );
    }

    #[test]
    fn s15_unknown_target_is_case_insensitive() {
        assert!(is_unknown_target("Unknown"));
        assert!(is_unknown_target(" UNKNOWN "));
        assert!(!is_unknown_target("unknown-service"));
    }

    #[test]
    fn s18_site_path_strips_a_line_or_range() {
        assert_eq!(site_path("src/a.rs:12"), "src/a.rs");
        assert_eq!(site_path("src/a.rs:10-20"), "src/a.rs");
        assert_eq!(site_path("src/a.rs"), "src/a.rs");
        assert_eq!(site_path("src/a.rs:L12"), "src/a.rs:L12");
        assert_eq!(site_path("a:b.rs:3"), "a:b.rs");
        assert_eq!(site_path(":12"), ":12");
        assert_eq!(site_path(" src/a.rs:1 "), "src/a.rs");
    }

    #[test]
    fn s18_frontmatter_site_is_carried() {
        let parsed = parse(PAGE);
        let (edges, _) = edges_of("09-interfaces.md", &parsed.fields, &parsed.body);
        assert_eq!(edges[0].site.as_deref(), Some("src/publish/sqs.py:57"));
        assert_eq!(edges[1].site, None);
    }

    #[test]
    fn s18_table_site_cell_loses_its_backticks() {
        let parsed = parse(TABLE_PAGE);
        let (edges, _) = edges_of("09-interfaces.md", &parsed.fields, &parsed.body);
        assert_eq!(edges[0].site.as_deref(), Some("src/publish/sqs.py:57"));
        assert_eq!(
            edges[1].site.as_deref(),
            Some("src/clients/customers.py:12")
        );
    }

    #[test]
    fn s18_a_linked_site_cell_loses_its_backticks_too() {
        let page = "---\ngenerated_date: 2026-09-04\n---\n\n## Produces\n\n| Kind | Name | To | Site |\n|---|---|---|---|\n| sqs | other-thing | record-store | [`src/publish.py:3`](https://example.test/blob/abc/src/publish.py#L3) |\n";
        let parsed = parse(page);
        let (edges, warnings) = edges_of("09-interfaces.md", &parsed.fields, &parsed.body);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].site.as_deref(), Some("src/publish.py:3"));
        assert_eq!(
            site_path(edges[0].site.as_deref().unwrap()),
            "src/publish.py"
        );
    }

    #[test]
    fn s18_only_one_pair_of_backticks_comes_off_a_cell() {
        assert_eq!(unbacktick("`src/a.rs:1`"), "src/a.rs:1");
        assert_eq!(unbacktick("`a` and `b`"), "a` and `b");
        assert_eq!(unbacktick("plain"), "plain");
        assert_eq!(unbacktick("`"), "`");
        let page = "---\ngenerated_date: 2026-09-04\n---\n\n## Produces\n\n| Kind | Name | To | Site |\n|---|---|---|---|\n| sqs | `a` and `b` | record-store | `src/publish.py:3` |\n";
        let parsed = parse(page);
        let (edges, warnings) = edges_of("09-interfaces.md", &parsed.fields, &parsed.body);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(edges[0].name, "a` and `b");
        assert_eq!(edges[0].site.as_deref(), Some("src/publish.py:3"));
    }

    #[test]
    fn s18_a_table_without_a_site_column_yields_none() {
        let page = "---\ngenerated_date: 2026-09-04\n---\n\n## Produces\n\n| Kind | Name | To |\n|---|---|---|\n| sqs | file-ingest | record-store |\n";
        let parsed = parse(page);
        let (edges, warnings) = edges_of("09-interfaces.md", &parsed.fields, &parsed.body);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].site, None);
    }
}
