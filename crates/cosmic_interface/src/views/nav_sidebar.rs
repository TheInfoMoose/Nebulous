use cosmic::iced::{Alignment, Background, Border, Length};
use cosmic::widget;
use cosmic::Element;

use crate::app::{Message, NebulousApp, ViewMode};

/// Renders the left navigation sidebar containing:
/// 1. Primary View Mode Switcher (`Kanban Board`, `Metrics & Hotspots`, `Phases & Scopes`, `Git & Settings`)
/// 2. Domain Scope selector (`All Scopes` + individual scopes from `phase.toml` with counts & tech stack badges)
/// 3. Daily Effort & Phase Completion valuation summary card
pub fn view_nav_sidebar(app: &NebulousApp) -> Element<'_, Message> {
    let view_mode_section = view_mode_switcher(app);
    let scopes_section = view_scopes_selector(app);
    let effort_section = view_daily_effort_summary(app);

    let sidebar_col = widget::column::with_capacity(7)
        .push(view_mode_section)
        .push(widget::divider::horizontal::default())
        .push(scopes_section)
        .push(widget::space::vertical().height(Length::Fill))
        .push(widget::divider::horizontal::default())
        .push(effort_section)
        .spacing(12)
        .width(Length::Fixed(248.0))
        .height(Length::Fill);

    widget::container(sidebar_col)
        .padding([12, 12])
        .width(Length::Fixed(256.0))
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

fn view_mode_switcher(app: &NebulousApp) -> Element<'_, Message> {
    let hotspot_count = app.hotspot_alert_count();
    let metrics_label = if hotspot_count > 0 {
        format!("◔ Metrics & Hotspots ({hotspot_count})")
    } else {
        "◔ Metrics & Hotspots".to_string()
    };

    let modes = [
        (ViewMode::Board, "▦ Kanban Board".to_string()),
        (ViewMode::Metrics, metrics_label),
        (ViewMode::PhasesAndScopes, "◈ Phases & Scopes".to_string()),
        (ViewMode::Settings, "⚙ Git & Settings".to_string()),
    ];

    let mut col = widget::column::with_capacity(modes.len() + 1)
        .push(widget::text::caption_heading("VIEW MODE"))
        .spacing(6)
        .width(Length::Fill);

    for (mode, label) in modes {
        let is_selected = app.view_mode == mode;
        let btn = if is_selected {
            widget::button::suggested(label)
                .width(Length::Fill)
                .height(Length::Fixed(32.0))
                .on_press(Message::SwitchView(mode))
        } else {
            widget::button::standard(label)
                .width(Length::Fill)
                .height(Length::Fixed(32.0))
                .on_press(Message::SwitchView(mode))
        };
        col = col.push(btn);
    }

    col.into()
}

fn view_scopes_selector(app: &NebulousApp) -> Element<'_, Message> {
    let mut col = widget::column::with_capacity(8)
        .push(widget::text::caption_heading("CRATE & DOMAIN SCOPES"))
        .spacing(6)
        .width(Length::Fill);

    let total_phase_items = app.active_phase_total_items();
    let all_scopes_label = format!("• All Scopes ({total_phase_items})");
    let all_btn = if app.selected_scope.is_none() {
        widget::button::suggested(all_scopes_label)
            .width(Length::Fill)
            .height(Length::Fixed(30.0))
            .on_press(Message::SelectScope(None))
    } else {
        widget::button::standard(all_scopes_label)
            .width(Length::Fill)
            .height(Length::Fixed(30.0))
            .on_press(Message::SelectScope(None))
    };
    col = col.push(all_btn);

    let mut scopes_list = widget::column::with_capacity(8)
        .spacing(6)
        .width(Length::Fill);

    if let Some(phase) = app.active_phase() {
        for scope_idx in &phase.scopes {
            let count = scope_idx.manifest.total_items();
            let scope_id = &scope_idx.scope_id;
            let is_selected = app.selected_scope.as_deref() == Some(scope_id.as_str());

            let btn_label = format!("• {scope_id} ({count})");
            let scope_btn = if is_selected {
                widget::button::suggested(btn_label)
                    .width(Length::Fill)
                    .height(Length::Fixed(30.0))
                    .on_press(Message::SelectScope(Some(scope_id.clone())))
            } else {
                widget::button::standard(btn_label)
                    .width(Length::Fill)
                    .height(Length::Fixed(30.0))
                    .on_press(Message::SelectScope(Some(scope_id.clone())))
            };

            let stack_tags = if scope_idx.manifest.scope.tech_stack.is_empty() {
                scope_idx.manifest.scope.target.clone()
            } else {
                let stack_preview = scope_idx
                    .manifest
                    .scope
                    .tech_stack
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" · ");
                format!("{} [{}]", scope_idx.manifest.scope.target, stack_preview)
            };

            let scope_entry = widget::column::with_capacity(2)
                .push(scope_btn)
                .push(widget::container(widget::text::caption(stack_tags)).padding([0, 6]))
                .spacing(2)
                .width(Length::Fill);

            scopes_list = scopes_list.push(scope_entry);
        }
    }

    col.push(widget::scrollable(scopes_list).height(Length::Shrink))
        .into()
}

fn view_daily_effort_summary(app: &NebulousApp) -> Element<'_, Message> {
    let target_daily_hours = app.target_daily_hours();
    let logged_minutes = app.active_phase_logged_minutes();
    let logged_hours = f64::from(logged_minutes) / 60.0;
    let progress = if target_daily_hours > 0.0 {
        (logged_hours / target_daily_hours).clamp(0.0, 1.0) as f32
    } else {
        0.0
    };

    let (completed, total) = app.active_phase_completion_counts();

    let effort_col = widget::column::with_capacity(5)
        .push(widget::text::caption_heading("DAILY EFFORT & PACE"))
        .push(
            widget::row::with_capacity(2)
                .push(widget::text::body(format!(
                    "{logged_hours:.1}h / {target_daily_hours:.1}h"
                )))
                .push(widget::space::horizontal().width(Length::Fill))
                .push(widget::text::caption(format!("{logged_minutes}m logged")))
                .align_y(Alignment::Center),
        )
        .push(
            widget::progress_bar::linear::Linear::new()
                .girth(8.0)
                .progress(progress)
                .width(Length::Fill),
        )
        .push(widget::text::caption(format!(
            "Phase: {completed}/{total} completed"
        )))
        .spacing(6)
        .width(Length::Fill);

    widget::container(effort_col)
        .padding([8, 10])
        .width(Length::Fill)
        .class(cosmic::theme::Container::Secondary)
        .into()
}
