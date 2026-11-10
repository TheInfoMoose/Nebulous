use std::cmp::Ordering;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::error::{Result, TrackerError};
use crate::release::approvals::Approvals;
use crate::release::debver;
use crate::release::manifest::Manifest;
use crate::release::rootfs;
use crate::release::Violation;

fn strip_comments(text: &str) -> String {
    text.lines()
        .map(|line| line.split_once('#').map_or(line, |(head, _)| head))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_token_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-')
}

pub fn has_token(text: &str, token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    let bytes = text.as_bytes();
    let token_len = token.len();
    for (idx, _) in text.match_indices(token) {
        let before_ok = idx == 0 || !is_token_byte(bytes[idx - 1]);
        let after_idx = idx + token_len;
        let after_ok = after_idx == bytes.len() || !is_token_byte(bytes[after_idx]);
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Tool {
    name: String,
    enabled: bool,
}

fn scan_tools(text: &str) -> Vec<Tool> {
    let mut tools = Vec::new();
    let mut current: Option<Tool> = None;

    for raw_line in text.lines() {
        let stripped = raw_line.trim();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }
        if let Some(rest) = stripped.strip_prefix("- name:") {
            if let Some(tool) = current.take() {
                tools.push(tool);
            }
            let name = rest.trim().trim_matches(&['"', '\''][..]).to_string();
            current = Some(Tool {
                name,
                enabled: false,
            });
        } else if let Some(ref mut tool) = current {
            if !stripped.starts_with('-') {
                if let Some((k, v)) = stripped.split_once(':') {
                    if k.trim() == "enabled" {
                        let val = v.trim().trim_matches(&['"', '\''][..]);
                        tool.enabled = val.eq_ignore_ascii_case("true");
                    }
                }
            }
        }
    }

    if let Some(tool) = current {
        tools.push(tool);
    }

    tools
}

fn read_optional_string(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(source) => Err(TrackerError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

pub fn verify_repo_invariants(
    manifest: &Manifest,
    approvals: &Approvals,
    repo_root: &Path,
) -> Result<Vec<Violation>> {
    let mut violations = Vec::new();
    let ok = |item: &str| approvals.covers(item, None);

    // 1. Validate custom-tools/manifest.yaml enabled tools
    let ct_yaml = repo_root.join("custom-tools").join("manifest.yaml");
    let ct_yaml_text = read_optional_string(&ct_yaml)?;
    let tools = ct_yaml_text.as_deref().map(scan_tools).unwrap_or_default();
    let custom_tool = tools.iter().rev().find(|t| t.name == "novos-custom-tools");

    match custom_tool {
        None => {
            if !ok("novos-custom-tools") {
                violations.push(Violation::new(
                    "repo_custom_tool_missing",
                    "novos-custom-tools",
                    "novos-custom-tools missing from custom-tools/manifest.yaml",
                ));
            }
        }
        Some(tool) if !tool.enabled => {
            if !ok("novos-custom-tools") {
                violations.push(Violation::new(
                    "repo_custom_tool_disabled",
                    "novos-custom-tools",
                    "novos-custom-tools is disabled in custom-tools/manifest.yaml",
                ));
            }
        }
        Some(_) => {}
    }

    // 2. Validate calamares/modules/* presence and netinstall.yaml structure in repo
    let cal_dir = repo_root.join("calamares").join("modules");
    for cfg in &manifest.calamares_modules.required_configs {
        if !cal_dir.join(cfg).is_file() && !ok(cfg) {
            violations.push(Violation::new(
                "repo_calamares_config_missing",
                cfg.clone(),
                format!("Required Calamares config calamares/modules/{cfg} missing"),
            ));
        }
    }
    for cmod in &manifest.calamares_modules.custom_modules {
        if !cal_dir.join(cmod).is_dir() && !ok(cmod) {
            violations.push(Violation::new(
                "repo_calamares_module_missing",
                cmod.clone(),
                format!("Required Calamares custom module calamares/modules/{cmod} missing"),
            ));
        }
    }

    let netinstall_yaml = cal_dir.join("netinstall.yaml");
    if netinstall_yaml.is_file() && !ok("netinstall.yaml") {
        let raw_ni = fs::read_to_string(&netinstall_yaml).map_err(|source| TrackerError::Io {
            path: netinstall_yaml.clone(),
            source,
        })?;
        let ni_text = strip_comments(&raw_ni);
        if !ni_text.contains("- name:") || !ni_text.contains("packages:") {
            violations.push(Violation::new(
                "repo_calamares_netinstall_invalid",
                "netinstall.yaml",
                "calamares/modules/netinstall.yaml is missing profile groups or package lists",
            ));
        }
    }

    // 3. Validate Cargo.toml crate versions against manifest custom_binaries min_version
    let workspace_cargo = repo_root.join("Cargo.toml");
    if let Some(ws_text) = read_optional_string(&workspace_cargo)? {
        let ws_data: toml::Value =
            toml::from_str(&ws_text).map_err(|source| TrackerError::TomlDeserialize {
                path: Some(workspace_cargo.clone()),
                source,
            })?;

        if let Some(members) = ws_data
            .get("workspace")
            .and_then(toml::Value::as_table)
            .and_then(|ws| ws.get("members"))
            .and_then(toml::Value::as_array)
        {
            for member_val in members {
                let Some(member) = member_val.as_str() else {
                    continue;
                };
                let member_cargo = repo_root.join(member).join("Cargo.toml");
                if !member_cargo.is_file() && !ok(member) {
                    violations.push(Violation::new(
                        "repo_cargo_member_missing",
                        member,
                        format!(
                            "Workspace member Cargo.toml missing: {}",
                            member_cargo.display()
                        ),
                    ));
                }
            }
        }

        let crate_versions = rootfs::workspace_crate_versions(repo_root);
        for (crate_name, crate_ver) in &crate_versions {
            let bin_key = format!("/usr/local/bin/{crate_name}");
            if let Some(bin_spec) = manifest.custom_binaries.get(&bin_key) {
                if debver::compare(crate_ver, &bin_spec.min_version) == Ordering::Less
                    && !approvals.covers(crate_name, Some(crate_ver))
                {
                    violations.push(Violation::new(
                        "repo_cargo_version_downgrade",
                        crate_name.clone(),
                        format!(
                            "Crate {crate_name} version {crate_ver} < manifest min_version {}",
                            bin_spec.min_version
                        ),
                    ));
                }
            }
        }
    }

    // 4. Validate stage script package lists against manifest.toml (ignoring comments)
    let stages_dir = repo_root.join("scripts").join("stages");
    let mut stage_paths: Vec<(String, PathBuf)> = Vec::new();
    match fs::read_dir(&stages_dir) {
        Ok(entries) => {
            for entry_res in entries {
                let entry = entry_res.map_err(|source| TrackerError::Io {
                    path: stages_dir.clone(),
                    source,
                })?;
                let file_name = entry.file_name().to_string_lossy().into_owned();
                if file_name.ends_with(".sh") {
                    stage_paths.push((file_name, entry.path()));
                }
            }
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(source) => {
            return Err(TrackerError::Io {
                path: stages_dir,
                source,
            });
        }
    }
    stage_paths.sort_by(|a, b| a.0.cmp(&b.0));

    let mut stage_texts = Vec::with_capacity(stage_paths.len());
    for (_, path) in stage_paths {
        let text = fs::read_to_string(&path).map_err(|source| TrackerError::Io { path, source })?;
        stage_texts.push(text);
    }

    let mut combined_stages = strip_comments(&stage_texts.join("\n"));
    if let Some(ref ct_text) = ct_yaml_text {
        combined_stages.push('\n');
        combined_stages.push_str(&strip_comments(ct_text));
    }

    for pkg_name in manifest.packages.core.keys() {
        if !has_token(&combined_stages, pkg_name) && !ok(pkg_name) {
            violations.push(Violation::new(
                "repo_stage_package_missing",
                pkg_name.clone(),
                format!("Core package '{pkg_name}' missing from scripts/stages/*.sh"),
            ));
        }
    }

    let has_cosmic_monorepo = has_token(&combined_stages, "cosmic-epoch-monorepo");
    for pkg_name in manifest.packages.cosmic.keys() {
        if !has_token(&combined_stages, pkg_name) && !has_cosmic_monorepo && !ok(pkg_name) {
            violations.push(Violation::new(
                "repo_stage_package_missing",
                pkg_name.clone(),
                format!("COSMIC package '{pkg_name}' missing from scripts/stages/*.sh"),
            ));
        }
    }

    for (bin_path, bin_spec) in &manifest.custom_binaries {
        let bname = bin_path.rsplit('/').next().unwrap_or(bin_path.as_str());
        if !has_token(&combined_stages, bname) && !ok(bin_path) && !ok(bname) {
            violations.push(Violation::new(
                "repo_stage_binary_missing",
                bin_path.clone(),
                format!("Custom binary '{bin_path}' missing from scripts/stages/*.sh"),
            ));
        }
        for req_flag in &bin_spec.required_flags {
            let flag_item = format!("{bin_path}:{req_flag}");
            if !has_token(&combined_stages, req_flag) && !ok(bin_path) && !ok(&flag_item) {
                violations.push(Violation::new(
                    "repo_stage_flag_missing",
                    flag_item,
                    format!(
                        "Required flag '{req_flag}' for '{bin_path}' missing from stage scripts"
                    ),
                ));
            }
        }
    }

    Ok(violations)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tempfile::tempdir;

    use super::*;
    use crate::release::approvals::Approval;
    use crate::release::manifest::{BinarySpec, CalamaresSpec, ManifestPackages, PackageSpec};

    fn open_approval(item_name: &str) -> Approvals {
        Approvals {
            approvals: vec![Approval {
                item_name: item_name.to_string(),
                reason: "Approved in unit test".to_string(),
                approved: true,
                timestamp: "2026-11-08T12:00:00Z".to_string(),
                version: None,
            }],
        }
    }

    fn version_approval(item_name: &str, version: &str) -> Approvals {
        Approvals {
            approvals: vec![Approval {
                item_name: item_name.to_string(),
                reason: "Version-scoped approval in unit test".to_string(),
                approved: true,
                timestamp: "2026-11-08T12:00:00Z".to_string(),
                version: Some(version.to_string()),
            }],
        }
    }

    fn sample_manifest() -> Manifest {
        let mut core = BTreeMap::new();
        core.insert("dracut".to_string(), PackageSpec::default());
        core.insert("btrfs-progs".to_string(), PackageSpec::default());

        let mut cosmic = BTreeMap::new();
        cosmic.insert("cosmic-comp".to_string(), PackageSpec::default());

        let mut custom_binaries = BTreeMap::new();
        custom_binaries.insert(
            "/usr/local/bin/testtool".to_string(),
            BinarySpec {
                required: true,
                min_version: "0.1.0".to_string(),
                required_flags: vec!["--pre-packages".to_string()],
            },
        );

        Manifest {
            schema_version: 1,
            os: Default::default(),
            packages: ManifestPackages { core, cosmic },
            custom_binaries,
            calamares_modules: CalamaresSpec {
                required: true,
                required_configs: vec!["fstab.conf".to_string()],
                custom_modules: vec!["limine-bootloader".to_string()],
            },
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
    }

    #[test]
    fn has_token_and_strip_comments_respect_token_boundaries_and_comments() {
        assert!(has_token("apt install dracut btrfs-progs", "dracut"));
        assert!(!has_token("dracut-core", "dracut"));
        assert!(!has_token("btrfs-progs", "btrfs"));
        assert!(has_token("dracut", "dracut"));
        assert!(has_token("dracut ", "dracut"));
        assert!(has_token(" dracut", "dracut"));
        assert!(has_token("novos-setup --pre-packages", "--pre-packages"));
        assert!(!has_token("novos-setup ---pre-packages", "--pre-packages"));
        assert!(!has_token("anything", ""));

        let commented = "# apt install dracut\napt install btrfs-progs # dracut\n";
        let stripped = strip_comments(commented);
        assert!(!has_token(&stripped, "dracut"));
        assert!(has_token(&stripped, "btrfs-progs"));
    }

    #[test]
    fn scan_tools_parses_quoted_names_booleans_and_ignores_nested_list_items() {
        let yaml = "\
# comment line
tools:
  - name: \"novos-custom-tools\"
    enabled: true
    binaries:
      - \"/usr/local/bin/x\"
  - name: 'second-tool'
    enabled: false
  - name: default-disabled-tool
";
        let tools = scan_tools(yaml);
        assert_eq!(
            tools,
            vec![
                Tool {
                    name: "novos-custom-tools".to_string(),
                    enabled: true,
                },
                Tool {
                    name: "second-tool".to_string(),
                    enabled: false,
                },
                Tool {
                    name: "default-disabled-tool".to_string(),
                    enabled: false,
                },
            ]
        );
    }

    #[test]
    fn clean_repo_tree_produces_zero_violations() {
        let tmp = tempdir().expect("tempdir");
        write_clean_repo(tmp.path());
        let manifest = sample_manifest();

        let violations = verify_repo_invariants(&manifest, &Approvals::default(), tmp.path())
            .expect("verify_repo_invariants");
        assert!(
            violations.is_empty(),
            "unexpected violations: {violations:?}"
        );
    }

    #[test]
    fn each_repo_violation_type_is_reported_and_skipped_by_open_approval() {
        let manifest = sample_manifest();

        // 1a. repo_custom_tool_missing
        {
            let tmp = tempdir().expect("tempdir");
            write_clean_repo(tmp.path());
            fs::remove_file(tmp.path().join("custom-tools/manifest.yaml")).expect("rm yaml");

            let v = verify_repo_invariants(&manifest, &Approvals::default(), tmp.path())
                .expect("verify");
            assert_eq!(
                v,
                vec![Violation::new(
                    "repo_custom_tool_missing",
                    "novos-custom-tools",
                    "novos-custom-tools missing from custom-tools/manifest.yaml",
                )]
            );
            assert!(verify_repo_invariants(
                &manifest,
                &open_approval("novos-custom-tools"),
                tmp.path()
            )
            .expect("verify approved")
            .is_empty());
        }

        // 1b. repo_custom_tool_disabled
        {
            let tmp = tempdir().expect("tempdir");
            write_clean_repo(tmp.path());
            fs::write(
                tmp.path().join("custom-tools/manifest.yaml"),
                "tools:\n  - name: novos-custom-tools\n    enabled: false\n",
            )
            .expect("disable tool");

            let v = verify_repo_invariants(&manifest, &Approvals::default(), tmp.path())
                .expect("verify");
            assert_eq!(
                v,
                vec![Violation::new(
                    "repo_custom_tool_disabled",
                    "novos-custom-tools",
                    "novos-custom-tools is disabled in custom-tools/manifest.yaml",
                )]
            );
            assert!(verify_repo_invariants(
                &manifest,
                &open_approval("novos-custom-tools"),
                tmp.path()
            )
            .expect("verify approved")
            .is_empty());
        }

        // 2a. repo_calamares_config_missing
        {
            let tmp = tempdir().expect("tempdir");
            write_clean_repo(tmp.path());
            fs::remove_file(tmp.path().join("calamares/modules/fstab.conf")).expect("rm fstab");

            let v = verify_repo_invariants(&manifest, &Approvals::default(), tmp.path())
                .expect("verify");
            assert_eq!(
                v,
                vec![Violation::new(
                    "repo_calamares_config_missing",
                    "fstab.conf",
                    "Required Calamares config calamares/modules/fstab.conf missing",
                )]
            );
            assert!(
                verify_repo_invariants(&manifest, &open_approval("fstab.conf"), tmp.path())
                    .expect("verify approved")
                    .is_empty()
            );
        }

        // 2b. repo_calamares_module_missing
        {
            let tmp = tempdir().expect("tempdir");
            write_clean_repo(tmp.path());
            fs::remove_dir_all(tmp.path().join("calamares/modules/limine-bootloader"))
                .expect("rm module dir");

            let v = verify_repo_invariants(&manifest, &Approvals::default(), tmp.path())
                .expect("verify");
            assert_eq!(
                v,
                vec![Violation::new(
                    "repo_calamares_module_missing",
                    "limine-bootloader",
                    "Required Calamares custom module calamares/modules/limine-bootloader missing",
                )]
            );
            assert!(verify_repo_invariants(
                &manifest,
                &open_approval("limine-bootloader"),
                tmp.path()
            )
            .expect("verify approved")
            .is_empty());
        }

        // 2c. repo_calamares_netinstall_invalid
        {
            let tmp = tempdir().expect("tempdir");
            write_clean_repo(tmp.path());
            fs::write(
                tmp.path().join("calamares/modules/netinstall.yaml"),
                "# - name: commented out\n# packages:\n",
            )
            .expect("write invalid netinstall.yaml");

            let v = verify_repo_invariants(&manifest, &Approvals::default(), tmp.path())
                .expect("verify");
            assert_eq!(
                v,
                vec![Violation::new(
                    "repo_calamares_netinstall_invalid",
                    "netinstall.yaml",
                    "calamares/modules/netinstall.yaml is missing profile groups or package lists",
                )]
            );
            assert!(verify_repo_invariants(
                &manifest,
                &open_approval("netinstall.yaml"),
                tmp.path()
            )
            .expect("verify approved")
            .is_empty());
        }

        // 3a. repo_cargo_member_missing
        {
            let tmp = tempdir().expect("tempdir");
            write_clean_repo(tmp.path());
            fs::remove_file(tmp.path().join("crates/testtool/Cargo.toml"))
                .expect("rm member Cargo.toml");

            let v = verify_repo_invariants(&manifest, &Approvals::default(), tmp.path())
                .expect("verify");
            let expected_detail = format!(
                "Workspace member Cargo.toml missing: {}",
                tmp.path().join("crates/testtool/Cargo.toml").display()
            );
            assert_eq!(
                v,
                vec![Violation::new(
                    "repo_cargo_member_missing",
                    "crates/testtool",
                    expected_detail,
                )]
            );
            assert!(verify_repo_invariants(
                &manifest,
                &open_approval("crates/testtool"),
                tmp.path()
            )
            .expect("verify approved")
            .is_empty());
        }

        // 3b. repo_cargo_version_downgrade
        {
            let tmp = tempdir().expect("tempdir");
            write_clean_repo(tmp.path());
            fs::write(
                tmp.path().join("crates/testtool/Cargo.toml"),
                "[package]\nname = \"testtool\"\nversion = \"0.0.9\"\n",
            )
            .expect("downgrade member Cargo.toml");

            let v = verify_repo_invariants(&manifest, &Approvals::default(), tmp.path())
                .expect("verify");
            assert_eq!(
                v,
                vec![Violation::new(
                    "repo_cargo_version_downgrade",
                    "testtool",
                    "Crate testtool version 0.0.9 < manifest min_version 0.1.0",
                )]
            );
            assert!(
                verify_repo_invariants(&manifest, &open_approval("testtool"), tmp.path())
                    .expect("verify open approved")
                    .is_empty()
            );
            assert!(verify_repo_invariants(
                &manifest,
                &version_approval("testtool", "0.0.9"),
                tmp.path()
            )
            .expect("verify matching version approved")
            .is_empty());
            assert_eq!(
                verify_repo_invariants(
                    &manifest,
                    &version_approval("testtool", "0.0.8"),
                    tmp.path()
                )
                .expect("verify mismatched version approval")
                .len(),
                1
            );
        }

        // 4a. repo_stage_package_missing (core and cosmic)
        {
            let tmp = tempdir().expect("tempdir");
            write_clean_repo(tmp.path());
            fs::write(
                tmp.path().join("scripts/stages/10-packages.sh"),
                "#!/bin/sh\napt-get install -y btrfs-progs cosmic-comp\n/usr/local/bin/testtool --pre-packages\n",
            )
            .expect("omit dracut");

            let v = verify_repo_invariants(&manifest, &Approvals::default(), tmp.path())
                .expect("verify");
            assert_eq!(
                v,
                vec![Violation::new(
                    "repo_stage_package_missing",
                    "dracut",
                    "Core package 'dracut' missing from scripts/stages/*.sh",
                )]
            );
            assert!(
                verify_repo_invariants(&manifest, &open_approval("dracut"), tmp.path())
                    .expect("verify approved")
                    .is_empty()
            );

            fs::write(
                tmp.path().join("scripts/stages/10-packages.sh"),
                "#!/bin/sh\napt-get install -y dracut btrfs-progs\n/usr/local/bin/testtool --pre-packages\n",
            )
            .expect("omit cosmic-comp");

            let v_cosmic = verify_repo_invariants(&manifest, &Approvals::default(), tmp.path())
                .expect("verify cosmic");
            assert_eq!(
                v_cosmic,
                vec![Violation::new(
                    "repo_stage_package_missing",
                    "cosmic-comp",
                    "COSMIC package 'cosmic-comp' missing from scripts/stages/*.sh",
                )]
            );
            assert!(
                verify_repo_invariants(&manifest, &open_approval("cosmic-comp"), tmp.path())
                    .expect("verify cosmic approved")
                    .is_empty()
            );
        }

        // 4b. repo_stage_binary_missing
        {
            let tmp = tempdir().expect("tempdir");
            write_clean_repo(tmp.path());
            fs::write(
                tmp.path().join("scripts/stages/10-packages.sh"),
                "#!/bin/sh\napt-get install -y dracut btrfs-progs cosmic-comp\n# --pre-packages only in flag line:\necho --pre-packages\n",
            )
            .expect("omit testtool binary");

            let v = verify_repo_invariants(&manifest, &Approvals::default(), tmp.path())
                .expect("verify");
            assert_eq!(
                v,
                vec![Violation::new(
                    "repo_stage_binary_missing",
                    "/usr/local/bin/testtool",
                    "Custom binary '/usr/local/bin/testtool' missing from scripts/stages/*.sh",
                )]
            );
            assert!(verify_repo_invariants(
                &manifest,
                &open_approval("/usr/local/bin/testtool"),
                tmp.path()
            )
            .expect("verify approved by path")
            .is_empty());
            assert!(
                verify_repo_invariants(&manifest, &open_approval("testtool"), tmp.path())
                    .expect("verify approved by basename")
                    .is_empty()
            );
        }

        // 4c. repo_stage_flag_missing
        {
            let tmp = tempdir().expect("tempdir");
            write_clean_repo(tmp.path());
            fs::write(
                tmp.path().join("scripts/stages/10-packages.sh"),
                "#!/bin/sh\napt-get install -y dracut btrfs-progs cosmic-comp\n/usr/local/bin/testtool\n",
            )
            .expect("omit --pre-packages flag");

            let v = verify_repo_invariants(&manifest, &Approvals::default(), tmp.path())
                .expect("verify");
            assert_eq!(
                v,
                vec![Violation::new(
                    "repo_stage_flag_missing",
                    "/usr/local/bin/testtool:--pre-packages",
                    "Required flag '--pre-packages' for '/usr/local/bin/testtool' missing from stage scripts",
                )]
            );
            assert!(verify_repo_invariants(
                &manifest,
                &open_approval("/usr/local/bin/testtool"),
                tmp.path()
            )
            .expect("verify approved by path")
            .is_empty());
            assert!(verify_repo_invariants(
                &manifest,
                &open_approval("/usr/local/bin/testtool:--pre-packages"),
                tmp.path()
            )
            .expect("verify approved by item")
            .is_empty());
        }
    }

    #[test]
    fn cosmic_package_passes_when_cosmic_epoch_monorepo_token_is_present() {
        let tmp = tempdir().expect("tempdir");
        write_clean_repo(tmp.path());
        let manifest = sample_manifest();

        // Remove cosmic-comp from 10-packages.sh, but mention cosmic-epoch-monorepo in custom-tools/manifest.yaml.
        fs::write(
            tmp.path().join("scripts/stages/10-packages.sh"),
            "#!/bin/sh\napt-get install -y dracut btrfs-progs\n/usr/local/bin/testtool --pre-packages\n",
        )
        .expect("write stage without cosmic-comp");
        fs::write(
            tmp.path().join("custom-tools/manifest.yaml"),
            "tools:\n  - name: \"novos-custom-tools\"\n    enabled: true\n  - name: \"cosmic-epoch-monorepo\"\n    enabled: true\n",
        )
        .expect("write custom-tools with cosmic-epoch-monorepo");

        let violations = verify_repo_invariants(&manifest, &Approvals::default(), tmp.path())
            .expect("verify_repo_invariants");
        assert!(
            violations.is_empty(),
            "unexpected violations: {violations:?}"
        );
    }
}
