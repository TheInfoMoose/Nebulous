use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Result, TrackerError};
use crate::release::approvals::read_approvals;
use crate::release::config::{
    render_config, validate_config, Components, Guardrails, ReleaseConfig, ReleasePaths,
};
use crate::release::golden::{
    parse_golden, render_golden, Golden, GoldenComponent, Origin, GOLDEN_SCHEMA_VERSION,
};
use crate::release::loopstate::read_loop_item;
use crate::release::manifest::parse_manifest;

pub const SRC_GOLDEN: &str = "golden-snapshot.toml";
pub const SRC_MANIFEST: &str = "manifest.toml";
pub const SRC_APPROVALS: &str = "approvals.toml";
pub const SRC_PROJECT: &str = "project.toml";
pub const SRC_ITEMS: &str = "items";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportFile {
    pub dest: PathBuf,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportReport {
    pub golden_schema_from: u32,
    pub golden_converted: bool,
    pub placeholders: usize,
    pub components: usize,
    pub manifest_copied: bool,
    pub approvals_copied: bool,
    pub approvals: usize,
    pub guardrails_from_project: bool,
    pub loop_items: Vec<String>,
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPlan {
    pub files: Vec<ImportFile>,
    pub report: ImportReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportOutcome {
    Ready(Box<ImportPlan>),
    Conflict { existing: Vec<PathBuf> },
}

pub fn golden_v1_to_v2(mut golden: Golden, components: &Components) -> (Golden, usize) {
    if !golden.components.is_empty() {
        golden.schema_version = GOLDEN_SCHEMA_VERSION;
        return (golden, 0);
    }

    for (binary, version) in &golden.packages {
        let origin = if components.third_party.iter().any(|tp| tp == binary) {
            Origin::ThirdParty
        } else {
            Origin::Debian
        };
        golden.components.insert(
            binary.clone(),
            GoldenComponent {
                version: version.clone(),
                origin,
                kernel_line: false,
                source_unknown: true,
                binaries: BTreeMap::from([(binary.clone(), version.clone())]),
            },
        );
    }

    let count = golden.components.len();
    golden.schema_version = GOLDEN_SCHEMA_VERSION;
    (golden, count)
}

pub fn config_from_project(project_toml: &str, path: &Path) -> Result<(ReleaseConfig, bool)> {
    let mut table: toml::Table =
        toml::from_str(project_toml).map_err(|source| TrackerError::TomlDeserialize {
            path: Some(path.to_path_buf()),
            source,
        })?;

    let Some(raw_guardrails) = table.remove("guardrails") else {
        return Ok((ReleaseConfig::default(), false));
    };

    let toml::Value::Table(guardrails_table) = raw_guardrails else {
        return Err(TrackerError::ValidationError(format!(
            "{}: [guardrails] must be a TOML table",
            path.display()
        )));
    };

    let guardrails =
        Guardrails::deserialize(toml::Value::Table(guardrails_table)).map_err(|source| {
            TrackerError::TomlDeserialize {
                path: Some(path.to_path_buf()),
                source,
            }
        })?;

    let cfg = ReleaseConfig {
        guardrails,
        ..ReleaseConfig::default()
    };
    validate_config(&cfg, path)?;
    Ok((cfg, true))
}

fn read_required_source(path: &Path) -> Result<String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(err) if err.kind() == ErrorKind::NotFound => Err(TrackerError::ValidationError(
            format!("missing required import source '{}'", path.display()),
        )),
        Err(source) => Err(TrackerError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn dest_exists(path: &Path) -> bool {
    path.exists() || fs::symlink_metadata(path).is_ok()
}

pub fn plan_import(from: &Path, paths: &ReleasePaths) -> Result<ImportOutcome> {
    if !from.is_dir() {
        return Err(TrackerError::ValidationError(format!(
            "import source '{}' is not a directory",
            from.display()
        )));
    }

    let golden_src = from.join(SRC_GOLDEN);
    let golden_raw_text = read_required_source(&golden_src)?;
    let raw_golden = parse_golden(&golden_raw_text, &golden_src)?;
    let golden_schema_from = raw_golden.schema_version;
    let golden_converted = raw_golden.components.is_empty();

    let manifest_src = from.join(SRC_MANIFEST);
    let manifest_text = read_required_source(&manifest_src)?;
    parse_manifest(&manifest_text, &manifest_src)?;

    let approvals_src = from.join(SRC_APPROVALS);
    let (approvals_file, approvals_copied, approvals_count) =
        match fs::read_to_string(&approvals_src) {
            Ok(text) => {
                let parsed = read_approvals(&approvals_src)?;
                let count = parsed.approvals.len();
                (
                    Some(ImportFile {
                        dest: paths.approvals(),
                        text,
                    }),
                    true,
                    count,
                )
            }
            Err(err) if err.kind() == ErrorKind::NotFound => (None, false, 0),
            Err(source) => {
                return Err(TrackerError::Io {
                    path: approvals_src,
                    source,
                });
            }
        };

    let project_src = from.join(SRC_PROJECT);
    let project_text = read_required_source(&project_src)?;
    let (cfg, guardrails_from_project) = config_from_project(&project_text, &project_src)?;
    let config_text = render_config(&cfg)?;

    let (converted_golden, placeholders) = golden_v1_to_v2(raw_golden, &cfg.components);
    let components = converted_golden.components.len();
    let golden_text = render_golden(&converted_golden)?;

    let items_src_dir = from.join(SRC_ITEMS);
    let mut loop_items_by_id: BTreeMap<String, (PathBuf, ImportFile)> = BTreeMap::new();
    if items_src_dir.is_dir() {
        let mut item_paths: Vec<PathBuf> = Vec::new();
        let entries = fs::read_dir(&items_src_dir).map_err(|source| TrackerError::Io {
            path: items_src_dir.clone(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| TrackerError::Io {
                path: items_src_dir.clone(),
                source,
            })?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            if path.is_file() {
                item_paths.push(path);
            }
        }
        item_paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        for item_path in item_paths {
            let text = fs::read_to_string(&item_path).map_err(|source| TrackerError::Io {
                path: item_path.clone(),
                source,
            })?;
            let parsed = read_loop_item(&item_path)?.ok_or_else(|| {
                TrackerError::ValidationError(format!(
                    "loop-state item '{}' disappeared during import planning",
                    item_path.display()
                ))
            })?;
            let trimmed_id = parsed.item.id.trim();
            let id = if trimmed_id.is_empty() {
                item_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string()
            } else {
                trimmed_id.to_string()
            };
            let dest = paths.loop_state_item(&id)?;
            if let Some((first_path, _)) = loop_items_by_id.get(&id) {
                return Err(TrackerError::ValidationError(format!(
                    "duplicate loop-state item id '{id}' in {} and {}",
                    first_path.display(),
                    item_path.display()
                )));
            }
            loop_items_by_id.insert(id, (item_path, ImportFile { dest, text }));
        }
    }

    let mut skipped: Vec<String> = Vec::new();
    let top_entries = fs::read_dir(from).map_err(|source| TrackerError::Io {
        path: from.to_path_buf(),
        source,
    })?;
    for entry in top_entries {
        let entry = entry.map_err(|source| TrackerError::Io {
            path: from.to_path_buf(),
            source,
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if matches!(
            name.as_str(),
            SRC_GOLDEN | SRC_MANIFEST | SRC_APPROVALS | SRC_PROJECT | SRC_ITEMS
        ) {
            continue;
        }
        let is_dir = entry
            .file_type()
            .map(|ft| ft.is_dir())
            .unwrap_or_else(|_| entry.path().is_dir());
        if is_dir {
            skipped.push(format!("{name}/"));
        } else {
            skipped.push(name);
        }
    }
    skipped.sort();

    let mut files = Vec::new();
    files.push(ImportFile {
        dest: paths.golden(),
        text: golden_text,
    });
    files.push(ImportFile {
        dest: paths.manifest(),
        text: manifest_text,
    });
    if let Some(af) = approvals_file {
        files.push(af);
    }
    files.push(ImportFile {
        dest: paths.config(),
        text: config_text,
    });

    let mut loop_items = Vec::with_capacity(loop_items_by_id.len());
    for (id, (_src, file)) in loop_items_by_id {
        loop_items.push(id);
        files.push(file);
    }

    let mut existing: Vec<PathBuf> = files
        .iter()
        .filter(|f| dest_exists(&f.dest))
        .map(|f| f.dest.clone())
        .collect();
    existing.sort();
    if !existing.is_empty() {
        return Ok(ImportOutcome::Conflict { existing });
    }

    let report = ImportReport {
        golden_schema_from,
        golden_converted,
        placeholders,
        components,
        manifest_copied: true,
        approvals_copied,
        approvals: approvals_count,
        guardrails_from_project,
        loop_items,
        skipped,
    };

    Ok(ImportOutcome::Ready(Box::new(ImportPlan { files, report })))
}

fn rollback_import(written: &[PathBuf], original: TrackerError) -> TrackerError {
    let mut first_rollback_err: Option<TrackerError> = None;
    for path in written.iter().rev() {
        if let Err(source) = fs::remove_file(path) {
            if first_rollback_err.is_none() {
                first_rollback_err = Some(TrackerError::Io {
                    path: path.clone(),
                    source,
                });
            }
        }
    }

    if let Some(rollback_err) = first_rollback_err {
        let paths = written
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        TrackerError::ValidationError(format!(
            "import failed: {original}; rollback also failed: {rollback_err}; check {paths}"
        ))
    } else {
        original
    }
}

/// Writes all planned import files in order, rolling back any files written by this call on failure.
///
/// The caller must hold `TrackerStorage::with_write_lock` across [`plan_import`] and this call
/// and must have passed the §5.1 user-only gate.
pub fn commit_import(plan: &ImportPlan) -> Result<Vec<PathBuf>> {
    let mut written: Vec<PathBuf> = Vec::with_capacity(plan.files.len());

    for file in &plan.files {
        if fs::symlink_metadata(&file.dest).is_ok() {
            let err = TrackerError::ValidationError(format!(
                "{} appeared after planning; import writes only new files",
                file.dest.display()
            ));
            return Err(rollback_import(&written, err));
        }

        if let Some(parent) = file.dest.parent() {
            if !parent.as_os_str().is_empty() {
                if let Err(source) = fs::create_dir_all(parent) {
                    let err = TrackerError::Io {
                        path: parent.to_path_buf(),
                        source,
                    };
                    return Err(rollback_import(&written, err));
                }
            }
        }

        if let Err(err) = crate::storage::atomic_write_str(&file.dest, &file.text) {
            return Err(rollback_import(&written, err));
        }

        written.push(file.dest.clone());
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use tempfile::{tempdir, TempDir};

    use super::*;
    use crate::release::config::{load_config, DEFAULT_AGENT_ENV_MARKERS, DEFAULT_BUILD_HARNESS};
    use crate::release::golden::read_golden;
    use crate::storage::TrackerStorage;

    const SAMPLE_V1_GOLDEN: &str = "\
# Legacy v1 golden snapshot
schema_version = 1

[iso]
path = \"/tmp/neb-test/example.iso\"
size_bytes = 1048576
vmlinuz_sha256 = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"
initrd_sha256 = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"
extracted_at = \"2026-11-08T12:00:00Z\"

[os_release]
ID = \"exampleos\"
VERSION_ID = \"2026.11\"

[boot]
files = [\"initrd.img\", \"vmlinuz\"]

[calamares_modules]
configs = [\"welcome.conf\"]
custom_modules = []

[calamares_modules.presence]
\"welcome.conf\" = true

[custom_binaries]

[packages]
example-pkg = \"1.0-1\"
example-tool = \"2.3-1\"
libexample1 = \"0.5-2\"
";

    const SAMPLE_MANIFEST: &str = "\
# Minimal release manifest comment preserved verbatim
schema_version = 1

[os]
required = true
distro_id = \"exampleos\"

[packages.core]
example-pkg = { required = true, min_version = \"1.0-1\" }
";

    const SAMPLE_PROJECT: &str = "\
[project]
name = \"exampleos\"

[guardrails]
loop_repeat_warn_threshold = 4
";

    const SAMPLE_ITEM: &str = "\
# Item comment preserved verbatim
[item]
id = \"item-001-example\"
phase_id = \"phase-1\"
title = \"Example CI item\"
status = \"in_progress\"
loop_locked = false
retry_count = 1
negative_constraints = []

[[attempts]]
timestamp = \"2026-11-08T12:00:00Z\"
stage = \"build\"
status = \"fail\"
error_summary = \"example failure\"
error_fingerprint = \"0123456789abcdef\"
diff_fingerprint = \"clean\"
modified_files = []
log_file = \"/tmp/neb-test/build.log\"
";

    const SAMPLE_APPROVALS: &str = "\
# Approvals comment preserved verbatim
[[approvals]]
item_name = \"libexample1\"
reason = \"Approved test exception\"
approved = true
timestamp = \"2026-11-08T12:00:00Z\"
";

    fn setup_fixture() -> (TempDir, PathBuf, ReleasePaths) {
        let tmp = tempdir().expect("tempdir");
        let storage = TrackerStorage::init_standard(tmp.path(), "test-proj").expect("init");
        let paths = ReleasePaths::new(&storage);

        let src = tmp.path().join(".nebulous");
        fs::create_dir_all(src.join(SRC_ITEMS)).expect("create items dir");
        fs::create_dir_all(src.join("phases")).expect("create phases dir");

        fs::write(src.join(SRC_GOLDEN), SAMPLE_V1_GOLDEN).expect("write golden");
        fs::write(src.join(SRC_MANIFEST), SAMPLE_MANIFEST).expect("write manifest");
        fs::write(src.join(SRC_PROJECT), SAMPLE_PROJECT).expect("write project");
        fs::write(
            src.join(SRC_ITEMS).join("item-001-example.toml"),
            SAMPLE_ITEM,
        )
        .expect("write item");

        (tmp, src, paths)
    }

    #[test]
    fn golden_v1_to_v2_builds_placeholders_and_preserves_existing_v2() {
        let raw =
            parse_golden(SAMPLE_V1_GOLDEN, Path::new("golden-snapshot.toml")).expect("parse v1");
        let mut comps_cfg = Components::default();
        comps_cfg.third_party.push("example-tool".to_string());

        let (v2, count) = golden_v1_to_v2(raw, &comps_cfg);
        assert_eq!(count, 3);
        assert_eq!(v2.schema_version, GOLDEN_SCHEMA_VERSION);
        assert_eq!(v2.components.len(), 3);

        for (name, expected_ver, expected_origin) in [
            ("example-pkg", "1.0-1", Origin::Debian),
            ("example-tool", "2.3-1", Origin::ThirdParty),
            ("libexample1", "0.5-2", Origin::Debian),
        ] {
            let comp = v2.components.get(name).expect("component present");
            assert_eq!(comp.version, expected_ver);
            assert_eq!(comp.origin, expected_origin);
            assert!(comp.source_unknown);
            assert!(!comp.kernel_line);
            assert_eq!(
                comp.binaries,
                BTreeMap::from([(name.to_string(), expected_ver.to_string())])
            );
        }

        // A golden that already has components is unchanged apart from schema_version and returns 0.
        let mut already_v2 = v2.clone();
        already_v2.schema_version = 1;
        let (unchanged, second_count) = golden_v1_to_v2(already_v2, &Components::default());
        assert_eq!(second_count, 0);
        assert_eq!(unchanged, v2);
    }

    #[test]
    fn config_from_project_carries_guardrails_defaults_and_validates() {
        let path = Path::new("project.toml");
        let (cfg, from_proj) =
            config_from_project(SAMPLE_PROJECT, path).expect("config_from_project");
        assert!(from_proj);
        assert_eq!(cfg.guardrails.loop_repeat_warn_threshold, 4);
        assert_eq!(
            cfg.guardrails.agent_env_markers,
            DEFAULT_AGENT_ENV_MARKERS
                .iter()
                .map(|s| (*s).to_string())
                .collect::<Vec<_>>()
        );

        let (default_cfg, has_guardrails) =
            config_from_project("[project]\nname = \"x\"\n", path).expect("no guardrails");
        assert!(!has_guardrails);
        assert_eq!(default_cfg, ReleaseConfig::default());

        let relaxed = "[guardrails]\nallow_package_removal = true\n";
        let err = config_from_project(relaxed, path).expect_err("relaxed guardrail must fail");
        match err {
            TrackerError::ValidationError(msg) => {
                assert!(msg.contains("allow_package_removal"), "unexpected: {msg}");
            }
            other => panic!("expected ValidationError, got {other:?}"),
        }

        let non_table = "guardrails = 1\n";
        let err_non_table =
            config_from_project(non_table, path).expect_err("non-table guardrails must fail");
        assert!(matches!(err_non_table, TrackerError::ValidationError(_)));
    }

    #[test]
    fn happy_path_plan_and_commit_without_approvals() {
        let (_tmp, src, paths) = setup_fixture();

        let outcome = plan_import(&src, &paths).expect("plan_import");
        let ImportOutcome::Ready(plan) = outcome else {
            panic!("expected Ready, got {outcome:?}");
        };

        assert_eq!(plan.files.len(), 4);
        assert_eq!(
            plan.report,
            ImportReport {
                golden_schema_from: 1,
                golden_converted: true,
                placeholders: 3,
                components: 3,
                manifest_copied: true,
                approvals_copied: false,
                approvals: 0,
                guardrails_from_project: true,
                loop_items: vec!["item-001-example".to_string()],
                skipped: vec!["phases/".to_string()],
            }
        );

        let written = commit_import(&plan).expect("commit_import");
        assert_eq!(written.len(), 4);

        let golden = read_golden(&paths.golden())
            .expect("read_golden")
            .expect("golden exists");
        assert_eq!(golden.schema_version, 2);
        assert_eq!(golden.components.len(), 3);
        assert!(golden.components.values().all(|c| c.source_unknown));

        let cfg = load_config(&paths).expect("load_config");
        assert_eq!(cfg.guardrails.loop_repeat_warn_threshold, 4);
        assert_eq!(
            cfg.harness.build,
            DEFAULT_BUILD_HARNESS
                .iter()
                .map(|s| (*s).to_string())
                .collect::<Vec<_>>()
        );

        let manifest_disk = fs::read_to_string(paths.manifest()).expect("read manifest");
        assert_eq!(manifest_disk, SAMPLE_MANIFEST);

        let item_dest = paths
            .loop_state_item("item-001-example")
            .expect("item path");
        let item_disk = fs::read_to_string(item_dest).expect("read item");
        assert_eq!(item_disk, SAMPLE_ITEM);
    }

    #[test]
    fn happy_path_with_approvals_copies_five_files_verbatim() {
        let (_tmp, src, paths) = setup_fixture();
        fs::write(src.join(SRC_APPROVALS), SAMPLE_APPROVALS).expect("write approvals");

        let outcome = plan_import(&src, &paths).expect("plan_import");
        let ImportOutcome::Ready(plan) = outcome else {
            panic!("expected Ready, got {outcome:?}");
        };

        assert_eq!(plan.files.len(), 5);
        assert!(plan.report.approvals_copied);
        assert_eq!(plan.report.approvals, 1);

        commit_import(&plan).expect("commit_import");
        let approvals_disk = fs::read_to_string(paths.approvals()).expect("read approvals");
        assert_eq!(approvals_disk, SAMPLE_APPROVALS);
    }

    #[test]
    fn conflict_when_destination_exists_writes_nothing() {
        let (_tmp, src, paths) = setup_fixture();
        fs::create_dir_all(paths.root()).expect("create release root");
        fs::write(paths.golden(), "existing").expect("pre-create golden");

        let outcome = plan_import(&src, &paths).expect("plan_import");
        assert_eq!(
            outcome,
            ImportOutcome::Conflict {
                existing: vec![paths.golden()],
            }
        );
        assert!(!paths.manifest().exists());
        assert!(!paths.config().exists());
        assert!(!paths.loop_state_dir().exists());
    }

    #[test]
    fn refusals_are_write_free() {
        // 1. Missing golden source
        {
            let (_tmp, src, paths) = setup_fixture();
            fs::remove_file(src.join(SRC_GOLDEN)).expect("remove golden");
            let err = plan_import(&src, &paths).expect_err("missing golden must fail");
            match err {
                TrackerError::ValidationError(msg) => {
                    assert!(msg.contains(SRC_GOLDEN), "unexpected: {msg}");
                }
                other => panic!("expected ValidationError, got {other:?}"),
            }
            assert!(!paths.root().exists());
        }

        // 2. Missing manifest
        {
            let (_tmp, src, paths) = setup_fixture();
            fs::remove_file(src.join(SRC_MANIFEST)).expect("remove manifest");
            let err = plan_import(&src, &paths).expect_err("missing manifest must fail");
            match err {
                TrackerError::ValidationError(msg) => {
                    assert!(msg.contains(SRC_MANIFEST), "unexpected: {msg}");
                }
                other => panic!("expected ValidationError, got {other:?}"),
            }
            assert!(!paths.root().exists());
        }

        // 3. Relaxed guardrail in project.toml
        {
            let (_tmp, src, paths) = setup_fixture();
            fs::write(
                src.join(SRC_PROJECT),
                "[guardrails]\nallow_version_downgrade = true\n",
            )
            .expect("write relaxed project");
            let err = plan_import(&src, &paths).expect_err("relaxed guardrail must fail");
            match err {
                TrackerError::ValidationError(msg) => {
                    assert!(msg.contains("allow_version_downgrade"), "unexpected: {msg}");
                }
                other => panic!("expected ValidationError, got {other:?}"),
            }
            assert!(!paths.root().exists());
        }

        // 4. Item with invalid id `bad id`
        {
            let (_tmp, src, paths) = setup_fixture();
            fs::write(
                src.join(SRC_ITEMS).join("bad.toml"),
                "[item]\nid = \"bad id\"\n",
            )
            .expect("write bad item");
            let err = plan_import(&src, &paths).expect_err("invalid item id must fail");
            assert!(matches!(err, TrackerError::ValidationError(_)));
            assert!(!paths.root().exists());
        }

        // 5. Two items with the same id
        {
            let (_tmp, src, paths) = setup_fixture();
            fs::write(src.join(SRC_ITEMS).join("second.toml"), SAMPLE_ITEM)
                .expect("write duplicate item");
            let err = plan_import(&src, &paths).expect_err("duplicate item id must fail");
            match err {
                TrackerError::ValidationError(msg) => {
                    assert!(msg.contains("item-001-example"), "unexpected: {msg}");
                    assert!(msg.contains("item-001-example.toml"), "unexpected: {msg}");
                    assert!(msg.contains("second.toml"), "unexpected: {msg}");
                }
                other => panic!("expected ValidationError, got {other:?}"),
            }
            assert!(!paths.root().exists());
        }

        // 6. `from` is a file
        {
            let (tmp, _src, paths) = setup_fixture();
            let file_src = tmp.path().join("not-a-dir");
            fs::write(&file_src, "hello").expect("write file");
            let err = plan_import(&file_src, &paths).expect_err("file source must fail");
            assert!(matches!(err, TrackerError::ValidationError(_)));
            assert!(!paths.root().exists());
        }
    }

    #[test]
    fn commit_rollback_removes_previously_written_files_on_failure() {
        let (_tmp, src, paths) = setup_fixture();
        let ImportOutcome::Ready(plan) = plan_import(&src, &paths).expect("plan_import") else {
            panic!("expected Ready");
        };

        // Create a regular file at paths.loop_state_dir() so creating the item's parent dir fails.
        fs::create_dir_all(paths.root()).expect("create release dir");
        fs::write(paths.loop_state_dir(), "blocker").expect("write blocker file");

        let _err = commit_import(&plan).expect_err("commit_import must fail on blocked parent dir");
        assert!(!paths.golden().exists());
        assert!(!paths.manifest().exists());
        assert!(!paths.config().exists());
    }

    #[test]
    fn commit_recheck_aborts_and_rolls_back_when_dest_appears_after_planning() {
        let (_tmp, src, paths) = setup_fixture();
        let ImportOutcome::Ready(plan) = plan_import(&src, &paths).expect("plan_import") else {
            panic!("expected Ready");
        };

        // Create paths.manifest() after planning so golden.toml writes first and manifest re-check fails.
        fs::create_dir_all(paths.root()).expect("create release dir");
        fs::write(paths.manifest(), "appeared").expect("write manifest");

        let err = commit_import(&plan).expect_err("commit_import must fail when dest appears");
        match err {
            TrackerError::ValidationError(msg) => {
                assert!(msg.contains("manifest.toml"), "unexpected: {msg}");
                assert!(msg.contains("appeared after planning"), "unexpected: {msg}");
            }
            other => panic!("expected ValidationError, got {other:?}"),
        }
        assert!(!paths.golden().exists());
        assert!(!paths.config().exists());
    }

    #[test]
    fn edge_cases_stem_fallback_item_sorting_symlink_conflict_and_missing_project() {
        let (_tmp, src, paths) = setup_fixture();

        // Add an item with a blank id whose file name sorts before item-001-example.toml
        // but whose resolved stem ("z-stem-item") sorts after "item-001-example".
        fs::write(
            src.join(SRC_ITEMS).join("z-stem-item.toml"),
            "[item]\nid = \"   \"\ntitle = \"Blank id uses stem\"\n",
        )
        .expect("write stem item");
        fs::write(src.join("extra.txt"), "ignored").expect("write extra top-level file");

        let ImportOutcome::Ready(plan) = plan_import(&src, &paths).expect("plan_import") else {
            panic!("expected Ready");
        };
        assert_eq!(
            plan.report.loop_items,
            vec!["item-001-example".to_string(), "z-stem-item".to_string()]
        );
        assert_eq!(
            plan.report.skipped,
            vec!["extra.txt".to_string(), "phases/".to_string()]
        );

        // Broken symlink at destination is still detected as Conflict via symlink_metadata.
        #[cfg(unix)]
        {
            fs::create_dir_all(paths.root()).expect("create release root");
            std::os::unix::fs::symlink("/nonexistent/target", paths.config())
                .expect("create broken symlink");
            let conflict = plan_import(&src, &paths).expect("plan_import conflict");
            assert_eq!(
                conflict,
                ImportOutcome::Conflict {
                    existing: vec![paths.config()],
                }
            );
            fs::remove_file(paths.config()).expect("remove broken symlink");
        }

        // Missing project.toml fails with ValidationError naming project.toml.
        fs::remove_file(src.join(SRC_PROJECT)).expect("remove project.toml");
        let err = plan_import(&src, &paths).expect_err("missing project.toml must fail");
        match err {
            TrackerError::ValidationError(msg) => {
                assert!(msg.contains(SRC_PROJECT), "unexpected: {msg}");
            }
            other => panic!("expected ValidationError, got {other:?}"),
        }
    }

    #[test]
    fn rollback_import_surfaces_both_original_and_rollback_errors_when_remove_fails() {
        let dir = tempdir().expect("tempdir");
        let missing_a = dir.path().join("missing_golden.toml");
        let missing_b = dir.path().join("missing_manifest.toml");
        let orig = TrackerError::ValidationError("primary write failure".to_string());
        let combined = rollback_import(&[missing_a.clone(), missing_b.clone()], orig);
        let msg = combined.to_string();
        assert!(
            msg.contains("import failed: Validation error: primary write failure"),
            "unexpected: {msg}"
        );
        assert!(msg.contains("rollback also failed:"), "unexpected: {msg}");
        assert!(
            msg.contains(&missing_a.display().to_string()),
            "unexpected: {msg}"
        );
        assert!(
            msg.contains(&missing_b.display().to_string()),
            "unexpected: {msg}"
        );
    }
}
