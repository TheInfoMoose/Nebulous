use cosmic::iced::{Alignment, Background, Border, Length};
use cosmic::widget;
use cosmic::Element;

use crate::app::{Message, NebulousApp, ViewMode};

/// Canonical section titles rendered by the `Phases & Scopes` view.
pub const PHASES_SCOPES_SECTION_TITLES: [&str; 4] = [
    "1. Project Directory Ingestion & Component Catalog",
    "2. Component Manifest & Rolling-Release Pin Builder",
    "3. Registered Phases",
    "4. Domain Scopes in Active Phase",
];

/// Renders the dedicated `Phases & Scopes` view (`ViewMode::PhasesAndScopes`).
///
/// Sections:
/// 1. Project Directory Ingestion & Component Catalog (`Scan Project Directory`, discovered `[components]`,
///    1-click `Promote to Scope` and `Write/Update nebulous.component.toml`).
/// 2. Component Manifest & Rolling-Release Pin Builder (tags, flags, dependencies, `Pin Known-Good Release`,
///    and `Rollback to Pin`).
/// 3. Registered Phases Manager (list phases, activate phase, create new phase).
/// 4. Domain Scopes in Active Phase (list scopes with bound component & tracked file count, filter board,
///    and create custom scope form).
pub fn view_phases_and_scopes(app: &NebulousApp) -> Element<'_, Message> {
    let header_row = widget::row::with_capacity(3)
        .push(widget::text::title4(
            "Phases, Domain Scopes & Directory Ingestion Manager",
        ))
        .push(widget::space::horizontal().width(Length::Fill))
        .push(
            widget::button::standard("← Back to Kanban Board")
                .height(Length::Fixed(30.0))
                .on_press(Message::SwitchView(ViewMode::Board)),
        )
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let section_1 = view_ingestion_and_component_catalog(app);
    let section_2 = view_component_pin_builder(app);
    let section_3 = view_registered_phases_manager(app);
    let section_4 = view_domain_scopes_manager(app);

    let content = widget::column::with_capacity(9)
        .push(header_row)
        .push(section_1)
        .push(widget::divider::horizontal::default())
        .push(section_2)
        .push(widget::divider::horizontal::default())
        .push(section_3)
        .push(widget::divider::horizontal::default())
        .push(section_4)
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

fn view_ingestion_and_component_catalog(app: &NebulousApp) -> Element<'_, Message> {
    let write_manifests_btn = if app.settings_form.ingest_write_manifests {
        widget::button::suggested("✓ Write nebulous.component.toml")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::ToggleIngestWriteManifests(false))
    } else {
        widget::button::standard("Write nebulous.component.toml")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::ToggleIngestWriteManifests(true))
    };

    let promote_all_btn = if app.settings_form.ingest_promote_all {
        widget::button::suggested("✓ Promote All to Scopes")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::ToggleIngestPromoteAll(false))
    } else {
        widget::button::standard("Promote All to Scopes")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::ToggleIngestPromoteAll(true))
    };

    let scan_controls = widget::row::with_capacity(5)
        .push(
            widget::text_input(
                "Optional subpath within active project (leave empty for active project root)...",
                &app.settings_form.ingest_subpath_input,
            )
            .on_input(Message::IngestSubpathChanged)
            .on_submit(|_| Message::IngestProjectDirectory)
            .width(Length::Fill),
        )
        .push(write_manifests_btn)
        .push(promote_all_btn)
        .push(
            widget::button::suggested("🔍 Scan Project Directory")
                .padding([4, 14])
                .height(Length::Fixed(28.0))
                .on_press(Message::IngestProjectDirectory),
        )
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let active_root_caption = if app.storage.project_toml_path().is_file() {
        format!(
            "Active Project: {} · Active Project Root: {}",
            app.project_name(),
            app.storage.project_root().display()
        )
    } else {
        "Active Project Root: (none — select or register a project in Settings)".to_string()
    };
    let components = &app.index.project.components.registry;
    let mut comp_col = widget::column::with_capacity(components.len() + 4)
        .push(widget::text::heading(
            "1. Project Directory Ingestion & Component Catalog",
        ))
        .push(widget::text::caption(active_root_caption))
        .push(scan_controls)
        .spacing(8)
        .width(Length::Fill);

    if components.is_empty() {
        comp_col = comp_col.push(
            widget::container(widget::text::caption(
                "No components indexed yet. Click 'Scan Project Directory' to discover crates, packages, scripts, and OS image directories.",
            ))
            .padding([8, 10])
            .width(Length::Fill)
            .class(cosmic::theme::Container::Secondary),
        );
    } else {
        for comp in components {
            let has_disk_manifest = app
                .storage
                .component_manifest_path(&comp.relative_path)
                .is_ok_and(|p| p.is_file());
            let manifest_badge = if has_disk_manifest {
                "nebulous.component.toml: present"
            } else {
                "nebulous.component.toml: index-only"
            };
            let stack_display = if comp.tech_stack.is_empty() {
                "none".to_string()
            } else {
                comp.tech_stack.join(", ")
            };
            let pin_badge = match comp.pinned_release.as_ref() {
                Some(pin) => format!(" · 📌 pin: {}", pin.commit_sha),
                None => String::new(),
            };
            let is_selected = app.settings_form.selected_component_id == comp.id;

            let row = widget::row::with_capacity(8)
                .push(widget::text::monotext(comp.id.clone()))
                .push(widget::text::body(format!(
                    "{} ({})",
                    comp.name, comp.relative_path
                )))
                .push(widget::space::horizontal().width(Length::Fill))
                .push(widget::text::caption(format!(
                    "type: {} · stack: [{}] · {} files · {}{}",
                    comp.component_type,
                    stack_display,
                    comp.files.len(),
                    manifest_badge,
                    pin_badge
                )))
                .push(if is_selected {
                    widget::button::suggested("Editing Pin")
                        .padding([2, 10])
                        .height(Length::Fixed(26.0))
                        .on_press(Message::SelectComponentForPin(comp.id.clone()))
                } else {
                    widget::button::standard("Edit / Pin")
                        .padding([2, 10])
                        .height(Length::Fixed(26.0))
                        .on_press(Message::SelectComponentForPin(comp.id.clone()))
                })
                .push(
                    widget::button::standard("Write/Update nebulous.component.toml")
                        .padding([2, 10])
                        .height(Length::Fixed(26.0))
                        .on_press(Message::WriteComponentManifest(comp.id.clone())),
                )
                .push(
                    widget::button::suggested("Promote to Scope")
                        .padding([2, 10])
                        .height(Length::Fixed(26.0))
                        .on_press(Message::PromoteComponentToScope(comp.id.clone())),
                )
                .spacing(6)
                .align_y(Alignment::Center)
                .width(Length::Fill);

            comp_col = comp_col.push(
                widget::container(row)
                    .padding([8, 10])
                    .width(Length::Fill)
                    .class(cosmic::theme::Container::Secondary),
            );
        }
    }

    comp_col.into()
}

fn view_component_pin_builder(app: &NebulousApp) -> Element<'_, Message> {
    let selected_comp = app.selected_component_manifest();
    let comp_summary = match selected_comp {
        Some(comp) => {
            let deps_summary = if comp.dependencies.is_empty() {
                "no dependencies tracked".to_string()
            } else {
                comp.dependencies
                    .iter()
                    .map(|d| format!("{}={} ({})", d.name, d.current_version, d.kind))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let pin_summary = match comp.pinned_release.as_ref() {
                Some(pin) => format!(
                    "Pinned @ {} ({}) · {} pinned deps",
                    pin.commit_sha,
                    pin.pinned_at,
                    pin.dependency_pins.len()
                ),
                None => "No known-good release pinned yet".to_string(),
            };
            format!(
                "Selected Component: '{}' ({})  ·  Deps: [{}]  ·  {}",
                comp.id, comp.relative_path, deps_summary, pin_summary
            )
        }
        None => "Select or enter a component ID above to edit tags, flags, dependencies, and known-good release pins.".to_string(),
    };

    let metadata_row = widget::row::with_capacity(5)
        .push(
            widget::text_input(
                "Component ID (e.g. backend_core or novos_iso)",
                &app.settings_form.selected_component_id,
            )
            .on_input(Message::SelectComponentForPin)
            .width(Length::FillPortion(1)),
        )
        .push(
            widget::text_input(
                "Tags (comma-separated: rolling, sid, iso)",
                &app.settings_form.component_tags_input,
            )
            .on_input(Message::ComponentTagsInputChanged)
            .width(Length::FillPortion(1)),
        )
        .push(
            widget::text_input(
                "Build Flags (comma-separated: --iso-level=3, --zstd)",
                &app.settings_form.component_flags_input,
            )
            .on_input(Message::ComponentFlagsInputChanged)
            .width(Length::FillPortion(1)),
        )
        .push(
            widget::text_input(
                "Build Script Path (e.g. scripts/build-iso.sh)",
                &app.settings_form.component_build_script_input,
            )
            .on_input(Message::ComponentBuildScriptInputChanged)
            .width(Length::FillPortion(1)),
        )
        .push(
            widget::button::suggested("💾 Save Component TOML")
                .padding([4, 12])
                .height(Length::Fixed(28.0))
                .on_press(Message::SaveComponentMetadata),
        )
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let dep_row = widget::row::with_capacity(6)
        .push(
            widget::text_input(
                "Dep Name (e.g. linux-image-xanmod)",
                &app.settings_form.dep_name_input,
            )
            .on_input(Message::ComponentDepNameChanged)
            .width(Length::FillPortion(2)),
        )
        .push(
            widget::text_input(
                "Channel/Kind (apt_sid, kernel_xanmod, cargo)",
                &app.settings_form.dep_kind_input,
            )
            .on_input(Message::ComponentDepKindChanged)
            .width(Length::FillPortion(1)),
        )
        .push(
            widget::text_input(
                "Constraint (e.g. >= 6.11)",
                &app.settings_form.dep_constraint_input,
            )
            .on_input(Message::ComponentDepConstraintChanged)
            .width(Length::FillPortion(1)),
        )
        .push(
            widget::text_input(
                "Current Version (e.g. 6.11.5-xanmod1)",
                &app.settings_form.dep_version_input,
            )
            .on_input(Message::ComponentDepVersionChanged)
            .width(Length::FillPortion(1)),
        )
        .push(
            widget::text_input(
                "Source (e.g. deb.xanmod.org)",
                &app.settings_form.dep_source_input,
            )
            .on_input(Message::ComponentDepSourceChanged)
            .on_submit(|_| Message::UpsertComponentDependency)
            .width(Length::FillPortion(1)),
        )
        .push(
            widget::button::standard("+ Upsert Dependency")
                .padding([4, 12])
                .height(Length::Fixed(28.0))
                .on_press(Message::UpsertComponentDependency),
        )
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let pin_actions_row = widget::row::with_capacity(3)
        .push(
            widget::button::suggested("📌 Pin Known-Good Release")
                .padding([4, 14])
                .height(Length::Fixed(30.0))
                .on_press(Message::PinSelectedComponentRelease),
        )
        .push(
            widget::button::destructive("⏪ Rollback to Pin")
                .padding([4, 14])
                .height(Length::Fixed(30.0))
                .on_press(Message::RollbackSelectedComponentRelease),
        )
        .spacing(8)
        .align_y(Alignment::Center);

    widget::container(
        widget::column::with_capacity(5)
            .push(widget::text::heading(
                "2. Component Manifest & Rolling-Release Pin Builder",
            ))
            .push(widget::text::caption(comp_summary))
            .push(metadata_row)
            .push(dep_row)
            .push(pin_actions_row)
            .spacing(8)
            .width(Length::Fill),
    )
    .padding([10, 12])
    .width(Length::Fill)
    .class(cosmic::theme::Container::Secondary)
    .into()
}

fn view_registered_phases_manager(app: &NebulousApp) -> Element<'_, Message> {
    let mut phases_col = widget::column::with_capacity(app.index.phases.len() + 3)
        .push(widget::text::heading("3. Registered Phases"))
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

    phases_col.push(create_phase_form).into()
}

fn view_domain_scopes_manager(app: &NebulousApp) -> Element<'_, Message> {
    let mut scopes_col = widget::column::with_capacity(10)
        .push(widget::text::heading(format!(
            "4. Domain Scopes in Active Phase ('{}')",
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
            let bound_comp = scope
                .manifest
                .scope
                .bound_component
                .as_deref()
                .unwrap_or("unbound");
            let tracked_files_count = scope.manifest.scope.tracked_files.len();

            let row = widget::row::with_capacity(5)
                .push(widget::text::monotext(scope.scope_id.clone()))
                .push(widget::text::body(format!(
                    "target: {}",
                    scope.manifest.scope.target
                )))
                .push(widget::space::horizontal().width(Length::Fill))
                .push(widget::text::caption(format!(
                    "component: {bound_comp} · {tracked_files_count} tracked files · stack: [{stack}] · {} items",
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

    scopes_col.push(create_scope_form).into()
}
