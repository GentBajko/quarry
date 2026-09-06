//! The contract diff: a producer's payload tables against what each consumer records.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::errors::Result;
use crate::frontmatter::{self, INTERFACES_PAGE, Section, Table};
use crate::index::{Edge, Index};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PayloadField {
    pub(crate) name: String,
    pub(crate) field_type: Option<String>,
    pub(crate) required: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProducedContract {
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) fields: Option<Vec<PayloadField>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConsumerSection {
    Missing,
    NoFields,
    Fields(Vec<PayloadField>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConsumerPage {
    pub(crate) generated_date: Option<String>,
    pub(crate) section: ConsumerSection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    pub(crate) field: String,
    pub(crate) reason: String,
    pub(crate) breaking: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ContractBreak {
    pub(crate) consumer: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) field: String,
    pub(crate) reason: String,
    // Set whenever monorepo targets are configured; the target, not the repo,
    // is the producer name consumers write.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ConsumerOut {
    pub(crate) repo: String,
    pub(crate) fields: Vec<String>,
    pub(crate) generated_date: Option<String>,
    pub(crate) breaks: Vec<String>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ContractOut {
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) consumers: Vec<ConsumerOut>,
    // Set whenever monorepo targets are configured; the target, not the repo,
    // is the producer name consumers write.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) target: Option<String>,
}

// Every vector is serialised even when empty: a CI consumer indexes
// `result.breaks` without a null check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct CheckOut {
    pub(crate) repo: String,
    pub(crate) contracts: Vec<ContractOut>,
    pub(crate) breaks: Vec<ContractBreak>,
    pub(crate) warnings: Vec<String>,
    pub(crate) notes: Vec<String>,
}

/// The level-3-and-deeper sections that sit under `## <parent>`.
pub(crate) fn contract_sections(body: &str, parent: &str) -> Vec<Section> {
    let mut out = Vec::new();
    let mut under = false;
    for section in frontmatter::sections_of(body) {
        match section.level {
            1 => under = false,
            2 => under = section.normalized == parent,
            _ if under => out.push(section),
            _ => {}
        }
    }
    out
}

/// The section for one contract: exact heading first, then `<name> (` as a prefix.
pub(crate) fn find_contract<'a>(sections: &'a [Section], name: &str) -> Option<&'a Section> {
    let want = frontmatter::normalize_heading(name.trim_matches('`'));
    if want.is_empty() {
        return None;
    }
    let folded =
        |section: &Section| frontmatter::normalize_heading(section.heading.trim_matches('`'));
    if let Some(hit) = sections.iter().find(|s| folded(s) == want) {
        return Some(hit);
    }
    let prefix = format!("{want} (");
    sections.iter().find(|s| folded(s).starts_with(&prefix))
}

/// The payload rows of a `| Field | Type | Required |` table; `None` when the
/// table has no `Field` column at all.
pub(crate) fn fields_of(table: &Table) -> Option<Vec<PayloadField>> {
    let field_at = table.headers.iter().position(|h| h.as_str() == "field")?;
    let type_at = table.headers.iter().position(|h| h.as_str() == "type");
    let required_at = table.headers.iter().position(|h| h.as_str() == "required");
    let mut fields = Vec::new();
    for row in &table.rows {
        let Some(name) = row.get(field_at).map(|c| c.trim()) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        fields.push(PayloadField {
            name: name.to_string(),
            field_type: type_at
                .and_then(|i| row.get(i))
                .map(|c| c.trim().to_string())
                .filter(|c| !c.is_empty()),
            required: required_at
                .and_then(|i| row.get(i))
                .map(|c| parse_required(c)),
        });
    }
    Some(fields)
}

fn parse_required(cell: &str) -> bool {
    matches!(cell.trim().to_lowercase().as_str(), "yes" | "true")
}

fn fold_type(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn yes_no(flag: bool) -> &'static str {
    if flag { "yes" } else { "no" }
}

/// The contracts this repo's chapter says it produces, with their payload tables.
pub(crate) fn produced_contracts(text: &str) -> (Vec<ProducedContract>, Vec<String>) {
    let parsed = frontmatter::parse(text);
    let mut warnings: Vec<String> = parsed.warning.iter().cloned().collect();
    let (edges, edge_warnings) =
        frontmatter::edges_of(INTERFACES_PAGE, &parsed.fields, &parsed.body);
    warnings.extend(edge_warnings);
    let sections = contract_sections(&parsed.body, "produces");
    let mut contracts: Vec<ProducedContract> = Vec::new();
    for edge in edges {
        if !edge.produces {
            continue;
        }
        if contracts
            .iter()
            .any(|c| c.kind == edge.kind && c.name == edge.name)
        {
            continue;
        }
        let fields = find_contract(&sections, &edge.name)
            .and_then(|section| frontmatter::table_of(&section.body))
            .and_then(|table| fields_of(&table));
        contracts.push(ProducedContract {
            kind: edge.kind,
            name: edge.name,
            fields,
        });
    }
    (contracts, warnings)
}

/// What one consumer's chapter records for a contract.
pub(crate) fn consumer_page(text: &str, name: &str) -> ConsumerPage {
    let parsed = frontmatter::parse(text);
    let generated_date = parsed
        .fields
        .get("generated_date")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let sections = contract_sections(&parsed.body, "consumes");
    let section = match find_contract(&sections, name) {
        None => ConsumerSection::Missing,
        Some(section) => match frontmatter::table_of(&section.body).and_then(|t| fields_of(&t)) {
            None => ConsumerSection::NoFields,
            Some(fields) => ConsumerSection::Fields(fields),
        },
    };
    ConsumerPage {
        generated_date,
        section,
    }
}

/// The P1 verdicts, in the consumer's table order.
pub(crate) fn compare(producer: &[PayloadField], consumer: &[PayloadField]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for read in consumer {
        let Some(made) = producer.iter().find(|p| p.name == read.name) else {
            findings.push(Finding {
                field: read.name.clone(),
                reason: "no longer produced".to_string(),
                breaking: true,
            });
            continue;
        };
        if let (Some(theirs), Some(ours)) = (&made.field_type, &read.field_type)
            && fold_type(theirs) != fold_type(ours)
        {
            findings.push(Finding {
                field: read.name.clone(),
                reason: format!("type changed: {theirs} (consumer reads {ours})"),
                breaking: true,
            });
        }
        if let (Some(theirs), Some(ours)) = (made.required, read.required)
            && theirs != ours
        {
            findings.push(Finding {
                field: read.name.clone(),
                reason: format!(
                    "required flipped: {} (consumer reads {})",
                    yes_no(theirs),
                    yes_no(ours)
                ),
                breaking: false,
            });
        }
    }
    findings
}

pub(crate) fn run(
    index: &Index,
    repo: &str,
    producer_text: Option<&str>,
    clone: &Path,
) -> Result<CheckOut> {
    let mut out = CheckOut {
        repo: repo.to_string(),
        contracts: Vec::new(),
        breaks: Vec::new(),
        warnings: Vec::new(),
        notes: Vec::new(),
    };
    if !index.has_repo(repo)? {
        out.notes
            .push(format!("{repo} is not in the docs repo; run quarry add"));
        return Ok(out);
    }
    let produced: Option<Vec<ProducedContract>> = producer_text.map(|text| {
        let (contracts, warnings) = produced_contracts(text);
        out.warnings.extend(
            warnings
                .into_iter()
                .map(|w| format!("{INTERFACES_PAGE}: {w}")),
        );
        contracts
    });

    // Iteration follows the index's consumer edges: a contract deleted outright
    // is the break the gate exists for.
    let mut groups: BTreeMap<(String, String), Vec<Edge>> = BTreeMap::new();
    for edge in index.edges(repo, true)? {
        if edge.missing || edge.declared_by == "observed" || edge.to_repo == repo {
            continue;
        }
        groups
            .entry((edge.kind.clone(), edge.name.clone()))
            .or_default()
            .push(edge);
    }
    if groups.is_empty() {
        out.notes.push(format!(
            "no consumers of {repo} in the quarry; nothing to compare"
        ));
        return Ok(out);
    }

    for ((kind, name), edges) in groups {
        let row = produced
            .as_ref()
            .and_then(|c| c.iter().find(|p| p.kind == kind && p.name == name));
        let producer_fields: Option<Vec<PayloadField>> = match row {
            Some(row) => match &row.fields {
                Some(fields) => Some(fields.clone()),
                None => {
                    out.warnings.push(format!(
                        "no payload table for {kind} {name}; nothing to compare"
                    ));
                    None
                }
            },
            None => {
                if edges
                    .iter()
                    .any(|e| e.declared_by == "producer" || e.declared_by == "both")
                {
                    out.warnings.push(format!(
                        "{kind} {name} missing from Produces; treated as removed"
                    ));
                    Some(Vec::new())
                } else {
                    for edge in &edges {
                        out.notes.push(format!(
                            "{} declares {kind} {name} from {repo}; not in {repo}'s Produces table",
                            edge.to_repo
                        ));
                    }
                    continue;
                }
            }
        };

        let mut contract = ContractOut {
            kind: kind.clone(),
            name: name.clone(),
            consumers: Vec::new(),
            target: None,
        };
        for edge in edges {
            let consumer = edge.to_repo;
            let page = match fs::read_to_string(clone.join(&consumer).join(INTERFACES_PAGE)) {
                Ok(text) => consumer_page(&text, &name),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => ConsumerPage {
                    generated_date: None,
                    section: ConsumerSection::Missing,
                },
                Err(e) => return Err(e.into()),
            };
            let mut consumer_out = ConsumerOut {
                repo: consumer.clone(),
                fields: Vec::new(),
                generated_date: page.generated_date,
                breaks: Vec::new(),
                warnings: Vec::new(),
            };
            match page.section {
                ConsumerSection::Missing => out.notes.push(format!(
                    "{consumer} declares {kind} {name} but records no contract section"
                )),
                ConsumerSection::NoFields => out
                    .notes
                    .push(format!("{consumer} lists no fields for {name}")),
                ConsumerSection::Fields(read) => {
                    consumer_out.fields = read.iter().map(|f| f.name.clone()).collect();
                    if let Some(made) = &producer_fields {
                        for finding in compare(made, &read) {
                            let line = format!("{} {}", finding.field, finding.reason);
                            if finding.breaking {
                                consumer_out.breaks.push(line);
                                out.breaks.push(ContractBreak {
                                    consumer: consumer.clone(),
                                    kind: kind.clone(),
                                    name: name.clone(),
                                    field: finding.field,
                                    reason: finding.reason,
                                    target: None,
                                });
                            } else {
                                consumer_out.warnings.push(line);
                            }
                        }
                    }
                }
            }
            contract.consumers.push(consumer_out);
        }
        out.contracts.push(contract);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    const NESTED: &str =
        "## Produces\n\n### A\n\nx\n\n## Consumes\n\n### B\n\ny\n\n# Other\n\n### C\n\nz\n";

    fn headings(sections: &[Section]) -> Vec<&str> {
        sections.iter().map(|s| s.heading.as_str()).collect()
    }

    fn table(text: &str) -> Table {
        frontmatter::table_of(text).expect("table")
    }

    fn field(name: &str, ty: Option<&str>, required: Option<bool>) -> PayloadField {
        PayloadField {
            name: name.to_string(),
            field_type: ty.map(str::to_string),
            required,
        }
    }

    #[test]
    fn s14_contract_sections_follow_their_parent_heading() {
        assert_eq!(headings(&contract_sections(NESTED, "produces")), ["A"]);
        assert_eq!(headings(&contract_sections(NESTED, "consumes")), ["B"]);
        assert!(!headings(&contract_sections(NESTED, "produces")).contains(&"C"));
        assert!(!headings(&contract_sections(NESTED, "consumes")).contains(&"C"));
    }

    #[test]
    fn s14_exact_heading_beats_a_version_suffix() {
        let body = "## Produces\n\n### GET /records (v2)\n\nx\n\n### GET /records\n\ny\n";
        let sections = contract_sections(body, "produces");
        assert_eq!(
            find_contract(&sections, "GET /records").map(|s| s.heading.as_str()),
            Some("GET /records")
        );
        let suffixed = contract_sections("## Produces\n\n### GET /records (v2)\n\nx\n", "produces");
        assert_eq!(
            find_contract(&suffixed, "GET /records").map(|s| s.heading.as_str()),
            Some("GET /records (v2)")
        );
        assert!(find_contract(&suffixed, "GET /recordsx").is_none());
        let backticked = contract_sections("## Produces\n\n### `GET /records`\n\nx\n", "produces");
        assert_eq!(
            find_contract(&backticked, "GET /records").map(|s| s.heading.as_str()),
            Some("`GET /records`")
        );
        assert!(find_contract(&suffixed, "").is_none());
    }

    #[test]
    fn s14_required_accepts_yes_no_true_false_and_defaults_to_no() {
        let with = table(
            "| Field | Required |\n|---|---|\n| a | yes |\n| b | TRUE |\n| c | no |\n| d | maybe |\n| e |  |\n",
        );
        let required: Vec<Option<bool>> = fields_of(&with)
            .expect("fields")
            .into_iter()
            .map(|f| f.required)
            .collect();
        assert_eq!(
            required,
            [
                Some(true),
                Some(true),
                Some(false),
                Some(false),
                Some(false)
            ]
        );
        let without = table("| Field | Type |\n|---|---|\n| a | string |\n");
        assert_eq!(
            fields_of(&without).expect("fields")[0],
            field("a", Some("string"), None)
        );
    }

    #[test]
    fn s14_a_table_without_a_field_column_lists_no_fields() {
        let named = table("| Name | Type |\n|---|---|\n| a | string |\n");
        assert!(fields_of(&named).is_none());
        let bare = table("| Field |\n|---|\n| a |\n");
        assert_eq!(fields_of(&bare).expect("fields"), [field("a", None, None)]);
    }

    #[test]
    fn s14_types_compare_after_folding() {
        let same = compare(
            &[field("id", Some("String  (UUID)"), None)],
            &[field("id", Some("string (uuid)"), None)],
        );
        assert!(same.is_empty(), "{same:?}");
        let changed = compare(
            &[field("id", Some("string"), None)],
            &[field("id", Some("enum"), None)],
        );
        assert_eq!(changed.len(), 1);
        assert_eq!(
            changed[0].reason,
            "type changed: string (consumer reads enum)"
        );
        assert!(changed[0].breaking);
    }

    #[test]
    fn s14_compare_yields_the_three_verdicts() {
        let producer = [
            field("id", Some("string"), Some(true)),
            field("created_at", Some("string"), Some(true)),
            field("size", Some("int"), Some(false)),
        ];
        let consumer = [
            field("id", Some("string"), Some(false)),
            field("created_at", Some("string"), Some(true)),
            field("content_type", Some("enum"), Some(true)),
        ];
        let findings = compare(&producer, &consumer);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].field, "id");
        assert_eq!(
            findings[0].reason,
            "required flipped: yes (consumer reads no)"
        );
        assert!(!findings[0].breaking);
        assert_eq!(findings[1].field, "content_type");
        assert_eq!(findings[1].reason, "no longer produced");
        assert!(findings[1].breaking);
    }

    #[test]
    fn s14_produced_contracts_dedupe_rows_that_share_a_name() {
        let page = "---\ngenerated_date: 2026-09-03\n---\n\n## Produces\n\n| Kind | Name | To |\n|---|---|---|\n| http | GET /records | report-builder |\n| http | GET /records | chart-service |\n| sqs | audit-log | report-builder |\n\n### GET /records\n\n| Field | Type | Required |\n|---|---|---|\n| id | string | yes |\n";
        let (contracts, warnings) = produced_contracts(page);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(contracts.len(), 2);
        assert_eq!(contracts[0].name, "GET /records");
        assert_eq!(
            contracts[0].fields.as_deref(),
            Some([field("id", Some("string"), Some(true))].as_slice())
        );
        assert_eq!(contracts[1].name, "audit-log");
        assert_eq!(contracts[1].fields, None);
    }

    #[test]
    fn s14_consumer_page_reports_missing_no_fields_and_fields() {
        let head = "---\ngenerated_date: 2026-09-01\n---\n\n## Consumes\n\n| Kind | Name | From |\n|---|---|---|\n| http | GET /records | record-store |\n";
        assert_eq!(
            consumer_page(head, "GET /records").section,
            ConsumerSection::Missing
        );
        let prose = format!("{head}\n### GET /records (v2)\n\nStill being written.\n");
        assert_eq!(
            consumer_page(&prose, "GET /records").section,
            ConsumerSection::NoFields
        );
        let listed = format!(
            "{head}\n### GET /records (v2)\n\n| Field | Type | Required |\n|---|---|---|\n| id | string | yes |\n"
        );
        let page = consumer_page(&listed, "GET /records");
        assert_eq!(page.generated_date.as_deref(), Some("2026-09-01"));
        assert_eq!(
            page.section,
            ConsumerSection::Fields(vec![field("id", Some("string"), Some(true))])
        );
    }
}
