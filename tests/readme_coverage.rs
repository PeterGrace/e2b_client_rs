//! The README's coverage table is computed from the claims ledger
//! (`claims.json`, CLM-0184): one row per area (the `s1`..`s5` tags), with
//! the active claims' current verdicts. When the ledger changes, this test
//! fails and prints the table to paste.

use std::collections::BTreeMap;

use serde_json::Value;

/// The README rows, in order: area tag and label.
const AREAS: [(&str, &str); 6] = [
    ("s1", "S1 Lifecycle"),
    ("s2", "S2 Commands and PTY"),
    ("s3", "S3 Filesystem"),
    ("s4", "S4 Wire protocol"),
    ("s5", "S5 Publishability"),
    ("root", "System (root)"),
];
const VERDICTS: [&str; 4] = ["supported", "not_enough_info", "refuted", "unverified"];
const BEGIN: &str = "<!-- coverage:begin -->";
const END: &str = "<!-- coverage:end -->";

/// The table for the ledger, as Markdown.
fn expected_table(ledger: &Value) -> String {
    // `BTreeMap` keeps the counts keyed by (area, verdict).
    let mut counts: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    let claims = ledger["claims"].as_array().expect("claims array");
    for claim in claims {
        if claim["status"]["state"] != "active" {
            continue;
        }
        let verdict = claim["verdicts"]
            .as_array()
            .and_then(|verdicts| verdicts.last())
            .and_then(|last| last["verdict"].as_str())
            .unwrap_or("unverified");
        // An unknown verdict would drop out of every column, so fail instead.
        assert!(
            VERDICTS.contains(&verdict),
            "{}: unknown verdict {verdict}",
            claim["id"]
        );
        let tags = claim["tags"].as_array().expect("tags array");
        let area = AREAS
            .iter()
            .map(|(tag, _)| *tag)
            .find(|tag| tags.iter().any(|t| t == tag))
            .unwrap_or_else(|| panic!("{} has no area tag", claim["id"]));
        *counts.entry((area, verdict)).or_default() += 1;
    }
    let mut table = String::from(
        "| Area | Claims | Supported | Not enough info | Refuted | Unverified |\n\
         |---|---|---|---|---|---|\n",
    );
    let mut totals = [0usize; 4];
    for (tag, label) in AREAS {
        let row: Vec<usize> = VERDICTS
            .iter()
            .map(|verdict| counts.get(&(tag, *verdict)).copied().unwrap_or(0))
            .collect();
        for (total, count) in totals.iter_mut().zip(&row) {
            *total += count;
        }
        table.push_str(&format!(
            "| {label} | {} | {} | {} | {} | {} |\n",
            row.iter().sum::<usize>(),
            row[0],
            row[1],
            row[2],
            row[3]
        ));
    }
    table.push_str(&format!(
        "| **All** | **{}** | **{}** | **{}** | **{}** | **{}** |\n",
        totals.iter().sum::<usize>(),
        totals[0],
        totals[1],
        totals[2],
        totals[3]
    ));
    table
}

#[test]
fn readme_coverage_table_matches_the_ledger() {
    // Arrange
    let ledger: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/claims.json"))
            .expect("read claims.json"),
    )
    .expect("parse claims.json");
    let readme = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))
        .expect("read README.md");

    // Act
    let expected = expected_table(&ledger);
    let start = readme
        .find(BEGIN)
        .expect("README has the coverage:begin marker")
        + BEGIN.len();
    let end = readme
        .find(END)
        .expect("README has the coverage:end marker");
    let actual = readme[start..end].trim();

    // Assert
    assert_eq!(
        actual,
        expected.trim(),
        "README coverage table is stale; replace the lines between the markers with:\n\n{expected}"
    );
}
