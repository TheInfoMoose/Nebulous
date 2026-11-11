use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use backend_core::{
    release::{
        approvals::read_approvals,
        component::golden_from_inventory,
        config::ReleasePaths,
        golden::{read_golden, render_golden, write_golden},
        holds::{all_last_good, hold_file, ledger_holds, pins_from_release, render_hold_file},
        iso::extract_iso,
        ledger::read_ledger,
        loopstate::{git_diff_fingerprint, guardrail_state, record_attempt, AttemptInput},
        manifest::read_manifest,
        records::{latest_record, list_records, read_record},
        repo_invariants::verify_repo_invariants,
        selftest::run_self_test,
        status::{read_status, render_status_text, status_json},
        variance::evaluate_variance,
        ExitCode, HARD_STOP_LOOP_LOCK, HARD_STOP_VARIANCE_LOCK, STATUS_PASS,
    },
    TrackerError,
};
use serde_json::{json, Value};

use crate::args::{HoldFormat, ResultArg, StageArg};

use super::{resolve_item_path, run_or_error, ReleaseCtx, ReleaseRun};

/// Handler for `neb release self-test` (`command = "self-test"`).
pub fn handle_self_test() -> ReleaseRun {
    let report = run_self_test();
    let total = report.cases.len();
    let failed = report.failed().len();
    let passed = total - failed;

    let mut text = String::new();
    for case in &report.cases {
        if case.passed {
            let _ = writeln!(text, "ok   {}", case.name);
        } else {
            let _ = writeln!(text, "FAIL {}: {}", case.name, case.detail);
        }
    }
    let _ = writeln!(text, "{passed}/{total} passed");

    let (exit, status) = if report.all_passed() {
        (ExitCode::Ok, STATUS_PASS)
    } else {
        (ExitCode::Error, "SELF_TEST_FAILED")
    };

    let body = json!({
        "cases": report.cases,
        "passed": passed,
        "failed": failed,
    });

    ReleaseRun::new(exit, status, body, text)
}

/// Handler for `neb release status` (`command = "status"`).
pub fn handle_status(ctx: &ReleaseCtx) -> ReleaseRun {
    run_or_error(|| {
        let status = read_status(&ctx.paths)?;
        let body = status_json(&status)?;
        let text = render_status_text(&status);
        Ok(ReleaseRun::new(ExitCode::Ok, STATUS_PASS, body, text))
    })
}

/// Handler for `neb release holds` (`command = "holds"`).
pub fn handle_holds(
    ctx: &ReleaseCtx,
    all_last_good_flag: bool,
    format: Option<HoldFormat>,
) -> ReleaseRun {
    run_or_error(|| {
        let golden = read_golden(&ctx.paths.golden())?.ok_or_else(|| {
            TrackerError::ValidationError("golden.toml missing; run neb release import".to_string())
        })?;
        let ledger = read_ledger(&ctx.paths.failures())?;
        let records = list_records(&ctx.paths.releases_dir())?;

        let holds = if all_last_good_flag {
            all_last_good(&golden, &records)
        } else {
            ledger_holds(&ledger, &golden, &records)
        };

        let count = holds.len();
        let body = json!({
            "holds": &holds,
            "count": count,
        });

        let text = match format {
            Some(HoldFormat::HoldFile) => {
                let hf = hold_file(holds, &ctx.timestamp(), "neb release holds");
                render_hold_file(&hf)?
            }
            None => {
                if holds.is_empty() {
                    "no holds\n".to_string()
                } else {
                    let mut out = String::new();
                    for h in &holds {
                        let _ = writeln!(
                            out,
                            "{}  held {}  blocked {}  {}",
                            h.component,
                            h.held_version,
                            h.blocked_version,
                            h.reason.as_str()
                        );
                    }
                    out
                }
            }
        };

        Ok(ReleaseRun::new(ExitCode::Ok, STATUS_PASS, body, text))
    })
}

/// Handler for `neb release pins-from-release` (`command = "pins-from-release"`).
pub fn handle_pins_from_release(
    ctx: &ReleaseCtx,
    release: &str,
    components: &[String],
) -> ReleaseRun {
    run_or_error(|| {
        let (record_path, record) = if release == "latest" {
            let releases_dir = ctx.paths.releases_dir();
            latest_record(&releases_dir)?.ok_or_else(|| {
                TrackerError::ValidationError(format!(
                    "no release records in {}",
                    releases_dir.display()
                ))
            })?
        } else {
            let path = PathBuf::from(release);
            let rec = read_record(&path)?;
            (path, rec)
        };

        let holds = pins_from_release(&record, (!components.is_empty()).then_some(components))?;
        let record_name = record_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(release)
            .to_string();

        let body = json!({
            "holds": &holds,
            "release_record": record_name,
        });

        let hf = hold_file(holds, &ctx.timestamp(), "neb release pins-from-release");
        let text = render_hold_file(&hf)?;

        Ok(ReleaseRun::new(ExitCode::Ok, STATUS_PASS, body, text))
    })
}

fn is_golden_output_path(output: &Path, paths: &ReleasePaths) -> bool {
    let golden_path = paths.golden();
    if output == golden_path {
        return true;
    }
    let Some(out_name) = output.file_name() else {
        return false;
    };
    let out_parent = match output.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    if let (Ok(canon_out_parent), Ok(canon_release_root)) =
        (fs::canonicalize(out_parent), fs::canonicalize(paths.root()))
    {
        canon_out_parent.join(out_name) == canon_release_root.join("golden.toml")
    } else {
        false
    }
}

/// Handler for `neb release extract-golden` (`command = "extract-golden"`).
pub fn handle_extract_golden(ctx: &ReleaseCtx, iso: &Path, output: Option<&Path>) -> ReleaseRun {
    run_or_error(|| {
        if output.is_none() && !ctx.json {
            return Err(TrackerError::ValidationError(
                "--output is required without --json".to_string(),
            ));
        }

        if let Some(out) = output {
            if is_golden_output_path(out, &ctx.paths) {
                return Err(TrackerError::ValidationError(format!(
                    "extract-golden refuses to write to {}; use neb release bless-golden or neb release promote",
                    ctx.paths.golden().display()
                )));
            }
        }

        let ex = extract_iso(iso, &ctx.project_root())?;
        if !ex.identity.is_production() {
            return Err(TrackerError::ValidationError(format!(
                "{} is a test ISO (extra_live_layers: {}); extract-golden requires a production ISO",
                iso.display(),
                ex.identity.extra_live_layers.join(", ")
            )));
        }

        let g = golden_from_inventory(&ex.inventory, &ctx.config.components);
        let packages_count = g.packages.len();
        let components_count = g.components.len();
        let custom_binaries_count = g.custom_binaries.len();

        let (body, text) = match output {
            Some(out) => {
                write_golden(out, &g)?;
                let out_str = out.display().to_string();
                let body = json!({
                    "packages_count": packages_count,
                    "components_count": components_count,
                    "custom_binaries_count": custom_binaries_count,
                    "output": out_str,
                });
                let text = format!(
                    "extracted golden ({components_count} components, {packages_count} packages) -> {}\n",
                    out.display()
                );
                (body, text)
            }
            None => {
                let golden_toml = render_golden(&g)?;
                let body = json!({
                    "packages_count": packages_count,
                    "components_count": components_count,
                    "custom_binaries_count": custom_binaries_count,
                    "output": Value::Null,
                    "golden_toml": golden_toml,
                });
                let text = format!(
                    "extracted golden ({components_count} components, {packages_count} packages)\n"
                );
                (body, text)
            }
        };

        Ok(ReleaseRun::new(ExitCode::Ok, "EXTRACTED", body, text))
    })
}

/// Handler for `neb release verify-variance` (`command = "verify-variance"`).
pub fn handle_verify_variance(
    ctx: &ReleaseCtx,
    iso: Option<&Path>,
    repo_only: bool,
    manifest: Option<&Path>,
    golden: Option<&Path>,
    approvals: Option<&Path>,
) -> ReleaseRun {
    run_or_error(|| {
        let manifest_path = manifest
            .map(Path::to_path_buf)
            .unwrap_or_else(|| ctx.paths.manifest());
        let golden_path = golden
            .map(Path::to_path_buf)
            .unwrap_or_else(|| ctx.paths.golden());
        let approvals_path = approvals
            .map(Path::to_path_buf)
            .unwrap_or_else(|| ctx.paths.approvals());

        let manifest_doc = read_manifest(&manifest_path)?.ok_or_else(|| {
            TrackerError::ValidationError(
                "manifest.toml missing; run neb release import".to_string(),
            )
        })?;
        let approvals_doc = read_approvals(&approvals_path)?;
        let project_root = ctx.project_root();

        let mut violations = verify_repo_invariants(&manifest_doc, &approvals_doc, &project_root)?;
        let effective_repo_only = repo_only || iso.is_none();

        let (
            checked_packages_count,
            checked_binaries_count,
            checked_components_count,
            upgrades,
            abi_renames,
            held,
        ) = if let Some(iso_path) = iso.filter(|_| !repo_only) {
            let ex = extract_iso(iso_path, &project_root)?;
            let live = golden_from_inventory(&ex.inventory, &ctx.config.components);
            let golden_doc = read_golden(&golden_path)?;
            let report =
                evaluate_variance(&live, &manifest_doc, golden_doc.as_ref(), &approvals_doc)?;
            violations.extend(report.violations);

            let held_items: Vec<Value> = match &golden_doc {
                Some(g) => {
                    let ledger = read_ledger(&ctx.paths.failures())?;
                    let records = list_records(&ctx.paths.releases_dir())?;
                    ledger_holds(&ledger, g, &records)
                        .into_iter()
                        .filter(|h| {
                            live.components
                                .get(&h.component)
                                .is_some_and(|c| c.version == h.held_version)
                        })
                        .map(|h| json!({ "component": h.component, "version": h.held_version }))
                        .collect()
                }
                None => Vec::new(),
            };

            (
                report.checked_packages_count,
                report.checked_binaries_count,
                report.checked_components_count,
                report.upgrades,
                report.abi_renames,
                held_items,
            )
        } else {
            (0, 0, 0, Vec::new(), Vec::new(), Vec::new())
        };

        let (exit, status) = if violations.is_empty() {
            (ExitCode::Ok, STATUS_PASS)
        } else {
            (ExitCode::VarianceLock, HARD_STOP_VARIANCE_LOCK)
        };

        let mut text = String::new();
        for v in &violations {
            let _ = writeln!(text, "{}: {}: {}", v.kind, v.item, v.detail);
        }
        if violations.is_empty() {
            let _ = writeln!(text, "variance: PASS");
        } else {
            let _ = writeln!(
                text,
                "variance: HARD_STOP_VARIANCE_LOCK ({} violations)",
                violations.len()
            );
        }

        let body = json!({
            "repo_only": effective_repo_only,
            "checked_packages_count": checked_packages_count,
            "checked_binaries_count": checked_binaries_count,
            "checked_components_count": checked_components_count,
            "upgraded_count": upgrades.len(),
            "upgrades": upgrades,
            "abi_renames": abi_renames,
            "held": held,
            "violations": violations,
        });

        Ok(ReleaseRun::new(exit, status, body, text))
    })
}

/// Handler for `neb release record-attempt` (`command = "record-attempt"`).
pub fn handle_record_attempt(
    ctx: &ReleaseCtx,
    item: &str,
    stage: StageArg,
    status: ResultArg,
    error_summary: &str,
    log_file: Option<&Path>,
) -> ReleaseRun {
    run_or_error(|| {
        let outcome = ctx.storage.with_write_lock(|_| {
            let path = resolve_item_path(&ctx.paths, item)?;
            let input = AttemptInput {
                stage: stage.into(),
                status: status.into(),
                error_summary: error_summary.to_string(),
                log_file: log_file.map(Path::to_path_buf),
                diff: git_diff_fingerprint(&ctx.project_root()),
                timestamp: ctx.timestamp(),
            };
            record_attempt(&path, input, &ctx.config.guardrails)
        })?;

        let (exit, run_status) = if outcome.loop_locked {
            (ExitCode::LoopLock, HARD_STOP_LOOP_LOCK)
        } else {
            (ExitCode::Ok, "ATTEMPT_RECORDED")
        };

        let mut text = format!(
            "item {}: retry {}, locked {}\n",
            outcome.item_id, outcome.retry_count, outcome.loop_locked
        );
        for nc in &outcome.negative_constraints {
            let _ = writeln!(text, "{nc}");
        }

        let body = serde_json::to_value(&outcome).map_err(|e| {
            TrackerError::ValidationError(format!("record-attempt serialization failed: {e}"))
        })?;

        Ok(ReleaseRun::new(exit, run_status, body, text))
    })
}

/// Handler for `neb release check-guardrail` (`command = "check-guardrail"`).
pub fn handle_check_guardrail(ctx: &ReleaseCtx, item: &str) -> ReleaseRun {
    run_or_error(|| {
        let path = resolve_item_path(&ctx.paths, item)?;
        let gs = guardrail_state(&path)?.ok_or_else(|| {
            TrackerError::ValidationError(format!("loop-state item not found: {}", path.display()))
        })?;

        let manifest_doc = read_manifest(&ctx.paths.manifest())?.ok_or_else(|| {
            TrackerError::ValidationError(
                "manifest.toml missing; run neb release import".to_string(),
            )
        })?;
        let approvals_doc = read_approvals(&ctx.paths.approvals())?;
        let variance_violations =
            verify_repo_invariants(&manifest_doc, &approvals_doc, &ctx.project_root())?;

        let variance_status = if variance_violations.is_empty() {
            STATUS_PASS
        } else {
            HARD_STOP_VARIANCE_LOCK
        };

        let (exit, status) = if gs.loop_locked {
            (ExitCode::LoopLock, HARD_STOP_LOOP_LOCK)
        } else if !variance_violations.is_empty() {
            (ExitCode::VarianceLock, HARD_STOP_VARIANCE_LOCK)
        } else {
            (ExitCode::Ok, STATUS_PASS)
        };

        let text = format!(
            "GUARDRAIL_STATE: item={} status={} locked={} retry_count={} negative_constraints={} variance={}\n",
            gs.item_id,
            gs.status,
            gs.loop_locked,
            gs.retry_count,
            gs.negative_constraints.len(),
            variance_status
        );

        let body = json!({
            "item_id": gs.item_id,
            "item_status": gs.status,
            "loop_locked": gs.loop_locked,
            "retry_count": gs.retry_count,
            "negative_constraints": gs.negative_constraints,
            "variance_status": variance_status,
            "variance_violations": variance_violations,
        });

        Ok(ReleaseRun::new(exit, status, body, text))
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use backend_core::{
        release::{
            config::ReleaseConfig,
            golden::{Golden, GoldenComponent, Origin, GOLDEN_SCHEMA_VERSION},
            holds::{Hold, HoldReason},
            inventory::{BootInfo, CalamaresModules, IsoMeta},
            ledger::{record_failure, Failure, FailureSource, Phase, COMPONENT_CLASS},
            records::{
                write_record, RecordComponent, ReleaseRecord, RELEASE_RECORD_SCHEMA_VERSION,
            },
            selftest::SELF_TEST_CASES,
        },
        TrackerStorage,
    };
    use chrono::{TimeZone, Utc};
    use tempfile::tempdir;

    use super::*;
    use crate::commands::release::render;

    const SAMPLE_MANIFEST_TOML: &str = r#"
schema_version = 1

[os]
required = false

[packages.core]
dracut = { required = true, min_version = "0" }
btrfs-progs = { required = true, min_version = "0" }

[packages.cosmic]
cosmic-comp = { required = true, min_version = "0" }

[custom_binaries."/usr/local/bin/testtool"]
required = true
min_version = "0.1.0"
required_flags = ["--pre-packages"]

[calamares_modules]
required = true
required_configs = ["fstab.conf"]
custom_modules = ["limine-bootloader"]
"#;

    fn make_ctx(root: &Path, json: bool) -> ReleaseCtx {
        let storage = TrackerStorage::init_standard(root, "test-proj").expect("init_standard");
        let paths = ReleasePaths::new(&storage);
        ReleaseCtx {
            storage,
            paths,
            config: ReleaseConfig::default(),
            json,
            now: Utc.with_ymd_and_hms(2026, 11, 11, 12, 0, 0).unwrap(),
        }
    }

    fn sample_golden() -> Golden {
        let mut components = BTreeMap::new();
        components.insert(
            "example-pkg".to_string(),
            GoldenComponent {
                version: "1.0-1".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([("example-pkg".to_string(), "1.0-1".to_string())]),
            },
        );
        components.insert(
            "other-pkg".to_string(),
            GoldenComponent {
                version: "2.0-1".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([("other-pkg".to_string(), "2.0-1".to_string())]),
            },
        );
        Golden {
            schema_version: GOLDEN_SCHEMA_VERSION,
            iso: IsoMeta {
                path: "/tmp/neb-test/golden.iso".to_string(),
                size_bytes: 1024,
                vmlinuz_sha256: "a".repeat(64),
                initrd_sha256: "b".repeat(64),
                extracted_at: "2026-11-10T12:00:00Z".to_string(),
            },
            os_release: BTreeMap::from([("VERSION_ID".to_string(), "2026.11".to_string())]),
            boot: BootInfo { files: Vec::new() },
            calamares_modules: CalamaresModules {
                configs: Vec::new(),
                custom_modules: Vec::new(),
                presence: BTreeMap::new(),
            },
            custom_binaries: BTreeMap::new(),
            packages: BTreeMap::from([
                ("example-pkg".to_string(), "1.0-1".to_string()),
                ("other-pkg".to_string(), "2.0-1".to_string()),
            ]),
            components,
        }
    }

    fn write_clean_repo(root: &Path) {
        let ct_dir = root.join("custom-tools");
        fs::create_dir_all(&ct_dir).expect("create custom-tools dir");
        fs::write(
            ct_dir.join("manifest.yaml"),
            "tools:\n  - name: \"novos-custom-tools\"\n    enabled: true\n",
        )
        .expect("write custom-tools/manifest.yaml");

        let cal_dir = root.join("calamares").join("modules");
        fs::create_dir_all(cal_dir.join("limine-bootloader")).expect("create custom module dir");
        fs::write(cal_dir.join("fstab.conf"), "mountPoints: []\n").expect("write fstab.conf");
        fs::write(
            cal_dir.join("netinstall.yaml"),
            "- name: \"Base\"\n  packages:\n    - example-pkg\n",
        )
        .expect("write netinstall.yaml");

        let crate_dir = root.join("crates").join("testtool");
        fs::create_dir_all(&crate_dir).expect("create crate dir");
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/testtool\"]\n",
        )
        .expect("write workspace Cargo.toml");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"testtool\"\nversion = \"0.2.0\"\n",
        )
        .expect("write member Cargo.toml");

        let stages_dir = root.join("scripts").join("stages");
        fs::create_dir_all(&stages_dir).expect("create stages dir");
        fs::write(
            stages_dir.join("10-packages.sh"),
            "#!/bin/sh\napt-get install -y dracut btrfs-progs cosmic-comp\n/usr/local/bin/testtool --pre-packages\n",
        )
        .expect("write stage script");

        let manifest_path = root.join(".tracker").join("release").join("manifest.toml");
        fs::create_dir_all(manifest_path.parent().unwrap()).expect("create release dir");
        fs::write(&manifest_path, SAMPLE_MANIFEST_TOML).expect("write manifest.toml");
    }

    #[test]
    fn self_test_passes_all_fourteen_cases() {
        let run = handle_self_test();
        assert_eq!(run.exit, ExitCode::Ok);
        assert_eq!(run.status, "PASS");
        assert_eq!(run.body["passed"], SELF_TEST_CASES.len());
        assert_eq!(run.body["failed"], 0);
        assert_eq!(run.body["cases"].as_array().unwrap().len(), 14);
        assert!(run.text.contains("14/14 passed"));
    }

    #[test]
    fn status_on_empty_tree_has_six_contract_keys() {
        let tmp = tempdir().expect("tempdir");
        let ctx = make_ctx(tmp.path(), true);

        let run = handle_status(&ctx);
        assert_eq!(run.exit, ExitCode::Ok);
        assert_eq!(run.status, "PASS");

        let rendered = render("status", &run, true);
        assert_eq!(rendered.code, 0);
        let env: Value = serde_json::from_str(&rendered.stdout).expect("parse status json");
        for key in [
            "golden",
            "candidate",
            "bisect",
            "holds",
            "failures_count",
            "last_release",
        ] {
            assert!(
                env.get(key).is_some(),
                "missing status envelope key '{key}'"
            );
        }
    }

    #[test]
    fn holds_missing_golden_ledger_hold_and_all_last_good() {
        let tmp = tempdir().expect("tempdir");
        let ctx = make_ctx(tmp.path(), false);

        // 1. Missing golden -> exit 1.
        let missing = handle_holds(&ctx, false, None);
        assert_eq!(missing.exit, ExitCode::Error);
        assert!(missing.text.contains("golden.toml missing"));

        // 2. Golden with no ledger entries -> count 0 and "no holds\n".
        let golden = sample_golden();
        write_golden(&ctx.paths.golden(), &golden).expect("write golden");
        let empty_holds = handle_holds(&ctx, false, None);
        assert_eq!(empty_holds.exit, ExitCode::Ok);
        assert_eq!(empty_holds.body["count"], 0);
        assert_eq!(empty_holds.text, "no holds\n");

        // 3. Golden with one ledger entry -> count 1, table line, and HoldFile text.
        record_failure(
            &ctx.paths.failures(),
            Failure {
                component: "example-pkg".to_string(),
                version: "1.1-1".to_string(),
                phase: Phase::Build,
                class: COMPONENT_CLASS.to_string(),
                classifier_rule: "apt-breaks".to_string(),
                log: "/tmp/neb-test/build.log".to_string(),
                recorded_at: "2026-11-11T10:00:00Z".to_string(),
                source: FailureSource::SingleChange,
                members: Vec::new(),
            },
        )
        .expect("record failure");

        let table_run = handle_holds(&ctx, false, None);
        assert_eq!(table_run.exit, ExitCode::Ok);
        assert_eq!(table_run.status, "PASS");
        assert_eq!(table_run.body["count"], 1);
        assert_eq!(
            table_run.text,
            "example-pkg  held 1.0-1  blocked 1.1-1  failure\n"
        );

        let hf_run = handle_holds(&ctx, false, Some(HoldFormat::HoldFile));
        assert_eq!(hf_run.exit, ExitCode::Ok);
        let parsed_holds: Vec<Hold> =
            serde_json::from_value(hf_run.body["holds"].clone()).expect("deserialize holds");
        assert_eq!(parsed_holds.len(), 1);
        assert_eq!(parsed_holds[0].component, "example-pkg");
        assert_eq!(parsed_holds[0].reason, HoldReason::Failure);
        let expected_hf = hold_file(parsed_holds, &ctx.timestamp(), "neb release holds");
        assert_eq!(
            hf_run.text,
            render_hold_file(&expected_hf).expect("render_hold_file")
        );

        // 4. all_last_good -> one hold per golden component.
        let all_run = handle_holds(&ctx, true, None);
        assert_eq!(all_run.exit, ExitCode::Ok);
        assert_eq!(all_run.body["count"], 2);
    }

    #[test]
    fn pins_from_release_empty_written_and_unknown_component() {
        let tmp = tempdir().expect("tempdir");
        let ctx = make_ctx(tmp.path(), false);

        // 1. latest with no records -> exit 1.
        let empty = handle_pins_from_release(&ctx, "latest", &[]);
        assert_eq!(empty.exit, ExitCode::Error);
        assert!(empty.text.contains("no release records in"));

        // 2. Write a release record -> returns its holds (both via "latest" and explicit path).
        let rec = ReleaseRecord {
            schema_version: RELEASE_RECORD_SCHEMA_VERSION,
            version: "2026.11".to_string(),
            promoted_at: "2026-11-10T10:00:00Z".to_string(),
            promoted_by: "neb release promote".to_string(),
            reason: String::new(),
            snapshot: "20261110T000000Z".to_string(),
            iso_sha256: "a".repeat(64),
            golden_sha256: "b".repeat(64),
            hold_file_sha256: String::new(),
            candidate_id: "aaaaaaaaaaaa-20261110T100000Z".to_string(),
            bisect_rounds: 0,
            phases: BTreeMap::new(),
            components: BTreeMap::from([(
                "example-pkg".to_string(),
                RecordComponent {
                    version: "1.0-1".to_string(),
                    origin: Origin::Debian,
                    binaries: BTreeMap::from([("example-pkg".to_string(), "1.0-1".to_string())]),
                },
            )]),
            third_party_cache: BTreeMap::new(),
            candidate: None,
        };
        let rec_file = "2026.11-20261110T100000Z.toml";
        let rec_path = ctx.paths.releases_dir().join(rec_file);
        write_record(&rec_path, &rec).expect("write record");

        let ok_run = handle_pins_from_release(&ctx, "latest", &[]);
        assert_eq!(ok_run.exit, ExitCode::Ok);
        assert_eq!(ok_run.status, "PASS");
        assert_eq!(ok_run.body["release_record"], rec_file);
        let parsed_holds: Vec<Hold> =
            serde_json::from_value(ok_run.body["holds"].clone()).expect("deserialize holds");
        assert_eq!(parsed_holds.len(), 1);
        assert_eq!(parsed_holds[0].component, "example-pkg");
        assert_eq!(parsed_holds[0].reason, HoldReason::Pin);
        let expected_hf = hold_file(
            parsed_holds,
            &ctx.timestamp(),
            "neb release pins-from-release",
        );
        assert_eq!(
            ok_run.text,
            render_hold_file(&expected_hf).expect("render_hold_file")
        );

        let by_path_run =
            handle_pins_from_release(&ctx, rec_path.to_str().unwrap(), &["example-pkg".into()]);
        assert_eq!(by_path_run.exit, ExitCode::Ok);
        assert_eq!(by_path_run.body["release_record"], rec_file);

        // 3. Unknown component -> exit 1, naming it.
        let unknown = handle_pins_from_release(&ctx, "latest", &["missing-pkg".to_string()]);
        assert_eq!(unknown.exit, ExitCode::Error);
        assert!(unknown.text.contains("missing-pkg"));
    }

    #[test]
    fn extract_golden_refusal_paths() {
        let tmp = tempdir().expect("tempdir");
        let ctx_human = make_ctx(tmp.path(), false);
        let missing_iso = Path::new("/tmp/neb-test/nonexistent.iso");

        // 1. No output and not json -> exit 1, and ISO is never touched.
        let no_out = handle_extract_golden(&ctx_human, missing_iso, None);
        assert_eq!(no_out.exit, ExitCode::Error);
        assert!(no_out.text.contains("--output is required without --json"));

        // 2. Output == the golden path -> exit 1 (before and after .tracker/release exists, and via symlink parent).
        let golden_path = ctx_human.paths.golden();
        let to_golden = handle_extract_golden(&ctx_human, missing_iso, Some(&golden_path));
        assert_eq!(to_golden.exit, ExitCode::Error);
        assert!(to_golden.text.contains("refuses to write to"));

        fs::create_dir_all(ctx_human.paths.root()).expect("mkdir release");
        let to_golden_canon = handle_extract_golden(&ctx_human, missing_iso, Some(&golden_path));
        assert_eq!(to_golden_canon.exit, ExitCode::Error);
        assert!(to_golden_canon.text.contains("refuses to write to"));

        #[cfg(unix)]
        {
            let sym_dir = tmp.path().join("release-link");
            std::os::unix::fs::symlink(ctx_human.paths.root(), &sym_dir).expect("symlink release");
            let sym_golden = sym_dir.join("golden.toml");
            let to_sym_golden = handle_extract_golden(&ctx_human, missing_iso, Some(&sym_golden));
            assert_eq!(to_sym_golden.exit, ExitCode::Error);
            assert!(to_sym_golden.text.contains("refuses to write to"));
        }

        // 3. Nonexistent ISO with a valid output -> exit 1.
        let valid_out = tmp.path().join("extracted-golden.toml");
        let bad_iso = handle_extract_golden(&ctx_human, missing_iso, Some(&valid_out));
        assert_eq!(bad_iso.exit, ExitCode::Error);
        assert!(bad_iso.text.contains("ISO file not found"));
    }

    #[test]
    fn verify_variance_repo_only_missing_manifest_clean_and_violation() {
        let tmp = tempdir().expect("tempdir");
        let ctx = make_ctx(tmp.path(), false);

        // 1. Missing manifest -> exit 1.
        let missing = handle_verify_variance(&ctx, None, true, None, None, None);
        assert_eq!(missing.exit, ExitCode::Error);
        assert!(missing.text.contains("manifest.toml missing"));

        // 2. Clean repo tree -> exit 0 / PASS.
        write_clean_repo(tmp.path());

        let clean = handle_verify_variance(&ctx, None, true, None, None, None);
        assert_eq!(clean.exit, ExitCode::Ok);
        assert_eq!(clean.status, "PASS");
        assert_eq!(clean.body["repo_only"], true);
        assert_eq!(clean.body["checked_components_count"], 0);
        assert!(clean.body["violations"].as_array().unwrap().is_empty());
        assert_eq!(clean.text, "variance: PASS\n");

        // 3. Remove required custom-tools/manifest.yaml -> exit 2 with non-empty violations.
        fs::remove_file(tmp.path().join("custom-tools").join("manifest.yaml"))
            .expect("remove custom-tools/manifest.yaml");
        let broken = handle_verify_variance(&ctx, None, true, None, None, None);
        assert_eq!(broken.exit, ExitCode::VarianceLock);
        assert_eq!(broken.exit.code(), 2);
        assert_eq!(broken.status, HARD_STOP_VARIANCE_LOCK);
        assert!(!broken.body["violations"].as_array().unwrap().is_empty());
        assert!(broken.text.contains("variance: HARD_STOP_VARIANCE_LOCK"));
    }

    #[test]
    fn record_attempt_and_check_guardrail_lifecycle() {
        let tmp = tempdir().expect("tempdir");
        let ctx = make_ctx(tmp.path(), false);
        write_clean_repo(tmp.path());

        // Missing item on check-guardrail -> exit 1.
        let missing = handle_check_guardrail(&ctx, "item-001-example");
        assert_eq!(missing.exit, ExitCode::Error);
        assert!(missing.text.contains("loop-state item not found"));

        // First fail -> exit 0, unlocked.
        let r1 = handle_record_attempt(
            &ctx,
            "item-001-example",
            StageArg::Build,
            ResultArg::Fail,
            "dpkg failed",
            None,
        );
        assert_eq!(r1.exit, ExitCode::Ok);
        assert_eq!(r1.status, "ATTEMPT_RECORDED");
        assert_eq!(r1.body["retry_count"], 1);
        assert_eq!(r1.body["loop_locked"], false);

        // Unlocked item with clean repo -> exit 0 / PASS on check-guardrail.
        let cg_ok = handle_check_guardrail(&ctx, "item-001-example");
        assert_eq!(cg_ok.exit, ExitCode::Ok);
        assert_eq!(cg_ok.status, "PASS");
        assert_eq!(cg_ok.body["item_status"], "in_progress");
        assert_eq!(cg_ok.body["variance_status"], "PASS");

        // Unlocked item with repo violation -> exit 2 / HARD_STOP_VARIANCE_LOCK on check-guardrail.
        let ct_manifest = tmp.path().join("custom-tools").join("manifest.yaml");
        let saved_ct = fs::read_to_string(&ct_manifest).expect("read manifest.yaml");
        fs::remove_file(&ct_manifest).expect("remove manifest.yaml");
        let cg_var = handle_check_guardrail(&ctx, "item-001-example");
        assert_eq!(cg_var.exit, ExitCode::VarianceLock);
        assert_eq!(cg_var.exit.code(), 2);
        assert_eq!(cg_var.status, HARD_STOP_VARIANCE_LOCK);
        assert_eq!(cg_var.body["variance_status"], HARD_STOP_VARIANCE_LOCK);
        fs::write(&ct_manifest, saved_ct).expect("restore manifest.yaml");

        // Second identical fail -> exit 0, negative constraint added.
        let r2 = handle_record_attempt(
            &ctx,
            "item-001-example",
            StageArg::Build,
            ResultArg::Fail,
            "dpkg failed",
            None,
        );
        assert_eq!(r2.exit, ExitCode::Ok);
        assert_eq!(r2.body["retry_count"], 2);
        assert_eq!(r2.body["loop_locked"], false);
        assert!(!r2.body["negative_constraints"]
            .as_array()
            .unwrap()
            .is_empty());

        // Third identical fail -> exit 3 / HARD_STOP_LOOP_LOCK, file at loop_state_item path.
        let r3 = handle_record_attempt(
            &ctx,
            "item-001-example",
            StageArg::Build,
            ResultArg::Fail,
            "dpkg failed",
            None,
        );
        assert_eq!(r3.exit, ExitCode::LoopLock);
        assert_eq!(r3.exit.code(), 3);
        assert_eq!(r3.status, HARD_STOP_LOOP_LOCK);
        assert_eq!(r3.body["retry_count"], 3);
        assert_eq!(r3.body["loop_locked"], true);
        let expected_path = ctx
            .paths
            .loop_state_item("item-001-example")
            .expect("item path");
        assert!(expected_path.is_file());

        // Locked item on check-guardrail -> exit 3, item_status present, no body `status` key.
        let cg_locked = handle_check_guardrail(&ctx, "item-001-example");
        assert_eq!(cg_locked.exit, ExitCode::LoopLock);
        assert_eq!(cg_locked.exit.code(), 3);
        assert_eq!(cg_locked.status, HARD_STOP_LOOP_LOCK);
        let body_obj = cg_locked.body.as_object().expect("body object");
        assert!(body_obj.contains_key("item_status"));
        assert!(!body_obj.contains_key("status"));
        assert_eq!(body_obj["item_status"], "blocked_loop_hard_stop");
        assert_eq!(body_obj["loop_locked"], true);
        assert!(cg_locked.text.starts_with(
            "GUARDRAIL_STATE: item=item-001-example status=blocked_loop_hard_stop locked=true retry_count=3 negative_constraints=1 variance=PASS"
        ));
    }
}
