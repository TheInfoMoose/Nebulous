use std::collections::HashSet;

use crate::error::{Result, TrackerError};

/// A single step parsed from a plan file's `## Step Execution Roadmap` tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanStep {
    pub step_id: String,
    pub title: String,
    pub command: Option<String>,
    pub target_files: Vec<String>,
    pub order: Option<u32>,
    pub completed: bool,
}

const ROADMAP_HEADING: &str = "## Step Execution Roadmap";
const UP_NEXT_HEADING: &str = "### Up Next";
const COMPLETED_HEADING: &str = "### Completed";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    UpNext,
    Completed,
    Other,
}

/// Parse the `### Up Next` and `### Completed` markdown tables under `## Step Execution Roadmap`.
pub fn parse_roadmap(markdown: &str) -> Result<Vec<PlanStep>> {
    let mut in_roadmap = false;
    let mut found_roadmap = false;
    let mut found_up_next = false;
    let mut found_completed = false;
    let mut section = Section::Other;

    let mut up_next_lines: Vec<&str> = Vec::new();
    let mut completed_lines: Vec<&str> = Vec::new();

    for line in markdown.lines() {
        let trimmed = line.trim();
        if !in_roadmap {
            if trimmed == ROADMAP_HEADING {
                in_roadmap = true;
                found_roadmap = true;
            }
            continue;
        }

        if trimmed.starts_with("## ") && !trimmed.starts_with("###") {
            break;
        }

        if trimmed == UP_NEXT_HEADING {
            found_up_next = true;
            section = Section::UpNext;
            continue;
        }
        if trimmed == COMPLETED_HEADING {
            found_completed = true;
            section = Section::Completed;
            continue;
        }
        if trimmed.starts_with("### ") {
            section = Section::Other;
            continue;
        }

        if trimmed.starts_with('|') {
            match section {
                Section::UpNext => up_next_lines.push(trimmed),
                Section::Completed => completed_lines.push(trimmed),
                Section::Other => {}
            }
        }
    }

    if !found_roadmap {
        return Err(TrackerError::ValidationError(format!(
            "Missing required heading '{ROADMAP_HEADING}'"
        )));
    }
    if !found_up_next {
        return Err(TrackerError::ValidationError(format!(
            "Missing required heading '{UP_NEXT_HEADING}'"
        )));
    }
    if !found_completed {
        return Err(TrackerError::ValidationError(format!(
            "Missing required heading '{COMPLETED_HEADING}'"
        )));
    }

    let mut steps = Vec::new();
    let mut seen_ids = HashSet::new();

    for step in parse_up_next_table(&up_next_lines)? {
        if !seen_ids.insert(step.step_id.clone()) {
            return Err(TrackerError::ValidationError(format!(
                "Duplicate step_id '{}' in roadmap",
                step.step_id
            )));
        }
        steps.push(step);
    }

    for step in parse_completed_table(&completed_lines)? {
        if !seen_ids.insert(step.step_id.clone()) {
            return Err(TrackerError::ValidationError(format!(
                "Duplicate step_id '{}' in roadmap",
                step.step_id
            )));
        }
        steps.push(step);
    }

    Ok(steps)
}

fn parse_up_next_table(lines: &[&str]) -> Result<Vec<PlanStep>> {
    let Some((&header_line, body_lines)) = lines.split_first() else {
        return Err(TrackerError::ValidationError(
            "Missing column '#' in ### Up Next table".to_string(),
        ));
    };

    let headers: Vec<String> = split_table_row(header_line)
        .into_iter()
        .map(|c| clean_cell(&c))
        .collect();
    let header_len = headers.len();

    let order_idx = find_column(&headers, &["#"], "#")?;
    let step_idx = find_column(&headers, &["Step", "Step ID"], "Step")?;
    let command_idx = find_column(&headers, &["Command"], "Command")?;
    let scope_idx = find_column(&headers, &["Scope"], "Scope")?;
    let target_files_idx = find_column(&headers, &["Target Files"], "Target Files")?;
    let waits_on_idx = find_column(&headers, &["Waits on"], "Waits on")?;

    let mut steps = Vec::new();
    for &raw_line in body_lines {
        let cells = split_table_row(raw_line);
        if cells.len() != header_len {
            return Err(TrackerError::ValidationError(format!(
                "Row cell count ({}) does not match header ({header_len}): {raw_line}",
                cells.len()
            )));
        }
        if is_separator_row(&cells) {
            continue;
        }

        let step_id = clean_cell(&cells[step_idx]);
        if step_id.starts_with("[USER]") {
            continue;
        }

        let order_str = clean_cell(&cells[order_idx]);
        let order = order_str.parse::<u32>().map_err(|_| {
            TrackerError::ValidationError(format!(
                "Invalid Up Next '#' order value '{order_str}' in row: {raw_line}"
            ))
        })?;

        let title = clean_cell(&cells[scope_idx]);
        let command = parse_command(&cells[command_idx]);
        let target_files = parse_target_files(&cells[target_files_idx]);
        let waits_on = clean_cell(&cells[waits_on_idx]);
        let completed = waits_on.starts_with("COMPLETED");

        steps.push(PlanStep {
            step_id,
            title,
            command,
            target_files,
            order: Some(order),
            completed,
        });
    }

    Ok(steps)
}

fn parse_completed_table(lines: &[&str]) -> Result<Vec<PlanStep>> {
    let Some((&header_line, body_lines)) = lines.split_first() else {
        return Err(TrackerError::ValidationError(
            "Missing column 'Step ID' in ### Completed table".to_string(),
        ));
    };

    let headers: Vec<String> = split_table_row(header_line)
        .into_iter()
        .map(|c| clean_cell(&c))
        .collect();
    let header_len = headers.len();

    let step_idx = find_column(&headers, &["Step ID", "Step"], "Step ID")?;
    let command_idx = find_column(&headers, &["Command"], "Command")?;
    let scope_idx = find_column(&headers, &["Scope"], "Scope")?;
    let target_files_idx = find_column(&headers, &["Target Files"], "Target Files")?;

    let mut steps = Vec::new();
    for &raw_line in body_lines {
        let cells = split_table_row(raw_line);
        if cells.len() != header_len {
            return Err(TrackerError::ValidationError(format!(
                "Row cell count ({}) does not match header ({header_len}): {raw_line}",
                cells.len()
            )));
        }
        if is_separator_row(&cells) {
            continue;
        }

        let step_id = clean_cell(&cells[step_idx]);
        if step_id.starts_with("[USER]") {
            continue;
        }

        let title = clean_cell(&cells[scope_idx]);
        let command = parse_command(&cells[command_idx]);
        let target_files = parse_target_files(&cells[target_files_idx]);

        steps.push(PlanStep {
            step_id,
            title,
            command,
            target_files,
            order: None,
            completed: true,
        });
    }

    Ok(steps)
}

fn find_column(headers: &[String], accepted: &[&str], canonical: &str) -> Result<usize> {
    headers
        .iter()
        .position(|h| accepted.iter().any(|cand| h.eq_ignore_ascii_case(cand)))
        .ok_or_else(|| {
            TrackerError::ValidationError(format!("Missing required column '{canonical}'"))
        })
}

fn is_separator_row(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells.iter().all(|cell| {
            let t = cell.trim();
            !t.is_empty() && t.contains('-') && t.chars().all(|c| c == '-' || c == ':')
        })
}

fn split_table_row(raw_line: &str) -> Vec<String> {
    let trimmed = raw_line.trim();
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = if inner.ends_with('|') && !inner.ends_with(r"\|") {
        &inner[..inner.len() - 1]
    } else {
        inner
    };

    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = inner.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&'|') {
            chars.next();
            current.push('|');
        } else if ch == '|' {
            cells.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    cells.push(current);
    cells
}

fn clean_cell(raw: &str) -> String {
    raw.replace("**", "").replace('`', "").trim().to_string()
}

fn parse_command(raw: &str) -> Option<String> {
    let cleaned = clean_cell(raw);
    if cleaned.is_empty() || cleaned == "—" {
        None
    } else {
        Some(cleaned)
    }
}

fn parse_target_files(raw: &str) -> Vec<String> {
    let cleaned = clean_cell(raw);
    if cleaned.is_empty() || cleaned == "—" {
        return Vec::new();
    }

    cleaned
        .split(", ")
        .map(|entry| {
            let trimmed = entry.trim();
            trimmed
                .strip_suffix("(NEW)")
                .or_else(|| trimmed.strip_suffix("(DEL)"))
                .unwrap_or(trimmed)
                .trim()
                .to_string()
        })
        .filter(|entry| !entry.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SYNTHETIC_FIXTURE: &str = r#"# Synthetic Plan

## Step Execution Roadmap

### Up Next

| # | Step | Command | Scope | Target Files | Waits on |
| :---: | :--- | :---: | :--- | :--- | :--- |
| 1 | **STEP 4A** | `/goal` | Add parser module | `crates/core/src/parser.rs` (NEW), `crates/core/src/lib.rs` | **COMPLETED** |
| 2 | **STEP 4B** | `/boost` | Wire CLI subcommand | `crates/cli/src/args.rs` | — |
| 3 | **[USER] SEED** | — | Run manual seed | `.tracker/` | — |

### Completed

| Step ID | Status | Command | Scope | Target Files |
| :--- | :---: | :---: | :--- | :--- |
| **STEP 1–3** | **COMPLETED** (`abc1234`) | — | Initial foundation cleanup | `crates/legacy/src/old.rs` (DEL), `README.md` |

### Open Questions

| # | Step | Command | Scope | Target Files | Waits on |
| :---: | :--- | :---: | :--- | :--- | :--- |
| 99 | **STEP 99** | `/goal` | Ignored table outside subsections | `ignored.rs` | — |

## Next Major Section

### Up Next

| # | Step | Command | Scope | Target Files | Waits on |
| :---: | :--- | :---: | :--- | :--- | :--- |
| 100 | **STEP 100** | `/goal` | Ignored table outside roadmap | `ignored2.rs` | — |
"#;

    #[test]
    fn parse_roadmap_reads_up_next_and_completed() {
        let steps = parse_roadmap(SYNTHETIC_FIXTURE).expect("should parse synthetic fixture");
        assert_eq!(
            steps,
            vec![
                PlanStep {
                    step_id: "STEP 4A".to_string(),
                    title: "Add parser module".to_string(),
                    command: Some("/goal".to_string()),
                    target_files: vec![
                        "crates/core/src/parser.rs".to_string(),
                        "crates/core/src/lib.rs".to_string(),
                    ],
                    order: Some(1),
                    completed: true,
                },
                PlanStep {
                    step_id: "STEP 4B".to_string(),
                    title: "Wire CLI subcommand".to_string(),
                    command: Some("/boost".to_string()),
                    target_files: vec!["crates/cli/src/args.rs".to_string()],
                    order: Some(2),
                    completed: false,
                },
                PlanStep {
                    step_id: "STEP 1–3".to_string(),
                    title: "Initial foundation cleanup".to_string(),
                    command: None,
                    target_files: vec![
                        "crates/legacy/src/old.rs".to_string(),
                        "README.md".to_string(),
                    ],
                    order: None,
                    completed: true,
                },
            ]
        );
    }

    #[test]
    fn parse_roadmap_completed_in_waits_on_marks_completed() {
        let steps = parse_roadmap(SYNTHETIC_FIXTURE).expect("should parse synthetic fixture");
        let step_4a = steps
            .iter()
            .find(|s| s.step_id == "STEP 4A")
            .expect("should contain STEP 4A");
        assert!(step_4a.completed);

        let step_4b = steps
            .iter()
            .find(|s| s.step_id == "STEP 4B")
            .expect("should contain STEP 4B");
        assert!(!step_4b.completed);
    }

    #[test]
    fn parse_roadmap_skips_user_rows() {
        let steps = parse_roadmap(SYNTHETIC_FIXTURE).expect("should parse synthetic fixture");
        assert!(!steps.iter().any(|s| s.step_id.starts_with("[USER]")));
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn parse_roadmap_missing_heading_errors() {
        let no_roadmap = SYNTHETIC_FIXTURE.replace("## Step Execution Roadmap", "## Overview");
        let err = parse_roadmap(&no_roadmap).expect_err("missing roadmap heading should error");
        assert!(
            err.to_string().contains("## Step Execution Roadmap"),
            "error should name '## Step Execution Roadmap', got: {err}"
        );

        let no_completed = SYNTHETIC_FIXTURE.replace("### Completed", "### Done");
        let err = parse_roadmap(&no_completed).expect_err("missing Completed heading should error");
        assert!(
            err.to_string().contains("### Completed"),
            "error should name '### Completed', got: {err}"
        );
    }

    #[test]
    fn parse_roadmap_duplicate_step_id_errors() {
        let with_dup = SYNTHETIC_FIXTURE.replace("**STEP 1–3**", "**STEP 4B**");
        let err = parse_roadmap(&with_dup).expect_err("duplicate step_id should error");
        assert!(
            err.to_string().contains("STEP 4B"),
            "error should name duplicate ID 'STEP 4B', got: {err}"
        );
    }

    #[test]
    fn parse_roadmap_escaped_pipe_stays_in_one_cell() {
        let with_escaped_pipe = SYNTHETIC_FIXTURE.replace(
            "Initial foundation cleanup",
            "Forbid `curl \\| sh` in scripts",
        );
        let steps =
            parse_roadmap(&with_escaped_pipe).expect("escaped pipe in cell should parse cleanly");
        let step = steps
            .iter()
            .find(|s| s.step_id == "STEP 1–3")
            .expect("should find STEP 1–3");
        assert_eq!(step.title, "Forbid curl | sh in scripts");
        assert_eq!(step.command, None);
        assert_eq!(
            step.target_files,
            vec![
                "crates/legacy/src/old.rs".to_string(),
                "README.md".to_string(),
            ]
        );
        assert!(step.completed);
    }
}
