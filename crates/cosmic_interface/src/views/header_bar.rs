use cosmic::iced::{Alignment, Background, Border, Color, Length};
use cosmic::widget;
use cosmic::Element;

use crate::app::{FilterPill, Message, NebulousApp};
use backend_core::{ItemType, Severity};

/// Renders the custom composite top header & action bar inside the Nebulous shell.
///
/// Layout:
/// `[◆ Nebulous] [Phase: 01_core_engine ▾]   [All 6] [Bugs 3] [Features 3] [Crit/High] [⚠ Stagnant] [↻ Recurring]   [⏱ BUG-0001 45:12] [+Item] [⇅ a4f89c1]`
pub fn view_header_bar(app: &NebulousApp) -> Element<'_, Message> {
    let phase_selector = view_phase_selector(app);
    let filter_pills = view_filter_pills(app);
    let right_controls = view_right_controls(app);

    let top_row = widget::row::with_capacity(5)
        .push(phase_selector)
        .push(widget::space::horizontal().width(Length::Fixed(8.0)))
        .push(filter_pills)
        .push(widget::space::horizontal().width(Length::Fill))
        .push(right_controls)
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let mut col = widget::column::with_capacity(2)
        .push(top_row)
        .spacing(8)
        .width(Length::Fill);

    if app.quick_add.open {
        col = col.push(view_quick_add_bar(app));
    }

    widget::container(col)
        .padding([10, 14])
        .width(Length::Fill)
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

fn view_phase_selector(app: &NebulousApp) -> Element<'_, Message> {
    let brand_badge = widget::container(widget::text::heading(format!("◆ {}", app.project_name())))
        .padding([4, 8]);

    let selected_idx = app
        .phase_ids
        .iter()
        .position(|id| id == &app.active_phase_id);

    let phase_dropdown: Element<'_, Message> = if app.phase_labels.is_empty() {
        widget::text::caption("No Phases").into()
    } else {
        widget::dropdown(&app.phase_labels, selected_idx, Message::SelectPhaseIndex)
            .width(Length::Shrink)
            .into()
    };

    widget::row::with_capacity(3)
        .push(brand_badge)
        .push(widget::text::caption("Phase:"))
        .push(phase_dropdown)
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
}

fn view_filter_pills(app: &NebulousApp) -> Element<'_, Message> {
    let counts = app.filter_counts();
    let pills = [
        (FilterPill::All, format!("All ({})", counts.all)),
        (FilterPill::Bugs, format!("Bugs ({})", counts.bugs)),
        (
            FilterPill::Features,
            format!("Features ({})", counts.features),
        ),
        (
            FilterPill::HighOrCritical,
            format!("High/Crit ({})", counts.high_or_critical),
        ),
        (
            FilterPill::Stagnant,
            format!("⚠ Stagnant ({})", counts.stagnant),
        ),
        (
            FilterPill::Recurring,
            format!("↻ Recurring ({})", counts.recurring),
        ),
    ];

    let mut row = widget::row::with_capacity(pills.len() + 1)
        .spacing(6)
        .align_y(Alignment::Center);

    for (pill, label) in pills {
        let is_active = app.active_filter == pill;
        let btn = if is_active {
            widget::button::suggested(label)
                .padding([4, 10])
                .height(Length::Fixed(28.0))
                .on_press(Message::SetFilterPill(pill))
        } else {
            widget::button::standard(label)
                .padding([4, 10])
                .height(Length::Fixed(28.0))
                .on_press(Message::SetFilterPill(pill))
        };
        row = row.push(btn);
    }

    row.into()
}

fn view_right_controls(app: &NebulousApp) -> Element<'_, Message> {
    let mut row = widget::row::with_capacity(4)
        .spacing(8)
        .align_y(Alignment::Center);

    // Active timer indicator pill
    if let Some(ref timer) = app.active_timer {
        let mins = timer.elapsed_seconds / 60;
        let secs = timer.elapsed_seconds % 60;
        let timer_label = format!("⏱ {} {:02}:{:02}", timer.timer.item_id, mins, secs);

        let timer_pill = widget::row::with_capacity(2)
            .push(
                widget::button::suggested(timer_label)
                    .padding([4, 10])
                    .height(Length::Fixed(28.0))
                    .on_press(Message::SelectCard(timer.timer.item_id.clone())),
            )
            .push(
                widget::button::destructive("■ Stop")
                    .padding([4, 8])
                    .height(Length::Fixed(28.0))
                    .on_press(Message::StopTimer),
            )
            .spacing(4)
            .align_y(Alignment::Center);

        row = row.push(timer_pill);
    } else {
        let idle_badge = widget::container(widget::text::caption("⏱ Timer Idle"))
            .padding([4, 8])
            .class(cosmic::theme::Container::Secondary);
        row = row.push(idle_badge);
    }

    // Quick-add triggers (`+ Bug` / `+ Feature`)
    let quick_bug_btn = widget::button::standard("+ Bug")
        .padding([4, 10])
        .height(Length::Fixed(28.0))
        .on_press(Message::OpenQuickAdd(ItemType::Bug));

    let quick_feat_btn = widget::button::suggested("+ Feature")
        .padding([4, 10])
        .height(Length::Fixed(28.0))
        .on_press(Message::OpenQuickAdd(ItemType::Feature));

    // Git sync button showing short SHA and dirty state
    let sha_display = app
        .git_status
        .as_ref()
        .and_then(|st| st.head_short_sha.as_deref())
        .unwrap_or("uncommitted");
    let dirty_marker = if app.git_status.as_ref().is_some_and(|st| st.tracker_dirty) {
        "*"
    } else {
        ""
    };
    let sync_label = format!("⇅ {sha_display}{dirty_marker}");
    let sync_btn = widget::button::standard(sync_label)
        .padding([4, 10])
        .height(Length::Fixed(28.0))
        .on_press(Message::TriggerGitSync);

    row.push(quick_bug_btn)
        .push(quick_feat_btn)
        .push(sync_btn)
        .into()
}

fn view_quick_add_bar(app: &NebulousApp) -> Element<'_, Message> {
    let type_bug_btn = if app.quick_add.item_type == ItemType::Bug {
        widget::button::destructive("Bug")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::SetQuickAddType(ItemType::Bug))
    } else {
        widget::button::standard("Bug")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::SetQuickAddType(ItemType::Bug))
    };

    let type_feat_btn = if app.quick_add.item_type == ItemType::Feature {
        widget::button::suggested("Feature")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::SetQuickAddType(ItemType::Feature))
    } else {
        widget::button::standard("Feature")
            .padding([4, 10])
            .height(Length::Fixed(28.0))
            .on_press(Message::SetQuickAddType(ItemType::Feature))
    };

    let severity_options = [
        (Severity::Low, "Low"),
        (Severity::Medium, "Med"),
        (Severity::High, "High"),
        (Severity::Critical, "Crit"),
    ];
    let mut sev_row = widget::row::with_capacity(4).spacing(4);
    for (sev, label) in severity_options {
        let btn = if app.quick_add.severity == sev {
            widget::button::suggested(label)
                .padding([2, 8])
                .height(Length::Fixed(26.0))
                .on_press(Message::SetQuickAddSeverity(sev))
        } else {
            widget::button::standard(label)
                .padding([2, 8])
                .height(Length::Fixed(26.0))
                .on_press(Message::SetQuickAddSeverity(sev))
        };
        sev_row = sev_row.push(btn);
    }

    let placeholder = match app.quick_add.item_type {
        ItemType::Bug => "Quick-capture bug title (saves to active scope or inbox)...",
        ItemType::Feature => "Quick-capture feature title (saves to active scope or inbox)...",
    };

    let title_input = widget::text_input(placeholder, &app.quick_add.title)
        .on_input(Message::QuickAddTitleChanged)
        .on_submit(|_| Message::SubmitQuickAdd)
        .width(Length::Fill);

    let target_scope = app
        .selected_scope
        .as_deref()
        .unwrap_or_else(|| app.default_scope());
    let scope_badge = widget::container(widget::text::caption(format!("Scope: {target_scope}")))
        .padding([4, 8])
        .class(cosmic::theme::Container::Secondary);

    let create_btn = widget::button::suggested("Create")
        .padding([4, 12])
        .height(Length::Fixed(28.0))
        .on_press(Message::SubmitQuickAdd);

    let cancel_btn = widget::button::text("Cancel")
        .padding([4, 8])
        .height(Length::Fixed(28.0))
        .on_press(Message::CloseQuickAdd);

    let bar_row = widget::row::with_capacity(8)
        .push(type_bug_btn)
        .push(type_feat_btn)
        .push(sev_row)
        .push(scope_badge)
        .push(title_input)
        .push(create_btn)
        .push(cancel_btn)
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    widget::container(bar_row)
        .padding([8, 10])
        .width(Length::Fill)
        .class(cosmic::theme::Container::custom(|theme| {
            let cosmic = theme.cosmic();
            cosmic::iced::widget::container::Style {
                background: Some(Background::Color(Color::from_rgba(0.16, 0.19, 0.25, 0.95))),
                border: Border {
                    color: cosmic.accent.base.into(),
                    width: 1.0,
                    radius: cosmic.corner_radii.radius_xs.into(),
                },
                ..Default::default()
            }
        }))
        .into()
}
