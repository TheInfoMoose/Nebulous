use cosmic::iced::{Alignment, Background, Border, Length};
use cosmic::widget;
use cosmic::Element;

use crate::app::{Message, NebulousApp, ViewMode};
use backend_core::GitSyncMode;

/// Canonical section titles rendered by the `Git & Settings` view.
pub const SETTINGS_SECTION_TITLES: [&str; 5] = [
    "1. Multi-Project Registry",
    "2. Git Operations & Documented Commit Workflow",
    "3. Commit Documentation Policy & Remote Config",
    "4. Cross-Platform SecureStorage Credentials (OS Keyring + AES-256-GCM Vault)",
    "5. Metrics & Stagnation Thresholds",
];

/// Renders the dedicated `Git & Settings` view (`ViewMode::Settings`).
///
/// Sections:
/// 1. Multi-Project Registry (list registered projects across the system, switch active project,
///    add a new project directory with optional initial ingestion, or remove from registry).
/// 2. Git Operations & Documented Commit Workflow (`Fetch`, `Pull`, `Push`, `Sync All`, and
///    Documented Commit Builder with Item ID, `.tracker/` only vs. Workspace + `.tracker/`, and
///    summary validated against `[git.commit_policy]`).
/// 3. Commit Documentation Policy & Remote Config (`require_item_id`, `require_scope`,
///    `auto_append_to_desc`, `min_summary_length`, remote/branch, and `manual` vs. `scheduled` sync).
/// 4. Cross-Platform `SecureStorage` Credentials (HTTPS PATs and SSH Ed25519 key generation/import
///    stored in `SecureStorage` — OS Keyring + AES-256-GCM Vault).
/// 5. Metrics & Stagnation Thresholds (`target_daily_hours`, `stagnant_item_threshold_days`, `default_scope`).
pub fn view_settings(app: &NebulousApp) -> Element<'_, Message> {
    let header_row = widget::row::with_capacity(3)
        .push(widget::text::title4(
            "Git Sync, Multi-Project Registry, Commit Policy & SecureStorage Settings",
        ))
        .push(widget::space::horizontal().width(Length::Fill))
        .push(
            widget::button::standard("← Back to Kanban Board")
                .height(Length::Fixed(30.0))
                .on_press(Message::SwitchView(ViewMode::Board)),
        )
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let section_1 = view_multi_project_registry_section(app);
    let section_2 = view_git_operations_and_documented_commit_section(app);
    let section_3 = view_commit_policy_and_remote_config_section(app);
    let section_4 = view_secure_storage_credentials_section(app);
    let section_5 = view_metrics_and_stagnation_thresholds_section(app);

    let content = widget::column::with_capacity(11)
        .push(header_row)
        .push(section_1)
        .push(widget::divider::horizontal::default())
        .push(section_2)
        .push(widget::divider::horizontal::default())
        .push(section_3)
        .push(widget::divider::horizontal::default())
        .push(section_4)
        .push(widget::divider::horizontal::default())
        .push(section_5)
        .spacing(14)
        .width(Length::Fill);

    widget::container(widget::scrollable(content).height(Length::Fill))
        .padding([14, 16])
        .width(Length::Fill)
        .height(Length::Fill)
        .class(cosmic::theme::Container::custom(|theme| {
            let cosmic = theme.cosmic();
            cosmic::iced::widget::container::Style {
                background: Some(Background::Color(
                    cosmic.primary(theme.transparent).base.into(),
                )),
                border: Border {
                    color: cosmic.bg_divider().into(),
                    width: 1.0,
                    radius: cosmic.corner_radii.radius_s.into(),
                },
                ..Default::default()
            }
        }))
        .into()
}

fn view_multi_project_registry_section(app: &NebulousApp) -> Element<'_, Message> {
    let current_root_str = app.storage.project_root().to_string_lossy().replace('\\', "/");
    let active_root_caption = if app.storage.project_toml_path().is_file() {
        format!(
            "Active Project: {} · Active Project Root: {current_root_str}",
            app.project_name()
        )
    } else {
        "Active Project Root: (none — select or register a project below)".to_string()
    };
    let mut projects_col = widget::column::with_capacity(app.project_registry.projects.len() + 4)
        .push(widget::text::heading("1. Multi-Project Registry"))
        .push(widget::text::caption(active_root_caption))
        .spacing(6)
        .width(Length::Fill);

    for proj in &app.project_registry.projects {
        let is_active = match app.project_registry.active_project_id.as_deref() {
            Some(active_id) => proj.id == active_id,
            None => proj.root_path == current_root_str,
        };

        let phase_tag = proj.active_phase.as_deref().unwrap_or("default");
        let row = widget::row::with_capacity(6)
            .push(widget::text::monotext(proj.id.clone()))
            .push(widget::text::body(proj.name.clone()))
            .push(widget::space::horizontal().width(Length::Fill))
            .push(widget::text::caption(format!(
                "{} · branch: {} · phase: {}",
                proj.root_path, proj.default_branch, phase_tag
            )))
            .push(if is_active {
                widget::button::suggested("Active Project")
                    .padding([2, 10])
                    .height(Length::Fixed(26.0))
                    .on_press(Message::SwitchProject(proj.id.clone()))
            } else {
                widget::button::standard("Switch")
                    .padding([2, 10])
                    .height(Length::Fixed(26.0))
                    .on_press(Message::SwitchProject(proj.id.clone()))
            })
            .push(
                widget::button::destructive("Remove")
                    .padding([2, 10])
                    .height(Length::Fixed(26.0))
                    .on_press(Message::RemoveProjectFromRegistry(proj.id.clone())),
            )
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Length::Fill);

        projects_col = projects_col.push(
            widget::container(row)
                .padding([8, 10])
                .width(Length::Fill)
                .class(cosmic::theme::Container::Secondary),
        );
    }

    let ingest_toggle_btn = if app.settings_form.new_project_ingest_on_add {
        widget::button::suggested("✓ Run Initial Ingestion")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::ToggleNewProjectIngest(false))
    } else {
        widget::button::standard("Run Initial Ingestion")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::ToggleNewProjectIngest(true))
    };

    let add_project_form = widget::container(
        widget::column::with_capacity(2)
            .push(widget::text::caption_heading(
                "REGISTER & INITIALIZE NEW PROJECT DIRECTORY",
            ))
            .push(
                widget::row::with_capacity(4)
                    .push(
                        widget::text_input(
                            "Project Root Path (e.g. /home/user/Projects/NovOS)",
                            &app.settings_form.new_project_path_input,
                        )
                        .on_input(Message::NewProjectPathChanged)
                        .width(Length::FillPortion(2)),
                    )
                    .push(
                        widget::text_input(
                            "Optional Project Name (e.g. NovOS)",
                            &app.settings_form.new_project_name_input,
                        )
                        .on_input(Message::NewProjectNameChanged)
                        .on_submit(|_| Message::AddProjectToRegistry)
                        .width(Length::FillPortion(1)),
                    )
                    .push(ingest_toggle_btn)
                    .push(
                        widget::button::suggested("+ Add & Switch Project")
                            .padding([4, 12])
                            .height(Length::Fixed(28.0))
                            .on_press(Message::AddProjectToRegistry),
                    )
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .width(Length::Fill),
            )
            .spacing(6)
            .width(Length::Fill),
    )
    .padding([10, 12])
    .width(Length::Fill)
    .class(cosmic::theme::Container::Secondary);

    projects_col.push(add_project_form).into()
}

fn view_git_operations_and_documented_commit_section(app: &NebulousApp) -> Element<'_, Message> {
    let sha = app
        .git_status
        .as_ref()
        .and_then(|s| s.head_short_sha.as_deref())
        .unwrap_or("uncommitted");
    let branch = app
        .git_status
        .as_ref()
        .and_then(|s| s.branch.as_deref())
        .unwrap_or(&app.index.project.git.branch);
    let (ahead, behind) = app
        .git_status
        .as_ref()
        .map(|s| (s.ahead_count, s.behind_count))
        .unwrap_or((0, 0));
    let dirty_tracker_count = app
        .git_status
        .as_ref()
        .map(|s| s.dirty_tracker_files.len())
        .unwrap_or(0);
    let dirty_workspace_count = app
        .git_status
        .as_ref()
        .map(|s| s.dirty_workspace_files.len())
        .unwrap_or(0);

    let git_ops_row = widget::row::with_capacity(6)
        .push(
            widget::button::standard("↓ Fetch")
                .height(Length::Fixed(30.0))
                .on_press(Message::GitFetch),
        )
        .push(
            widget::button::standard("⇊ Pull")
                .height(Length::Fixed(30.0))
                .on_press(Message::GitPull),
        )
        .push(
            widget::button::standard("⇈ Push")
                .height(Length::Fixed(30.0))
                .on_press(Message::GitPush),
        )
        .push(
            widget::button::suggested("⇅ Sync All")
                .height(Length::Fixed(30.0))
                .on_press(Message::TriggerGitSync),
        )
        .push(
            widget::button::standard("✓ Commit .tracker/ Only")
                .height(Length::Fixed(30.0))
                .on_press(Message::CommitTrackerNow),
        )
        .push(
            widget::button::standard("↻ Reconcile Indexes")
                .height(Length::Fixed(30.0))
                .on_press(Message::ReconcileIndexes),
        )
        .spacing(8)
        .align_y(Alignment::Center);

    let tracker_only_btn = if app.settings_form.commit_tracker_only {
        widget::button::suggested("Stage: .tracker/ Only")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::SetCommitTrackerOnly(true))
    } else {
        widget::button::standard("Stage: .tracker/ Only")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::SetCommitTrackerOnly(true))
    };

    let workspace_plus_tracker_btn = if !app.settings_form.commit_tracker_only {
        widget::button::suggested("Stage: Workspace + .tracker/")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::SetCommitTrackerOnly(false))
    } else {
        widget::button::standard("Stage: Workspace + .tracker/")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::SetCommitTrackerOnly(false))
    };

    let mut item_quick_pick_row = widget::row::with_capacity(6)
        .push(widget::text::caption("Quick-select Item:"))
        .spacing(6)
        .align_y(Alignment::Center);

    for card in app.active_phase_cards().into_iter().take(5) {
        let is_chosen = app.settings_form.commit_item_id_input.trim() == card.id;
        let btn = if is_chosen {
            widget::button::suggested(card.id.clone())
                .padding([2, 8])
                .height(Length::Fixed(24.0))
                .on_press(Message::CommitItemIdChanged(card.id))
        } else {
            widget::button::standard(card.id.clone())
                .padding([2, 8])
                .height(Length::Fixed(24.0))
                .on_press(Message::CommitItemIdChanged(card.id))
        };
        item_quick_pick_row = item_quick_pick_row.push(btn);
    }

    let min_len = app.index.project.git.commit_policy.min_summary_length;

    let commit_builder_top = widget::row::with_capacity(4)
        .push(
            widget::text_input(
                "Item ID (e.g. BUG-0001 / FEAT-0001)",
                &app.settings_form.commit_item_id_input,
            )
            .on_input(Message::CommitItemIdChanged)
            .width(Length::FillPortion(1)),
        )
        .push(
            widget::text_input(
                "Optional Scope Override (auto-resolved from item)",
                &app.settings_form.commit_scope_id_input,
            )
            .on_input(Message::CommitScopeIdChanged)
            .width(Length::FillPortion(1)),
        )
        .push(tracker_only_btn)
        .push(workspace_plus_tracker_btn)
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let commit_builder_bottom = widget::row::with_capacity(3)
        .push(
            widget::text_input(
                "Documented commit summary (validated against [git.commit_policy])...",
                &app.settings_form.commit_summary_input,
            )
            .on_input(Message::CommitSummaryChanged)
            .on_submit(|_| Message::SubmitDocumentedCommit)
            .width(Length::FillPortion(2)),
        )
        .push(
            widget::text_input(
                "Optional extended commit notes / agent fix log...",
                &app.settings_form.commit_body_input,
            )
            .on_input(Message::CommitBodyChanged)
            .width(Length::FillPortion(1)),
        )
        .push(
            widget::button::suggested("✓ Commit Documented")
                .padding([4, 14])
                .height(Length::Fixed(28.0))
                .on_press(Message::SubmitDocumentedCommit),
        )
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    widget::container(
        widget::column::with_capacity(6)
            .push(widget::text::heading(
                "2. Git Operations & Documented Commit Workflow",
            ))
            .push(widget::text::caption(format!(
                "Workspace: {}  ·  Branch: {branch} @ {sha}  ·  ↑{ahead} ↓{behind}  ·  Dirty .tracker/: {dirty_tracker_count}  ·  Dirty workspace: {dirty_workspace_count}  ·  Policy min summary: {min_len} chars",
                app.storage.project_root().display()
            )))
            .push(git_ops_row)
            .push(item_quick_pick_row)
            .push(commit_builder_top)
            .push(commit_builder_bottom)
            .spacing(8)
            .width(Length::Fill),
    )
    .padding([10, 12])
    .width(Length::Fill)
    .class(cosmic::theme::Container::Secondary)
    .into()
}

fn view_commit_policy_and_remote_config_section(app: &NebulousApp) -> Element<'_, Message> {
    let req_item_btn = if app.settings_form.commit_policy_require_item_id {
        widget::button::suggested("✓ require_item_id")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::ToggleCommitPolicyRequireItemId(false))
    } else {
        widget::button::standard("require_item_id")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::ToggleCommitPolicyRequireItemId(true))
    };

    let req_scope_btn = if app.settings_form.commit_policy_require_scope {
        widget::button::suggested("✓ require_scope")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::ToggleCommitPolicyRequireScope(false))
    } else {
        widget::button::standard("require_scope")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::ToggleCommitPolicyRequireScope(true))
    };

    let auto_append_btn = if app.settings_form.commit_policy_auto_append_to_desc {
        widget::button::suggested("✓ auto_append_to_desc")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::ToggleCommitPolicyAutoAppendDesc(false))
    } else {
        widget::button::standard("auto_append_to_desc")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::ToggleCommitPolicyAutoAppendDesc(true))
    };

    let policy_row = widget::row::with_capacity(6)
        .push(req_item_btn)
        .push(req_scope_btn)
        .push(auto_append_btn)
        .push(widget::text::caption("Min Summary Chars:"))
        .push(
            widget::text_input(
                "12",
                &app.settings_form.commit_policy_min_summary_len_input,
            )
            .on_input(Message::CommitPolicyMinSummaryLengthChanged)
            .width(Length::Fixed(70.0)),
        )
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let manual_mode_btn = if app.settings_form.git_sync_mode_input == GitSyncMode::Manual {
        widget::button::suggested("Manual Sync")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::SetGitSyncMode(GitSyncMode::Manual))
    } else {
        widget::button::standard("Manual Sync")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::SetGitSyncMode(GitSyncMode::Manual))
    };

    let scheduled_mode_btn = if app.settings_form.git_sync_mode_input == GitSyncMode::Scheduled {
        widget::button::suggested("Scheduled Sync")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::SetGitSyncMode(GitSyncMode::Scheduled))
    } else {
        widget::button::standard("Scheduled Sync")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::SetGitSyncMode(GitSyncMode::Scheduled))
    };

    let remote_row = widget::row::with_capacity(9)
        .push(manual_mode_btn)
        .push(scheduled_mode_btn)
        .push(widget::text::caption("Interval (m):"))
        .push(
            widget::text_input("30", &app.settings_form.git_sync_interval_input)
                .on_input(Message::GitSyncIntervalChanged)
                .width(Length::Fixed(70.0)),
        )
        .push(widget::text::caption("Remote:"))
        .push(
            widget::text_input("origin", &app.settings_form.git_remote_input)
                .on_input(Message::GitRemoteChanged)
                .width(Length::Fixed(100.0)),
        )
        .push(widget::text::caption("Branch:"))
        .push(
            widget::text_input("main", &app.settings_form.git_branch_input)
                .on_input(Message::GitBranchChanged)
                .width(Length::Fixed(100.0)),
        )
        .push(
            widget::button::suggested("💾 Save Git & Policy Settings")
                .padding([4, 12])
                .height(Length::Fixed(28.0))
                .on_press(Message::SaveGitConfig),
        )
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    widget::container(
        widget::column::with_capacity(3)
            .push(widget::text::heading(
                "3. Commit Documentation Policy & Remote Config",
            ))
            .push(policy_row)
            .push(remote_row)
            .spacing(8)
            .width(Length::Fill),
    )
    .padding([10, 12])
    .width(Length::Fill)
    .class(cosmic::theme::Container::Secondary)
    .into()
}

fn view_secure_storage_credentials_section(app: &NebulousApp) -> Element<'_, Message> {
    let pat_status_text = if app.settings_form.has_stored_pat {
        "Stored in SecureStorage (OS Keyring + AES-256-GCM Vault)"
    } else {
        "Not stored"
    };

    let pat_keyring_row = widget::row::with_capacity(4)
        .push(widget::text::caption(format!(
            "HTTPS PAT ({pat_status_text}):"
        )))
        .push(
            widget::text_input(
                "Enter HTTPS Personal Access Token (PAT)...",
                &app.settings_form.pat_input,
            )
            .on_input(Message::PatInputChanged)
            .on_submit(|_| Message::SavePatToKeyring)
            .width(Length::Fill),
        )
        .push(
            widget::button::suggested("🔑 Save PAT to Keyring")
                .padding([4, 10])
                .height(Length::Fixed(28.0))
                .on_press(Message::SavePatToKeyring),
        )
        .push(
            widget::button::destructive("🗑 Clear PAT")
                .padding([4, 10])
                .height(Length::Fixed(28.0))
                .on_press(Message::DeletePatFromKeyring),
        )
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let ssh_status_caption = match app.settings_form.stored_ssh_public_key.as_ref() {
        Some(pub_info) => format!(
            "SSH Ed25519 Key Stored ({}) · Fingerprint: {} · Public Key: {}",
            pub_info.algorithm, pub_info.fingerprint_sha256, pub_info.public_key_openssh
        ),
        None => "SSH Keypair: No SSH key stored in SecureStorage for this project".to_string(),
    };

    let ssh_generate_row = widget::row::with_capacity(5)
        .push(
            widget::text_input(
                "SSH Key Comment (e.g. developer@nebulous.dev)",
                &app.settings_form.ssh_comment_input,
            )
            .on_input(Message::SshKeyCommentChanged)
            .width(Length::Fill),
        )
        .push(
            widget::button::suggested("⚡ Generate Ed25519 Keypair")
                .padding([4, 12])
                .height(Length::Fixed(28.0))
                .on_press(Message::GenerateSshKeypair),
        )
        .push(
            widget::button::standard("📋 Copy Public Key")
                .padding([4, 10])
                .height(Length::Fixed(28.0))
                .on_press(Message::CopySshPublicKey),
        )
        .push(
            widget::button::destructive("🗑 Delete SSH Key")
                .padding([4, 10])
                .height(Length::Fixed(28.0))
                .on_press(Message::DeleteSshKeypair),
        )
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let ssh_import_row = widget::row::with_capacity(3)
        .push(
            widget::text_input(
                "Import OpenSSH Private Key (file path e.g. ~/.ssh/id_ed25519 or inline PEM)...",
                &app.settings_form.ssh_private_key_import_input,
            )
            .on_input(Message::SshKeyImportInputChanged)
            .width(Length::FillPortion(2)),
        )
        .push(
            widget::text_input(
                "Optional passphrase...",
                &app.settings_form.ssh_passphrase_input,
            )
            .on_input(Message::SshKeyPassphraseChanged)
            .on_submit(|_| Message::ImportSshKeypair)
            .width(Length::FillPortion(1)),
        )
        .push(
            widget::button::standard("📥 Import SSH Key")
                .padding([4, 12])
                .height(Length::Fixed(28.0))
                .on_press(Message::ImportSshKeypair),
        )
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    widget::container(
        widget::column::with_capacity(5)
            .push(widget::text::heading(
                "4. Cross-Platform SecureStorage Credentials (OS Keyring + AES-256-GCM Vault)",
            ))
            .push(pat_keyring_row)
            .push(widget::text::caption(ssh_status_caption))
            .push(ssh_generate_row)
            .push(ssh_import_row)
            .spacing(8)
            .width(Length::Fill),
    )
    .padding([10, 12])
    .width(Length::Fill)
    .class(cosmic::theme::Container::Secondary)
    .into()
}

fn view_metrics_and_stagnation_thresholds_section(app: &NebulousApp) -> Element<'_, Message> {
    widget::container(
        widget::column::with_capacity(2)
            .push(widget::text::heading("5. Metrics & Stagnation Thresholds"))
            .push(
                widget::row::with_capacity(7)
                    .push(widget::text::caption("Target Daily Hours:"))
                    .push(
                        widget::text_input("6.0", &app.settings_form.target_daily_hours_input)
                            .on_input(Message::TargetDailyHoursChanged)
                            .width(Length::Fixed(90.0)),
                    )
                    .push(widget::text::caption("Stagnant Threshold (days):"))
                    .push(
                        widget::text_input("4", &app.settings_form.stagnant_threshold_days_input)
                            .on_input(Message::StagnantThresholdDaysChanged)
                            .width(Length::Fixed(80.0)),
                    )
                    .push(widget::text::caption("Default Scope:"))
                    .push(
                        widget::text_input("inbox", &app.settings_form.default_scope_input)
                            .on_input(Message::DefaultScopeChanged)
                            .width(Length::Fixed(140.0)),
                    )
                    .push(
                        widget::button::suggested("💾 Save Metrics & Defaults")
                            .padding([4, 12])
                            .height(Length::Fixed(28.0))
                            .on_press(Message::SaveProjectAndMetricsConfig),
                    )
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .width(Length::Fill),
            )
            .spacing(8)
            .width(Length::Fill),
    )
    .padding([10, 12])
    .width(Length::Fill)
    .class(cosmic::theme::Container::Secondary)
    .into()
}
