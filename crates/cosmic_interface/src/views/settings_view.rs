use cosmic::iced::{Alignment, Background, Border, Length};
use cosmic::widget;
use cosmic::Element;

use crate::app::{Message, NebulousApp, ViewMode};
use backend_core::GitSyncMode;

/// Renders the Phases & Domain Scopes Manager and Git/Keyring/Metrics Settings view.
pub fn view_settings_or_phases(app: &NebulousApp) -> Element<'_, Message> {
    let title = match app.view_mode {
        ViewMode::PhasesAndScopes => "Phases & Domain Scopes Manager",
        _ => "Project Settings, Metrics Thresholds & Git/Keyring Sync",
    };

    let header_row = widget::row::with_capacity(3)
        .push(widget::text::title4(title))
        .push(widget::space::horizontal().width(Length::Fill))
        .push(
            widget::button::standard("← Back to Kanban Board")
                .height(Length::Fixed(30.0))
                .on_press(Message::SwitchView(ViewMode::Board)),
        )
        .align_y(Alignment::Center)
        .width(Length::Fill);

    // 1. Registered Phases & Create New Phase Form
    let mut phases_col = widget::column::with_capacity(app.index.phases.len() + 3)
        .push(widget::text::heading("1. Registered Phases"))
        .spacing(6)
        .width(Length::Fill);

    for (idx, phase) in app.index.phases.iter().enumerate() {
        let is_active = phase.phase_id == app.active_phase_id;
        let row = widget::row::with_capacity(5)
            .push(widget::text::monotext(phase.phase_id.clone()))
            .push(widget::text::body(phase.manifest.phase.title.clone()))
            .push(widget::space::horizontal().width(Length::Fill))
            .push(widget::text::caption(format!(
                "[{}] {}/{} done · {} scopes",
                phase.manifest.phase.status,
                phase.manifest.summary.completed_items,
                phase.manifest.summary.total_items,
                phase.scopes.len()
            )))
            .push(if is_active {
                widget::button::suggested("Active")
                    .padding([2, 10])
                    .height(Length::Fixed(26.0))
                    .on_press(Message::SelectPhaseIndex(idx))
            } else {
                widget::button::standard("Activate")
                    .padding([2, 10])
                    .height(Length::Fixed(26.0))
                    .on_press(Message::SelectPhaseIndex(idx))
            })
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Length::Fill);

        phases_col = phases_col.push(
            widget::container(row)
                .padding([8, 10])
                .width(Length::Fill)
                .class(cosmic::theme::Container::Secondary),
        );
    }

    let set_active_btn = if app.settings_form.new_phase_set_active {
        widget::button::suggested("✓ Set Active on Create")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::ToggleNewPhaseSetActive(false))
    } else {
        widget::button::standard("Set Active on Create")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::ToggleNewPhaseSetActive(true))
    };

    let create_phase_form = widget::container(
        widget::column::with_capacity(2)
            .push(widget::text::caption_heading("CREATE NEW PHASE"))
            .push(
                widget::row::with_capacity(4)
                    .push(
                        widget::text_input(
                            "Phase ID (e.g. 03_release_prep)",
                            &app.settings_form.new_phase_id,
                        )
                        .on_input(Message::NewPhaseIdChanged)
                        .width(Length::FillPortion(1)),
                    )
                    .push(
                        widget::text_input(
                            "Phase Title (e.g. Release & Packaging)",
                            &app.settings_form.new_phase_title,
                        )
                        .on_input(Message::NewPhaseTitleChanged)
                        .on_submit(|_| Message::CreatePhase)
                        .width(Length::FillPortion(2)),
                    )
                    .push(set_active_btn)
                    .push(
                        widget::button::suggested("+ Create Phase")
                            .padding([4, 12])
                            .height(Length::Fixed(28.0))
                            .on_press(Message::CreatePhase),
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

    phases_col = phases_col.push(create_phase_form);

    // 2. Domain Scopes in Active Phase & Create New Scope Form
    let mut scopes_col = widget::column::with_capacity(10)
        .push(widget::text::heading(format!(
            "2. Domain Scopes in Active Phase ('{}')",
            app.active_phase_id
        )))
        .spacing(6)
        .width(Length::Fill);

    if let Some(active_phase) = app.active_phase() {
        for scope in &active_phase.scopes {
            let stack = if scope.manifest.scope.tech_stack.is_empty() {
                "none".to_string()
            } else {
                scope.manifest.scope.tech_stack.join(", ")
            };
            let row = widget::row::with_capacity(4)
                .push(widget::text::monotext(scope.scope_id.clone()))
                .push(widget::text::body(format!(
                    "target: {}",
                    scope.manifest.scope.target
                )))
                .push(widget::space::horizontal().width(Length::Fill))
                .push(widget::text::caption(format!(
                    "stack: [{stack}] · {} items",
                    scope.manifest.total_items()
                )))
                .push(
                    widget::button::standard("Filter Board")
                        .padding([2, 10])
                        .height(Length::Fixed(26.0))
                        .on_press(Message::SelectScopeAndOpenBoard(scope.scope_id.clone())),
                )
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill);

            scopes_col = scopes_col.push(
                widget::container(row)
                    .padding([8, 10])
                    .width(Length::Fill)
                    .class(cosmic::theme::Container::Secondary),
            );
        }
    }

    let create_scope_form = widget::container(
        widget::column::with_capacity(3)
            .push(widget::text::caption_heading(
                "CREATE NEW DOMAIN SCOPE IN ACTIVE PHASE",
            ))
            .push(
                widget::row::with_capacity(3)
                    .push(
                        widget::text_input(
                            "Scope ID (e.g. telemetry_core)",
                            &app.settings_form.new_scope_id,
                        )
                        .on_input(Message::NewScopeIdChanged)
                        .width(Length::FillPortion(1)),
                    )
                    .push(
                        widget::text_input(
                            "Target Path (e.g. crates/telemetry_core)",
                            &app.settings_form.new_scope_target,
                        )
                        .on_input(Message::NewScopeTargetChanged)
                        .width(Length::FillPortion(1)),
                    )
                    .push(
                        widget::text_input(
                            "Tech Stack (comma-separated: rust, serde)",
                            &app.settings_form.new_scope_tech_stack,
                        )
                        .on_input(Message::NewScopeTechStackChanged)
                        .width(Length::FillPortion(1)),
                    )
                    .spacing(8)
                    .width(Length::Fill),
            )
            .push(
                widget::row::with_capacity(2)
                    .push(
                        widget::text_input(
                            "Optional scope description...",
                            &app.settings_form.new_scope_description,
                        )
                        .on_input(Message::NewScopeDescriptionChanged)
                        .on_submit(|_| Message::CreateScope)
                        .width(Length::Fill),
                    )
                    .push(
                        widget::button::suggested("+ Create Scope")
                            .padding([4, 12])
                            .height(Length::Fixed(28.0))
                            .on_press(Message::CreateScope),
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

    scopes_col = scopes_col.push(create_scope_form);

    // 3. Metrics Thresholds & Default Scope Configuration (`project.toml`)
    let metrics_config_card = widget::container(
        widget::column::with_capacity(3)
            .push(widget::text::heading(
                "3. Metrics Thresholds & Default Triage Scope",
            ))
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
    .class(cosmic::theme::Container::Secondary);

    // 4. OS Keyring PAT Management & Manual / Scheduled Git Sync Settings
    let sha = app
        .git_status
        .as_ref()
        .and_then(|s| s.head_short_sha.as_deref())
        .unwrap_or("uncommitted");
    let dirty_count = app
        .git_status
        .as_ref()
        .map(|s| s.dirty_tracker_files.len())
        .unwrap_or(0);
    let pat_status_text = if app.settings_form.has_stored_pat {
        "Stored in OS Keyring (nebulous-tracker)"
    } else {
        "Not stored in OS Keyring"
    };

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

    let git_sync_config_row = widget::row::with_capacity(8)
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
            widget::button::suggested("💾 Save Git Settings")
                .padding([4, 12])
                .height(Length::Fixed(28.0))
                .on_press(Message::SaveGitConfig),
        )
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let pat_keyring_row = widget::row::with_capacity(4)
        .push(widget::text::caption(format!(
            "OS Keyring PAT ({pat_status_text}):"
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

    let git_actions_row = widget::row::with_capacity(4)
        .push(
            widget::button::suggested("⇅ Sync .tracker/ Now")
                .height(Length::Fixed(30.0))
                .on_press(Message::TriggerGitSync),
        )
        .push(
            widget::button::standard("✓ Commit .tracker/ Changes")
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

    let git_card = widget::container(
        widget::column::with_capacity(5)
            .push(widget::text::heading(
                "4. OS Keyring PAT & Git Sync Configuration",
            ))
            .push(widget::text::caption(format!(
                "Workspace: {}  ·  HEAD: {sha}  ·  Dirty .tracker/ files: {dirty_count}",
                app.storage.project_root().display()
            )))
            .push(git_sync_config_row)
            .push(pat_keyring_row)
            .push(git_actions_row)
            .spacing(10)
            .width(Length::Fill),
    )
    .padding([10, 12])
    .width(Length::Fill)
    .class(cosmic::theme::Container::Secondary);

    let content = widget::column::with_capacity(8)
        .push(header_row)
        .push(phases_col)
        .push(widget::divider::horizontal::default())
        .push(scopes_col)
        .push(widget::divider::horizontal::default())
        .push(metrics_config_card)
        .push(widget::divider::horizontal::default())
        .push(git_card)
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
