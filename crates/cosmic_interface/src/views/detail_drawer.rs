use cosmic::iced::{Alignment, Background, Border, Color, Length};
use cosmic::widget;
use cosmic::Element;

use crate::app::{Message, NebulousApp};
use backend_core::{ItemStatus, ItemType, Severity};

/// Renders the right-hand lazy-loaded Context Drawer when a card is selected.
///
/// `item.toml` and `desc.md` are loaded lazily only when an item card is clicked.
/// Provides:
/// - Editable item title, status controls (with reopen recurrence logic), and severity selector
/// - One-click Agent Context Handoff buttons (`Copy @file Refs` & `Copy Bounded Agent Prompt`)
/// - Effort & Session Timer panel (Start/Stop timer, `+15m`/`+30m`/`+60m` quick log, custom minute input, `iteration_count`)
/// - Built-in Markdown editor (`cosmic::widget::text_editor`) for `desc.md` with atomic save, template helpers, and `Open in $EDITOR`
/// - Historical Fix Ledger (`recurrence_count` warning banner, chronological `previous_fixes` list, and `Close with Fix Summary` workflow capturing Git `HEAD` short SHA)
pub fn view_detail_drawer(app: &NebulousApp) -> Option<Element<'_, Message>> {
    let selected_id = app.selected_item_id.as_deref()?;
    let detail = app.selected_item_detail.as_ref()?;
    let item = &detail.manifest.item;
    let effort = &detail.manifest.effort;
    let history = &detail.manifest.history;

    let type_tag = match item.item_type {
        ItemType::Bug => "● BUG",
        ItemType::Feature => "◆ FEAT",
    };

    let close_drawer_btn = widget::button::text("✕")
        .padding([2, 8])
        .height(Length::Fixed(26.0))
        .on_press(Message::ClearSelectedCard);

    let header_row = widget::row::with_capacity(5)
        .push(widget::text::monotext(selected_id.to_string()))
        .push(widget::text::caption(format!("[{type_tag}]")))
        .push(widget::text::caption(format!("[{}]", item.severity)))
        .push(widget::space::horizontal().width(Length::Fill))
        .push(close_drawer_btn)
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let scope_line = widget::text::caption(format!(
        "Scope: {}  ·  Updated: {}",
        item.scope_ref, item.updated_at
    ));

    // Editable Title row
    let title_input = widget::text_input("Item title...", &app.drawer_title_input)
        .on_input(Message::DrawerTitleChanged)
        .on_submit(|_| Message::SaveDrawerTitle)
        .width(Length::Fill);

    let save_title_btn = widget::button::standard("Save Title")
        .padding([3, 8])
        .height(Length::Fixed(28.0))
        .on_press(Message::SaveDrawerTitle);

    let title_row = widget::row::with_capacity(2)
        .push(title_input)
        .push(save_title_btn)
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    // Status controls row
    let statuses = [
        (ItemStatus::Open, "Open"),
        (ItemStatus::InProgress, "In Prog"),
        (ItemStatus::Pending, "Pending"),
        (ItemStatus::Closed, "Closed"),
        (ItemStatus::Merged, "Merged"),
    ];
    let mut status_row = widget::row::with_capacity(statuses.len()).spacing(4);
    for (st, label) in statuses {
        let btn = if item.status == st {
            widget::button::suggested(label)
                .padding([3, 7])
                .height(Length::Fixed(26.0))
                .on_press(Message::QuickStatusTransition {
                    item_id: item.id.clone(),
                    new_status: st,
                })
        } else {
            widget::button::standard(label)
                .padding([3, 7])
                .height(Length::Fixed(26.0))
                .on_press(Message::QuickStatusTransition {
                    item_id: item.id.clone(),
                    new_status: st,
                })
        };
        status_row = status_row.push(btn);
    }

    // Severity selector row
    let severities = [
        (Severity::Low, "Low"),
        (Severity::Medium, "Med"),
        (Severity::High, "High"),
        (Severity::Critical, "Crit"),
    ];
    let mut sev_row = widget::row::with_capacity(severities.len() + 1)
        .push(widget::text::caption("Severity:"))
        .spacing(4)
        .align_y(Alignment::Center);

    for (sev, label) in severities {
        let btn = if item.severity == sev {
            widget::button::suggested(label)
                .padding([2, 8])
                .height(Length::Fixed(24.0))
                .on_press(Message::UpdateSelectedSeverity(sev))
        } else {
            widget::button::standard(label)
                .padding([2, 8])
                .height(Length::Fixed(24.0))
                .on_press(Message::UpdateSelectedSeverity(sev))
        };
        sev_row = sev_row.push(btn);
    }

    // Agent Context Handoff Bar (`Copy @file Refs` & `Copy Bounded Agent Prompt`)
    let copy_refs_btn = widget::button::standard("📋 Copy @file Refs")
        .padding([4, 10])
        .height(Length::Fixed(28.0))
        .width(Length::FillPortion(1))
        .on_press(Message::CopyAgentFileRefs);

    let copy_prompt_btn = widget::button::suggested("✦ Copy Agent Prompt")
        .padding([4, 10])
        .height(Length::Fixed(28.0))
        .width(Length::FillPortion(1))
        .on_press(Message::CopyBoundedAgentPrompt);

    let agent_handoff_section = widget::container(
        widget::column::with_capacity(2)
            .push(widget::text::caption_heading("AGENT CONTEXT HANDOFF"))
            .push(
                widget::row::with_capacity(2)
                    .push(copy_refs_btn)
                    .push(copy_prompt_btn)
                    .spacing(6)
                    .width(Length::Fill),
            )
            .spacing(6)
            .width(Length::Fill),
    )
    .padding([8, 10])
    .width(Length::Fill)
    .class(cosmic::theme::Container::Secondary);

    // Effort & Timer Panel
    let is_timer_running = app.is_timer_running_for(&item.id);
    let timer_btn = if is_timer_running {
        let elapsed_str = app
            .active_timer
            .as_ref()
            .map(|t| {
                format!(
                    "{:02}:{:02}",
                    t.elapsed_seconds / 60,
                    t.elapsed_seconds % 60
                )
            })
            .unwrap_or_else(|| "00:00".to_string());
        widget::button::destructive(format!("■ Stop ({elapsed_str})"))
            .padding([3, 8])
            .height(Length::Fixed(26.0))
            .on_press(Message::StopTimer)
    } else {
        widget::button::suggested("⏱ Start Timer")
            .padding([3, 8])
            .height(Length::Fixed(26.0))
            .on_press(Message::StartTimer(item.id.clone()))
    };

    let quick_effort_row = widget::row::with_capacity(5)
        .push(timer_btn)
        .push(
            widget::button::standard("+15m")
                .padding([3, 7])
                .height(Length::Fixed(26.0))
                .on_press(Message::LogManualMinutes(15)),
        )
        .push(
            widget::button::standard("+30m")
                .padding([3, 7])
                .height(Length::Fixed(26.0))
                .on_press(Message::LogManualMinutes(30)),
        )
        .push(
            widget::button::standard("+60m")
                .padding([3, 7])
                .height(Length::Fixed(26.0))
                .on_press(Message::LogManualMinutes(60)),
        )
        .spacing(4)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let custom_mins_row = widget::row::with_capacity(3)
        .push(widget::text::caption("Manual mins:"))
        .push(
            widget::text_input("Minutes (e.g. 25)", &app.drawer_manual_minutes_input)
                .on_input(Message::ManualMinutesInputChanged)
                .on_submit(|_| Message::SubmitCustomManualMinutes)
                .width(Length::Fill),
        )
        .push(
            widget::button::standard("+ Log")
                .padding([3, 8])
                .height(Length::Fixed(26.0))
                .on_press(Message::SubmitCustomManualMinutes),
        )
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let effort_hours = f64::from(effort.logged_time_minutes) / 60.0;
    let effort_panel = widget::container(
        widget::column::with_capacity(4)
            .push(
                widget::row::with_capacity(3)
                    .push(widget::text::caption_heading("EFFORT & SESSION TIMER"))
                    .push(widget::space::horizontal().width(Length::Fill))
                    .push(widget::text::caption(format!(
                        "{}m ({effort_hours:.1}h) · {} iter(s) · ↻ {} reopens",
                        effort.logged_time_minutes,
                        effort.iteration_count,
                        history.recurrence_count
                    )))
                    .align_y(Alignment::Center)
                    .width(Length::Fill),
            )
            .push(quick_effort_row)
            .push(custom_mins_row)
            .spacing(6)
            .width(Length::Fill),
    )
    .padding([8, 10])
    .width(Length::Fill)
    .class(cosmic::theme::Container::Secondary);

    // Markdown Specification Editor (`desc.md`) with atomic save & Open in $EDITOR
    let dirty_badge = if app.desc_editor_dirty {
        "SPECIFICATION (desc.md *unsaved*)"
    } else {
        "SPECIFICATION (desc.md)"
    };

    let editor_header_row = widget::row::with_capacity(4)
        .push(widget::text::caption_heading(dirty_badge))
        .push(widget::space::horizontal().width(Length::Fill))
        .push(
            widget::button::standard("↗ $EDITOR")
                .padding([2, 8])
                .height(Length::Fixed(24.0))
                .on_press(Message::OpenInExternalEditor),
        )
        .push(
            widget::button::suggested("💾 Save desc.md")
                .padding([2, 8])
                .height(Length::Fixed(24.0))
                .on_press(Message::SaveDescMarkdown),
        )
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let template_bar = widget::row::with_capacity(4)
        .push(widget::text::caption("Insert:"))
        .push(
            widget::button::standard("+ Repro Step")
                .padding([2, 6])
                .height(Length::Fixed(22.0))
                .on_press(Message::AppendDescTemplate(
                    "\n- [Repro] Observed under condition: ",
                )),
        )
        .push(
            widget::button::standard("+ Fix Note")
                .padding([2, 6])
                .height(Length::Fixed(22.0))
                .on_press(Message::AppendDescTemplate(
                    "\n- [Fix Note] Root cause & mitigation: ",
                )),
        )
        .push(
            widget::button::standard("+ Check")
                .padding([2, 6])
                .height(Length::Fixed(22.0))
                .on_press(Message::AppendDescTemplate(
                    "\n- [Verification] Verified with cargo test: ",
                )),
        )
        .spacing(4)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let md_editor = widget::text_editor::TextEditor::new(&app.desc_editor_content)
        .placeholder("Markdown specification (desc.md)...")
        .on_action(Message::DescEditorAction)
        .height(Length::Fixed(200.0));

    let editor_box = widget::container(md_editor)
        .padding([4, 4])
        .width(Length::Fill)
        .class(cosmic::theme::Container::Secondary);

    // Historical Fix Ledger (`recurrence_count` & `previous_fixes`) + Close/Reopen workflow
    let mut ledger_col = widget::column::with_capacity(history.previous_fixes.len() + 4)
        .push(widget::text::caption_heading(format!(
            "HISTORICAL FIX LEDGER ({}) · RECURRENCE: ↻ {}",
            history.previous_fixes.len(),
            history.recurrence_count
        )))
        .spacing(6)
        .width(Length::Fill);

    if history.recurrence_count > 0 {
        let recurrence_banner = widget::container(widget::text::caption(format!(
            "⚠ Regression Warning: Reopened ↻ {} time(s) after prior closure.",
            history.recurrence_count
        )))
        .padding([6, 8])
        .width(Length::Fill)
        .class(cosmic::theme::Container::custom(|theme| {
            let cosmic = theme.cosmic();
            cosmic::iced::widget::container::Style {
                background: Some(Background::Color(Color::from_rgba(0.88, 0.30, 0.42, 0.18))),
                border: Border {
                    color: Color::from_rgb(0.88, 0.30, 0.42),
                    width: 1.0,
                    radius: cosmic.corner_radii.radius_xs.into(),
                },
                ..Default::default()
            }
        }));
        ledger_col = ledger_col.push(recurrence_banner);
    }

    if history.previous_fixes.is_empty() {
        ledger_col = ledger_col.push(widget::text::caption("No previous fixes recorded."));
    } else {
        for (idx, fix) in history.previous_fixes.iter().enumerate() {
            let line = format!(
                "#{}: [{}] commit {} — {}",
                idx + 1,
                fix.date,
                fix.commit,
                fix.summary
            );
            ledger_col = ledger_col.push(
                widget::container(widget::text::caption(line))
                    .padding([5, 8])
                    .width(Length::Fill)
                    .class(cosmic::theme::Container::Secondary),
            );
        }
    }

    // Reopen with Reason panel (shown when item is Closed or Merged)
    if item.status.is_completed() {
        let reopen_panel = widget::container(
            widget::column::with_capacity(3)
                .push(widget::text::caption_heading(
                    "↻ REOPEN ITEM (AUTO-INCREMENTS RECURRENCE & ITERATION)",
                ))
                .push(
                    widget::text_input(
                        "Optional reopen reason (appended to desc.md)...",
                        &app.drawer_reopen_reason_input,
                    )
                    .on_input(Message::ReopenReasonInputChanged)
                    .on_submit(|_| Message::ReopenSelectedItem)
                    .width(Length::Fill),
                )
                .push(
                    widget::button::destructive("↻ Reopen Item")
                        .padding([4, 10])
                        .height(Length::Fixed(28.0))
                        .on_press(Message::ReopenSelectedItem),
                )
                .spacing(6)
                .width(Length::Fill),
        )
        .padding([8, 10])
        .width(Length::Fill)
        .class(cosmic::theme::Container::Secondary);

        ledger_col = ledger_col.push(reopen_panel);
    }

    // Close with Fix Summary panel (captures Git HEAD short SHA into `previous_fixes`)
    let head_sha = app
        .git_status
        .as_ref()
        .and_then(|s| s.head_short_sha.as_deref())
        .unwrap_or("uncommitted");

    let close_panel = widget::container(
        widget::column::with_capacity(4)
            .push(widget::text::caption_heading(format!(
                "✓ CLOSE WITH FIX SUMMARY (GIT HEAD: {head_sha})"
            )))
            .push(
                widget::text_input(
                    "Fix summary (recorded in previous_fixes)...",
                    &app.drawer_close_summary_input,
                )
                .on_input(Message::CloseSummaryInputChanged)
                .on_submit(|_| Message::CloseSelectedWithSummary { merged: false })
                .width(Length::Fill),
            )
            .push(
                widget::text_input(
                    "Commit SHA override (optional, defaults to Git HEAD)...",
                    &app.drawer_close_commit_input,
                )
                .on_input(Message::CloseCommitInputChanged)
                .on_submit(|_| Message::CloseSelectedWithSummary { merged: false })
                .width(Length::Fill),
            )
            .push(
                widget::row::with_capacity(2)
                    .push(
                        widget::button::suggested("✓ Close & Record Fix")
                            .padding([4, 10])
                            .height(Length::Fixed(28.0))
                            .width(Length::FillPortion(1))
                            .on_press(Message::CloseSelectedWithSummary { merged: false }),
                    )
                    .push(
                        widget::button::standard("⎇ Merge & Record Fix")
                            .padding([4, 10])
                            .height(Length::Fixed(28.0))
                            .width(Length::FillPortion(1))
                            .on_press(Message::CloseSelectedWithSummary { merged: true }),
                    )
                    .spacing(6)
                    .width(Length::Fill),
            )
            .spacing(6)
            .width(Length::Fill),
    )
    .padding([8, 10])
    .width(Length::Fill)
    .class(cosmic::theme::Container::Secondary);

    let drawer_inner = widget::column::with_capacity(14)
        .push(header_row)
        .push(scope_line)
        .push(title_row)
        .push(status_row)
        .push(sev_row)
        .push(agent_handoff_section)
        .push(effort_panel)
        .push(widget::divider::horizontal::default())
        .push(editor_header_row)
        .push(template_bar)
        .push(editor_box)
        .push(widget::divider::horizontal::default())
        .push(ledger_col)
        .push(close_panel)
        .spacing(8)
        .width(Length::Fixed(376.0));

    let scrollable_drawer = widget::scrollable(drawer_inner)
        .width(Length::Fill)
        .height(Length::Fill);

    Some(
        widget::container(scrollable_drawer)
            .padding([12, 12])
            .width(Length::Fixed(400.0))
            .height(Length::Fill)
            .class(cosmic::theme::Container::custom(|theme| {
                let cosmic = theme.cosmic();
                cosmic::iced::widget::container::Style {
                    background: Some(Background::Color(
                        cosmic.primary(theme.transparent).base.into(),
                    )),
                    border: Border {
                        color: Color::from_rgb(0.32, 0.56, 0.84),
                        width: 1.0,
                        radius: cosmic.corner_radii.radius_s.into(),
                    },
                    ..Default::default()
                }
            }))
            .into(),
    )
}
