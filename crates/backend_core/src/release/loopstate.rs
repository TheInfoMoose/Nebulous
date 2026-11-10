use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Result, TrackerError};
use crate::release::config::Guardrails;

static ANSI_ESCAPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\x1b(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])").expect("valid ANSI escape regex")
});

static TS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b\d{4}-\d{2}-\d{2}[t ]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:z|[+-]\d{2}:?\d{2})?\b")
        .expect("valid timestamp regex")
});

static HEX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b0x[0-9a-f]+\b").expect("valid hex address regex"));

static TMP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/tmp/[a-z0-9_.-]+").expect("valid /tmp path regex"));

static PID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bpid[=:\s]+\d+\b").expect("valid pid regex"));

static WS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s+").expect("valid whitespace regex"));

static ERR_KEYWORD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(error|fatal|panic|fail|failed|exception)\b")
        .expect("valid error keyword regex")
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stage {
    Build,
    LiveBoot,
    InstallE2e,
}

impl Stage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::LiveBoot => "live-boot",
            Self::InstallE2e => "install-e2e",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttemptStatus {
    Pass,
    Fail,
}

impl AttemptStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Attempt {
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub stage: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub error_summary: String,
    #[serde(default)]
    pub error_fingerprint: String,
    #[serde(default)]
    pub diff_fingerprint: String,
    #[serde(default)]
    pub modified_files: Vec<String>,
    #[serde(default)]
    pub log_file: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ItemSection {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub phase_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub loop_locked: bool,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default)]
    pub negative_constraints: Vec<String>,
    #[serde(default, skip_serializing)]
    pub attempts: Vec<Attempt>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LoopItem {
    pub item: ItemSection,
    #[serde(default)]
    pub attempts: Vec<Attempt>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<toml::Table>,
}

impl Eq for LoopItem {}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffFingerprint {
    pub fingerprint: String,
    pub modified_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptInput {
    pub stage: Stage,
    pub status: AttemptStatus,
    pub error_summary: String,
    pub log_file: Option<PathBuf>,
    pub diff: DiffFingerprint,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordOutcome {
    pub item_id: String,
    pub retry_count: u32,
    pub loop_locked: bool,
    pub error_fingerprint: String,
    pub diff_fingerprint: String,
    pub negative_constraints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardrailState {
    pub item_id: String,
    pub status: String,
    pub loop_locked: bool,
    pub retry_count: u32,
    pub negative_constraints: Vec<String>,
}

fn sha256_hex_16(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(16);
    for b in &digest[..8] {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
    }
    out
}

pub fn strip_ansi(text: &str) -> String {
    ANSI_ESCAPE_RE.replace_all(text, "").into_owned()
}

pub fn normalize_error_fingerprint(summary: &str, log_text: &str) -> String {
    let clean_summary = strip_ansi(summary);
    let clean_summary = clean_summary.trim();
    let clean_log = strip_ansi(log_text);
    let clean_log = clean_log.trim();

    let source = if !clean_summary.is_empty() {
        clean_summary
    } else {
        clean_log
    };
    let lower = source.to_lowercase();
    let step1 = TS_RE.replace_all(&lower, "<ts>");
    let step2 = HEX_RE.replace_all(&step1, "<hex>");
    let step3 = TMP_RE.replace_all(&step2, "/tmp/<tmp>");
    let step4 = PID_RE.replace_all(&step3, "pid=<pid>");
    let collapsed = WS_RE.replace_all(&step4, " ");
    let trimmed = collapsed.trim();

    sha256_hex_16(trimmed.as_bytes())
}

fn log_tail(path: &Path) -> String {
    let Ok(bytes) = fs::read(path) else {
        return String::new();
    };
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let start = lines.len().saturating_sub(40);
    let tail: String = lines[start..].concat();
    strip_ansi(&tail)
}

fn summary_from(error_summary: &str, tail: &str) -> String {
    let clean = strip_ansi(error_summary).trim().to_string();
    if !clean.is_empty() {
        return clean;
    }

    let clean_tail = strip_ansi(tail);
    let non_empty: Vec<&str> = clean_tail
        .lines()
        .map(str::trim)
        .filter(|ln| !ln.is_empty())
        .collect();
    if non_empty.is_empty() {
        return String::new();
    }

    if let Some(last_err) = non_empty
        .iter()
        .rev()
        .find(|ln| ERR_KEYWORD_RE.is_match(ln))
    {
        return (*last_err).to_string();
    }

    non_empty[non_empty.len() - 1].to_string()
}

pub fn git_diff_fingerprint(repo_root: &Path) -> DiffFingerprint {
    let Ok(diff_out) = Command::new("git")
        .args(["diff", "HEAD"])
        .current_dir(repo_root)
        .output()
    else {
        return DiffFingerprint {
            fingerprint: "clean".to_string(),
            modified_files: Vec::new(),
        };
    };

    let Ok(names_out) = Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .current_dir(repo_root)
        .output()
    else {
        return DiffFingerprint {
            fingerprint: "clean".to_string(),
            modified_files: Vec::new(),
        };
    };

    let diff_stdout = String::from_utf8_lossy(&diff_out.stdout);
    let diff_text = diff_stdout.trim();
    let names_stdout = String::from_utf8_lossy(&names_out.stdout);
    let modified_files: Vec<String> = names_stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    let fingerprint = if diff_text.is_empty() {
        "clean".to_string()
    } else {
        sha256_hex_16(diff_text.as_bytes())
    };

    DiffFingerprint {
        fingerprint,
        modified_files,
    }
}

fn file_stem_id(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string()
}

pub fn read_loop_item(path: &Path) -> Result<Option<LoopItem>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(TrackerError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    let mut parsed: LoopItem =
        toml::from_str(&text).map_err(|source| TrackerError::TomlDeserialize {
            path: Some(path.to_path_buf()),
            source,
        })?;

    if parsed.attempts.is_empty() && !parsed.item.attempts.is_empty() {
        parsed.attempts = std::mem::take(&mut parsed.item.attempts);
    } else {
        parsed.item.attempts.clear();
    }

    Ok(Some(parsed))
}

pub fn render_loop_item(li: &LoopItem) -> Result<String> {
    let border = "=".repeat(78);
    let body = toml::to_string(li)?;
    Ok(format!(
        "# {border}\n\
         # Nebulous Tiered Work Item: {}\n\
         # {border}\n\n\
         {body}",
        li.item.id
    ))
}

/// Atomically writes `li` to `path`, creating parent directories as needed.
///
/// The caller must hold `TrackerStorage::with_write_lock`.
pub fn write_loop_item(path: &Path, li: &LoopItem) -> Result<()> {
    let text = render_loop_item(li)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|source| TrackerError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }
    crate::storage::atomic_write_str(path, &text)
}

/// Records a build or test attempt in `path`, evaluates loop-breaker rules, and
/// atomically persists the updated loop-state item.
///
/// The caller must hold `TrackerStorage::with_write_lock`.
pub fn record_attempt(
    path: &Path,
    input: AttemptInput,
    guardrails: &Guardrails,
) -> Result<RecordOutcome> {
    let mut li = match read_loop_item(path)? {
        Some(existing) => existing,
        None => LoopItem {
            item: ItemSection {
                id: file_stem_id(path),
                status: "in_progress".to_string(),
                ..ItemSection::default()
            },
            ..LoopItem::default()
        },
    };

    let tail = input.log_file.as_deref().map(log_tail).unwrap_or_default();
    let summary = summary_from(&input.error_summary, &tail);
    let err_fp = match input.status {
        AttemptStatus::Fail => normalize_error_fingerprint(&summary, &tail),
        AttemptStatus::Pass => "none".to_string(),
    };

    let diff_fp = input.diff.fingerprint;
    let mod_files = input.diff.modified_files;

    let same_diff_prior = !diff_fp.is_empty()
        && diff_fp != "none"
        && li
            .attempts
            .iter()
            .any(|a| a.status == "fail" && a.diff_fingerprint == diff_fp);

    let log_file_str = input
        .log_file
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    li.attempts.push(Attempt {
        timestamp: input.timestamp,
        stage: input.stage.as_str().to_string(),
        status: input.status.as_str().to_string(),
        error_summary: summary.clone(),
        error_fingerprint: err_fp.clone(),
        diff_fingerprint: diff_fp.clone(),
        modified_files: mod_files.clone(),
        log_file: log_file_str,
    });

    match input.status {
        AttemptStatus::Pass => {
            li.item.retry_count = 0;
            if !li.item.loop_locked {
                li.item.status = "in_progress".to_string();
            }
        }
        AttemptStatus::Fail => {
            li.item.retry_count += 1;
            let same_err = li
                .attempts
                .iter()
                .filter(|a| a.status == "fail" && a.error_fingerprint == err_fp)
                .count() as u32;

            if same_err >= guardrails.loop_repeat_warn_threshold || same_diff_prior {
                let mod_str = if mod_files.is_empty() {
                    "working-tree".to_string()
                } else {
                    mod_files.join(", ")
                };
                let neg_prompt = format!(
                    "LOOP DETECTED [fingerprint={err_fp}]: Approach in [{mod_str}] failed \
                     repeatedly with '{summary}'. DO NOT re-apply the same diff or retry \
                     without changing root-cause architecture. Dropping packages or downgrading \
                     versions is strictly forbidden."
                );
                if !li.item.negative_constraints.contains(&neg_prompt) {
                    li.item.negative_constraints.push(neg_prompt);
                }
            }

            if li.item.retry_count >= guardrails.loop_max_retries_hard_stop {
                li.item.status = "blocked_loop_hard_stop".to_string();
                li.item.loop_locked = true;
            }
        }
    }

    write_loop_item(path, &li)?;

    Ok(RecordOutcome {
        item_id: li.item.id,
        retry_count: li.item.retry_count,
        loop_locked: li.item.loop_locked,
        error_fingerprint: err_fp,
        diff_fingerprint: diff_fp,
        negative_constraints: li.item.negative_constraints,
    })
}

pub fn guardrail_state(path: &Path) -> Result<Option<GuardrailState>> {
    Ok(read_loop_item(path)?.map(|li| GuardrailState {
        item_id: li.item.id,
        status: li.item.status,
        loop_locked: li.item.loop_locked,
        retry_count: li.item.retry_count,
        negative_constraints: li.item.negative_constraints,
    }))
}

/// Clears `loop_locked` and `retry_count`, resets `status` to `"in_progress"`,
/// and preserves `attempts`, `diagnostics`, and `negative_constraints`.
///
/// The caller must hold `TrackerStorage::with_write_lock` and must have passed
/// the user-only gate (§5.1).
pub fn unlock(path: &Path) -> Result<LoopItem> {
    let mut li = read_loop_item(path)?.ok_or_else(|| TrackerError::NotFound {
        kind: "loop-state item",
        id: file_stem_id(path),
    })?;
    li.item.loop_locked = false;
    li.item.retry_count = 0;
    li.item.status = "in_progress".to_string();
    write_loop_item(path, &li)?;
    Ok(li)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    const SAMPLE_ITEM_TOML: &str = r#"# ==============================================================================
# Nebulous Tiered Work Item: item-001-docker-qemu-ci
# ==============================================================================

[item]
id = "item-001-docker-qemu-ci"
phase_id = "phase-01-ci-foundation"
title = "Dockerized Build & QEMU Two-Tier Verification Workflow"
status = "in_progress"
loop_locked = false
retry_count = 0
negative_constraints = []

[[attempts]]
timestamp = "2026-11-07T21:59:47.832469+00:00"
stage = "install-e2e"
status = "fail"
error_summary = "Timed out after 1200s waiting for guest readiness"
error_fingerprint = "3b9c45640c421370"
diff_fingerprint = "c1c28460e7989d91"
modified_files = ["STEPPED_IMPLEMENTATION_PLAN.md", "scripts/test-docker-qemu.py"]
log_file = "/tmp/neb-test/20261107T213943Z-install-e2e-serial.log"

[[attempts]]
timestamp = "2026-11-08T13:00:19.059832+00:00"
stage = "live-boot"
status = "pass"
error_summary = "BdsDxe: starting Boot0001 \"UEFI QEMU DVD-ROM QM00005 \""
error_fingerprint = "none"
diff_fingerprint = "b94da4b636aa67c8"
modified_files = [".nebulous/items/item-001-docker-qemu-ci.toml", "scripts/test-docker-qemu.py"]
log_file = "/tmp/neb-test/20261108T130001Z-live-boot-serial.log"

[[attempts]]
timestamp = "2026-11-08T13:02:11.548153+00:00"
stage = "install-e2e"
status = "fail"
error_summary = "E: Unable to locate package example-pkg"
error_fingerprint = "f461c40155246c03"
diff_fingerprint = "98cca799abe75db0"
modified_files = ["scripts/test-docker-qemu.py"]
log_file = "/tmp/neb-test/20261108T130037Z-install-e2e-serial.log"

[[attempts]]
timestamp = "2026-11-08T14:24:40.410306+00:00"
stage = "install-e2e"
status = "pass"
error_summary = "BdsDxe: starting Boot0003"
error_fingerprint = "none"
diff_fingerprint = "clean"
modified_files = []
log_file = "/tmp/neb-test/20261108T142002Z-install-e2e-reboot-serial.log"
"#;

    #[test]
    fn parse_real_shape_round_trips_and_preserves_diagnostics_across_record_attempt() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("item-001-docker-qemu-ci.toml");
        let with_diag = format!(
            "{SAMPLE_ITEM_TOML}\n\
             [[diagnostics]]\n\
             timestamp = \"2026-11-08T15:00:00Z\"\n\
             hypothesis = \"missing mirror entry\"\n"
        );
        fs::write(&path, &with_diag).expect("write fixture");

        let parsed = read_loop_item(&path)
            .expect("read_loop_item")
            .expect("present");
        assert_eq!(parsed.item.id, "item-001-docker-qemu-ci");
        assert_eq!(parsed.attempts.len(), 4);
        assert!(parsed.item.negative_constraints.is_empty());
        assert_eq!(parsed.diagnostics.len(), 1);

        let rendered = render_loop_item(&parsed).expect("render_loop_item");
        assert!(rendered.starts_with(
            "# ==============================================================================\n\
             # Nebulous Tiered Work Item: item-001-docker-qemu-ci\n\
             # ==============================================================================\n\n"
        ));
        let round_path = tmp.path().join("round.toml");
        fs::write(&round_path, &rendered).expect("write round");
        let reparsed = read_loop_item(&round_path)
            .expect("read round")
            .expect("present");
        assert_eq!(reparsed, parsed);

        // Recording a new attempt preserves [[diagnostics]].
        record_attempt(
            &path,
            AttemptInput {
                stage: Stage::Build,
                status: AttemptStatus::Pass,
                error_summary: String::new(),
                log_file: None,
                diff: DiffFingerprint {
                    fingerprint: "clean".to_string(),
                    modified_files: Vec::new(),
                },
                timestamp: "2026-11-09T00:00:00Z".to_string(),
            },
            &Guardrails::default(),
        )
        .expect("record_attempt");

        let after = read_loop_item(&path)
            .expect("read after attempt")
            .expect("present");
        assert_eq!(after.attempts.len(), 5);
        assert_eq!(after.diagnostics.len(), 1);
        assert_eq!(
            after.diagnostics[0]
                .get("hypothesis")
                .and_then(|v| v.as_str()),
            Some("missing mirror entry")
        );
    }

    #[test]
    fn legacy_attempts_under_item_section_move_to_top_level() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("legacy-item.toml");
        let legacy = r#"
[item]
id = "legacy-item"
phase_id = "phase-01"
title = "Legacy nested attempts"
status = "in_progress"
loop_locked = false
retry_count = 1
negative_constraints = []

[[item.attempts]]
timestamp = "2026-11-01T00:00:00Z"
stage = "build"
status = "fail"
error_summary = "build failed"
error_fingerprint = "0123456789abcdef"
diff_fingerprint = "clean"
modified_files = []
log_file = ""
"#;
        fs::write(&path, legacy).expect("write legacy");

        let loaded = read_loop_item(&path)
            .expect("read legacy")
            .expect("present");
        assert_eq!(loaded.attempts.len(), 1);
        assert_eq!(loaded.attempts[0].error_summary, "build failed");
        assert!(loaded.item.attempts.is_empty());

        // Also verify rendering and re-reading an item with zero attempts works cleanly.
        let empty_item = LoopItem {
            item: ItemSection {
                id: "empty-item".to_string(),
                status: "in_progress".to_string(),
                ..ItemSection::default()
            },
            attempts: Vec::new(),
            diagnostics: Vec::new(),
        };
        let empty_path = tmp.path().join("empty-item.toml");
        write_loop_item(&empty_path, &empty_item).expect("write empty item");
        let empty_loaded = read_loop_item(&empty_path)
            .expect("read empty item")
            .expect("present");
        assert_eq!(empty_loaded, empty_item);
    }

    #[test]
    fn strip_ansi_removes_sgr_sequences() {
        let raw = "\x1b[31mERROR:\x1b[0m package build failed";
        assert_eq!(strip_ansi(raw), "ERROR: package build failed");
    }

    #[test]
    fn normalize_error_fingerprint_strips_volatiles_and_matches_fixed_vectors() {
        let fp_a = normalize_error_fingerprint(
            "E: build failed at 2026-11-08T13:02:11Z addr 0x7ffd1234 in /tmp/build.ab12 pid=4242",
            "",
        );
        let fp_b = normalize_error_fingerprint(
            "E: build failed at 2026-11-09 04:15:00.123+00:00 addr 0xdeadbeef in /tmp/work_99 pid 99",
            "",
        );
        assert_eq!(fp_a, fp_b);
        assert_eq!(fp_a.len(), 16);
        assert!(fp_a.chars().all(|c| c.is_ascii_hexdigit()));

        let fp_c = normalize_error_fingerprint("E: completely different error", "");
        assert_ne!(fp_a, fp_c);

        // Fixed vectors computed with nebulous-ctl.py's normalize_error_fingerprint on 2026-11-10.
        assert_eq!(
            normalize_error_fingerprint(
                "E: Unable to locate package example-pkg at 2026-11-08T13:02:11Z pid=4242 0x7ffd1234 /tmp/build.ab12",
                ""
            ),
            "fe0a0a3a04fb63c1"
        );
        assert_eq!(
            normalize_error_fingerprint("", "Timed out after 1200s waiting for guest readiness"),
            "3b9c45640c421370"
        );
    }

    #[test]
    fn summary_from_prefers_last_keyword_line_then_last_non_blank_line() {
        let tail_with_err = "starting step\nE: build failed\nok\n";
        assert_eq!(summary_from("", tail_with_err), "E: build failed");

        let tail_no_keywords = "step 1 done\nstep 2 done\nfinal status line\n\n";
        assert_eq!(summary_from("", tail_no_keywords), "final status line");
    }

    #[test]
    fn three_fail_attempts_warn_on_second_and_lock_on_third() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("item-loop.toml");
        let guardrails = Guardrails::default(); // warn = 2, hard_stop = 3

        let make_fail = |ts: &str, diff_fp: &str| AttemptInput {
            stage: Stage::Build,
            status: AttemptStatus::Fail,
            error_summary: "E: Unable to locate package example-pkg".to_string(),
            log_file: None,
            diff: DiffFingerprint {
                fingerprint: diff_fp.to_string(),
                modified_files: vec!["scripts/stages/03-core.sh".to_string()],
            },
            timestamp: ts.to_string(),
        };

        // Attempt #1: no constraint, unlocked, retry 1.
        let out1 = record_attempt(
            &path,
            make_fail("2026-11-10T01:00:00Z", "diff1"),
            &guardrails,
        )
        .expect("attempt 1");
        assert_eq!(out1.item_id, "item-loop");
        assert_eq!(out1.retry_count, 1);
        assert!(!out1.loop_locked);
        assert!(out1.negative_constraints.is_empty());

        // Attempt #2: one constraint, unlocked, retry 2.
        let out2 = record_attempt(
            &path,
            make_fail("2026-11-10T01:05:00Z", "diff2"),
            &guardrails,
        )
        .expect("attempt 2");
        assert_eq!(out2.retry_count, 2);
        assert!(!out2.loop_locked);
        assert_eq!(out2.negative_constraints.len(), 1);
        assert!(out2.negative_constraints[0].contains("LOOP DETECTED"));
        assert!(out2.negative_constraints[0].contains("scripts/stages/03-core.sh"));

        // Attempt #3: locked, status blocked_loop_hard_stop, retry 3, still one constraint (deduped).
        let out3 = record_attempt(
            &path,
            make_fail("2026-11-10T01:10:00Z", "diff3"),
            &guardrails,
        )
        .expect("attempt 3");
        assert_eq!(out3.retry_count, 3);
        assert!(out3.loop_locked);
        assert_eq!(out3.negative_constraints.len(), 1);

        let gs = guardrail_state(&path)
            .expect("guardrail_state")
            .expect("present");
        assert_eq!(gs.status, "blocked_loop_hard_stop");
        assert!(gs.loop_locked);
        assert_eq!(gs.retry_count, 3);
    }

    #[test]
    fn same_diff_fingerprint_across_different_errors_triggers_constraint() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("item-diff.toml");
        let guardrails = Guardrails::default();

        record_attempt(
            &path,
            AttemptInput {
                stage: Stage::Build,
                status: AttemptStatus::Fail,
                error_summary: "first distinct failure".to_string(),
                log_file: None,
                diff: DiffFingerprint {
                    fingerprint: "same_diff_123456".to_string(),
                    modified_files: Vec::new(),
                },
                timestamp: "2026-11-10T02:00:00Z".to_string(),
            },
            &guardrails,
        )
        .expect("first attempt");

        let out2 = record_attempt(
            &path,
            AttemptInput {
                stage: Stage::Build,
                status: AttemptStatus::Fail,
                error_summary: "second distinct failure".to_string(),
                log_file: None,
                diff: DiffFingerprint {
                    fingerprint: "same_diff_123456".to_string(),
                    modified_files: Vec::new(),
                },
                timestamp: "2026-11-10T02:05:00Z".to_string(),
            },
            &guardrails,
        )
        .expect("second attempt");

        assert_eq!(out2.negative_constraints.len(), 1);
        assert!(out2.negative_constraints[0].contains("[working-tree]"));
        assert!(out2.negative_constraints[0].contains("second distinct failure"));
    }

    #[test]
    fn pass_resets_retry_count_and_locked_item_stays_locked_on_pass() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("item-pass.toml");
        let guardrails = Guardrails {
            loop_max_retries_hard_stop: 2,
            ..Guardrails::default()
        };

        // 1 Fail -> retry 1.
        let f1 = record_attempt(
            &path,
            AttemptInput {
                stage: Stage::LiveBoot,
                status: AttemptStatus::Fail,
                error_summary: "boot timeout".to_string(),
                log_file: None,
                diff: DiffFingerprint {
                    fingerprint: "d1".to_string(),
                    modified_files: Vec::new(),
                },
                timestamp: "2026-11-10T03:00:00Z".to_string(),
            },
            &guardrails,
        )
        .expect("fail 1");
        assert_eq!(f1.retry_count, 1);

        // Pass -> retry 0, status in_progress.
        let p1 = record_attempt(
            &path,
            AttemptInput {
                stage: Stage::LiveBoot,
                status: AttemptStatus::Pass,
                error_summary: String::new(),
                log_file: None,
                diff: DiffFingerprint {
                    fingerprint: "d2".to_string(),
                    modified_files: Vec::new(),
                },
                timestamp: "2026-11-10T03:05:00Z".to_string(),
            },
            &guardrails,
        )
        .expect("pass 1");
        assert_eq!(p1.retry_count, 0);
        assert!(!p1.loop_locked);
        assert_eq!(
            guardrail_state(&path).unwrap().unwrap().status,
            "in_progress"
        );

        // 2 Fails -> locked (hard_stop = 2).
        for i in 0..2 {
            record_attempt(
                &path,
                AttemptInput {
                    stage: Stage::InstallE2e,
                    status: AttemptStatus::Fail,
                    error_summary: format!("install error {i}"),
                    log_file: None,
                    diff: DiffFingerprint {
                        fingerprint: format!("df{i}"),
                        modified_files: Vec::new(),
                    },
                    timestamp: "2026-11-10T03:10:00Z".to_string(),
                },
                &guardrails,
            )
            .expect("fail to lock");
        }

        // Pass on a locked item -> still locked, status stays blocked_loop_hard_stop.
        let p_locked = record_attempt(
            &path,
            AttemptInput {
                stage: Stage::InstallE2e,
                status: AttemptStatus::Pass,
                error_summary: String::new(),
                log_file: None,
                diff: DiffFingerprint {
                    fingerprint: "clean".to_string(),
                    modified_files: Vec::new(),
                },
                timestamp: "2026-11-10T03:15:00Z".to_string(),
            },
            &guardrails,
        )
        .expect("pass while locked");
        assert!(p_locked.loop_locked);
        assert_eq!(p_locked.retry_count, 0);
        let gs = guardrail_state(&path).unwrap().unwrap();
        assert!(gs.loop_locked);
        assert_eq!(gs.status, "blocked_loop_hard_stop");
    }

    #[test]
    fn unlock_clears_lock_and_retry_keeps_attempts_and_missing_returns_not_found() {
        let tmp = tempdir().expect("tempdir");
        let missing = tmp.path().join("does-not-exist.toml");
        let err = unlock(&missing).expect_err("missing item must fail");
        assert!(matches!(
            err,
            TrackerError::NotFound {
                kind: "loop-state item",
                ref id
            } if id == "does-not-exist"
        ));

        let path = tmp.path().join("locked-item.toml");
        let guardrails = Guardrails {
            loop_max_retries_hard_stop: 1,
            ..Guardrails::default()
        };
        record_attempt(
            &path,
            AttemptInput {
                stage: Stage::Build,
                status: AttemptStatus::Fail,
                error_summary: "failed".to_string(),
                log_file: None,
                diff: DiffFingerprint {
                    fingerprint: "clean".to_string(),
                    modified_files: Vec::new(),
                },
                timestamp: "2026-11-10T04:00:00Z".to_string(),
            },
            &guardrails,
        )
        .expect("lock item");

        let unlocked = unlock(&path).expect("unlock");
        assert!(!unlocked.item.loop_locked);
        assert_eq!(unlocked.item.retry_count, 0);
        assert_eq!(unlocked.item.status, "in_progress");
        assert_eq!(unlocked.attempts.len(), 1);
    }

    #[test]
    fn git_diff_fingerprint_reports_clean_and_modified_tree() {
        if Command::new("git").arg("--version").output().is_err() {
            eprintln!("skipping git_diff_fingerprint test: git not on PATH");
            return;
        }

        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();

        let status = Command::new("git")
            .arg("init")
            .current_dir(root)
            .status()
            .expect("git init");
        assert!(status.success());

        let tracked = root.join("tracked.txt");
        fs::write(&tracked, "initial\n").expect("write tracked.txt");

        assert!(Command::new("git")
            .args(["add", "tracked.txt"])
            .current_dir(root)
            .status()
            .expect("git add")
            .success());

        assert!(Command::new("git")
            .args([
                "-c",
                "user.name=testuser",
                "-c",
                "user.email=testuser@example.com",
                "commit",
                "-m",
                "initial",
            ])
            .current_dir(root)
            .status()
            .expect("git commit")
            .success());

        let clean = git_diff_fingerprint(root);
        assert_eq!(clean.fingerprint, "clean");
        assert!(clean.modified_files.is_empty());

        fs::write(&tracked, "modified\n").expect("modify tracked.txt");
        let dirty = git_diff_fingerprint(root);
        assert_ne!(dirty.fingerprint, "clean");
        assert_eq!(dirty.fingerprint.len(), 16);
        assert!(dirty.fingerprint.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(dirty.modified_files, vec!["tracked.txt".to_string()]);
    }
}
