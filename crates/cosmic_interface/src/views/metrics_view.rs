use cosmic::iced::{Alignment, Background, Border, Color, Length};
use cosmic::widget;
use cosmic::Element;

use crate::app::{Message, NebulousApp, ViewMode};

/// Renders the Metrics, Time Valuation, Stagnant Items & Recurring Bug Hotspots dashboard.
pub fn view_metrics_dashboard(app: &NebulousApp) -> Element<'_, Message> {
    let report = app.compute_current_metrics();

    let header_row = widget::row::with_capacity(4)
        .push(widget::text::title4(
            "Metrics: Phase/Scope Time Valuation, Stagnant Items & Recurring Hotspots",
        ))
        .push(widget::space::horizontal().width(Length::Fill))
        .push(
            widget::button::standard("↻ Refresh Metrics")
                .height(Length::Fixed(30.0))
                .on_press(Message::ReconcileIndexes),
        )
        .push(
            widget::button::standard("← Back to Kanban Board")
                .height(Length::Fixed(30.0))
                .on_press(Message::SwitchView(ViewMode::Board)),
        )
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    // Top 4 Summary Cards
    let summary_cards = widget::row::with_capacity(4)
        .push(metric_stat_card(
            "TOTAL LOGGED EFFORT",
            format!(
                "{:.1}h ({}m)",
                report.total_logged_hours, report.total_logged_minutes
            ),
            format!(
                "Target: {:.1}h/day · {:.2} target days equiv",
                report.target_daily_hours, report.target_days_equivalent
            ),
        ))
        .push(metric_stat_card(
            "PROJECT COMPLETION",
            format!("{} / {}", report.completed_items, report.total_items),
            format!("Active phase: {}", app.active_phase_id),
        ))
        .push(metric_stat_card(
            "STAGNANT ITEMS",
            format!("{}", report.stagnant_items.len()),
            format!(
                "Threshold: >= {}d without update",
                report.stagnant_threshold_days
            ),
        ))
        .push(metric_stat_card(
            "RECURRING HOTSPOTS",
            format!(
                "{} items / {} scopes",
                report.item_hotspots.len(),
                report.scope_hotspots.len()
            ),
            "Ranked by recurrence_count & fix ledger".to_string(),
        ))
        .spacing(12)
        .width(Length::Fill);

    // Section 1: Phase & Domain Scope Time Valuation vs `target_daily_hours`
    let mut valuation_col = widget::column::with_capacity(report.phases.len() + 2)
        .push(widget::text::heading(format!(
            "⏱ Phase & Scope Time Valuation (vs {:.1}h Target Daily Pace)",
            report.target_daily_hours
        )))
        .spacing(8)
        .width(Length::Fill);

    for phase in &report.phases {
        let is_active = phase.phase_id == app.active_phase_id;
        let active_tag = if is_active { " [ACTIVE]" } else { "" };
        let completion_ratio = if phase.total_items > 0 {
            (phase.completed_items as f32 / phase.total_items as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let pace_progress = if report.target_daily_hours > 0.0 {
            ((phase.logged_hours / report.target_daily_hours) as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let phase_header = widget::row::with_capacity(3)
            .push(widget::text::monotext(format!(
                "{}{}",
                phase.phase_id, active_tag
            )))
            .push(widget::text::body(phase.phase_title.clone()))
            .push(widget::space::horizontal().width(Length::Fill))
            .push(widget::text::caption(format!(
                "{}/{} done · {:.1}h ({}m) logged · {:.2} target days",
                phase.completed_items,
                phase.total_items,
                phase.logged_hours,
                phase.logged_minutes,
                phase.target_days_equivalent
            )))
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Length::Fill);

        let bars_row = widget::row::with_capacity(4)
            .push(widget::text::caption("Daily Target Pace:"))
            .push(
                widget::progress_bar::linear::Linear::new()
                    .girth(7.0)
                    .progress(pace_progress)
                    .width(Length::FillPortion(1)),
            )
            .push(widget::text::caption("Item Completion:"))
            .push(
                widget::progress_bar::linear::Linear::new()
                    .girth(7.0)
                    .progress(completion_ratio)
                    .width(Length::FillPortion(1)),
            )
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Length::Fill);

        let mut scope_rows = widget::column::with_capacity(phase.scopes.len()).spacing(4);
        for scope_val in &phase.scopes {
            let daily_pct = if report.target_daily_hours > 0.0 {
                (scope_val.logged_hours / report.target_daily_hours) * 100.0
            } else {
                0.0
            };

            let scope_row = widget::row::with_capacity(5)
                .push(widget::text::monotext(format!(
                    "  • {}",
                    scope_val.scope_id
                )))
                .push(widget::text::caption(format!("({})", scope_val.target)))
                .push(widget::space::horizontal().width(Length::Fill))
                .push(widget::text::caption(format!(
                    "{}/{} done · {:.1}h ({}m) · {:.0}% of daily target",
                    scope_val.completed_items,
                    scope_val.total_items,
                    scope_val.logged_hours,
                    scope_val.logged_minutes,
                    daily_pct
                )))
                .push(
                    widget::button::standard("Filter Board")
                        .padding([2, 8])
                        .height(Length::Fixed(24.0))
                        .on_press(Message::SelectPhaseScopeAndOpenBoard {
                            phase_id: scope_val.phase_id.clone(),
                            scope_id: scope_val.scope_id.clone(),
                        }),
                )
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill);

            scope_rows = scope_rows.push(scope_row);
        }

        let phase_card = widget::container(
            widget::column::with_capacity(3)
                .push(phase_header)
                .push(bars_row)
                .push(scope_rows)
                .spacing(6)
                .width(Length::Fill),
        )
        .padding([10, 12])
        .width(Length::Fill)
        .class(cosmic::theme::Container::Secondary);

        valuation_col = valuation_col.push(phase_card);
    }

    // Section 2: Stagnant Items Alert List
    let mut stagnant_col = widget::column::with_capacity(report.stagnant_items.len().max(1) + 1)
        .push(widget::text::heading(format!(
            "⚠ Stagnant Items ({} exceeding {}d threshold)",
            report.stagnant_items.len(),
            report.stagnant_threshold_days
        )))
        .spacing(6)
        .width(Length::Fill);

    if report.stagnant_items.is_empty() {
        stagnant_col = stagnant_col.push(widget::text::caption(
            "No non-closed items exceed the stagnation threshold.",
        ));
    } else {
        for entry in &report.stagnant_items {
            let row = widget::row::with_capacity(6)
                .push(widget::text::monotext(entry.item_id.clone()))
                .push(widget::text::caption(format!("[{}]", entry.severity)))
                .push(widget::text::body(entry.title.clone()))
                .push(widget::space::horizontal().width(Length::Fill))
                .push(widget::text::caption(format!(
                    "[{}/{}] ⚠ {}d stale · {}m logged",
                    entry.phase_id,
                    entry.scope_id,
                    entry.days_since_update,
                    entry.logged_time_minutes
                )))
                .push(
                    widget::button::suggested("Inspect")
                        .padding([2, 8])
                        .height(Length::Fixed(26.0))
                        .on_press(Message::SelectCard(entry.item_id.clone())),
                )
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill);

            stagnant_col = stagnant_col.push(
                widget::container(row)
                    .padding([8, 10])
                    .width(Length::Fill)
                    .class(cosmic::theme::Container::Secondary),
            );
        }
    }

    // Section 3: Recurring Bug Hotspots Leaderboard (Scopes + Items)
    let mut hotspots_col = widget::column::with_capacity(12)
        .push(widget::text::heading(
            "↻ Recurring Bug & Fix Hotspots Leaderboard",
        ))
        .spacing(8)
        .width(Length::Fill);

    // 3a. Fragile Domain Scopes Leaderboard
    hotspots_col = hotspots_col.push(widget::text::caption_heading("FRAGILE DOMAIN SCOPES"));
    if report.scope_hotspots.is_empty() {
        hotspots_col = hotspots_col.push(widget::text::caption(
            "No domain scopes have recorded recurrences or fix history yet.",
        ));
    } else {
        for (rank, scope_hs) in report.scope_hotspots.iter().enumerate() {
            let row = widget::row::with_capacity(5)
                .push(widget::text::monotext(format!(
                    "#{} {}",
                    rank + 1,
                    scope_hs.scope_id
                )))
                .push(widget::text::caption(format!(
                    "target: {}",
                    scope_hs.target
                )))
                .push(widget::space::horizontal().width(Length::Fill))
                .push(widget::text::caption(format!(
                    "↻ {} recurrences ({} items) · {} fixes · {}m total effort",
                    scope_hs.total_recurrences,
                    scope_hs.recurring_items_count,
                    scope_hs.total_previous_fixes,
                    scope_hs.total_logged_minutes
                )))
                .push(
                    widget::button::standard("Filter Board")
                        .padding([2, 8])
                        .height(Length::Fixed(24.0))
                        .on_press(Message::SelectScopeAndOpenBoard(scope_hs.scope_id.clone())),
                )
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill);

            hotspots_col = hotspots_col.push(
                widget::container(row)
                    .padding([8, 10])
                    .width(Length::Fill)
                    .class(cosmic::theme::Container::Secondary),
            );
        }
    }

    // 3b. Recurring Item Hotspots Leaderboard
    hotspots_col = hotspots_col.push(widget::text::caption_heading("RECURRING ITEM HOTSPOTS"));
    if report.item_hotspots.is_empty() {
        hotspots_col = hotspots_col.push(widget::text::caption(
            "No items have recurrence counts or previous fix records.",
        ));
    } else {
        for (rank, item_hs) in report.item_hotspots.iter().enumerate() {
            let latest_fix_note = item_hs
                .previous_fixes
                .last()
                .map(|f| format!("Latest fix [{} {}]: {}", f.date, f.commit, f.summary))
                .unwrap_or_else(|| "No recorded fix summary".to_string());

            let top_row = widget::row::with_capacity(6)
                .push(widget::text::monotext(format!(
                    "#{} {}",
                    rank + 1,
                    item_hs.item_id
                )))
                .push(widget::text::caption(format!("[{}]", item_hs.severity)))
                .push(widget::text::body(item_hs.title.clone()))
                .push(widget::space::horizontal().width(Length::Fill))
                .push(widget::text::caption(format!(
                    "[{}] ↻ {} reopens · {} iter · {} fixes · {}m",
                    item_hs.scope_id,
                    item_hs.recurrence_count,
                    item_hs.iteration_count,
                    item_hs.previous_fixes_count,
                    item_hs.logged_time_minutes
                )))
                .push(
                    widget::button::suggested("Inspect")
                        .padding([2, 8])
                        .height(Length::Fixed(26.0))
                        .on_press(Message::SelectCard(item_hs.item_id.clone())),
                )
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill);

            let card_col = widget::column::with_capacity(2)
                .push(top_row)
                .push(widget::text::caption(latest_fix_note))
                .spacing(4)
                .width(Length::Fill);

            let accent = if item_hs.recurrence_count > 0 {
                Color::from_rgb(0.88, 0.30, 0.42)
            } else {
                Color::from_rgb(0.32, 0.68, 0.52)
            };

            hotspots_col = hotspots_col.push(
                widget::container(card_col)
                    .padding([8, 10])
                    .width(Length::Fill)
                    .class(cosmic::theme::Container::custom(move |theme| {
                        let cosmic = theme.cosmic();
                        cosmic::iced::widget::container::Style {
                            background: Some(Background::Color(
                                cosmic.secondary(theme.transparent).base.into(),
                            )),
                            border: Border {
                                color: accent,
                                width: 1.0,
                                radius: cosmic.corner_radii.radius_xs.into(),
                            },
                            ..Default::default()
                        }
                    })),
            );
        }
    }

    let content = widget::column::with_capacity(8)
        .push(header_row)
        .push(summary_cards)
        .push(widget::divider::horizontal::default())
        .push(valuation_col)
        .push(widget::divider::horizontal::default())
        .push(stagnant_col)
        .push(widget::divider::horizontal::default())
        .push(hotspots_col)
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

fn metric_stat_card<'a>(title: &'a str, value: String, subtitle: String) -> Element<'a, Message> {
    let col = widget::column::with_capacity(3)
        .push(widget::text::caption_heading(title))
        .push(widget::text::title4(value))
        .push(widget::text::caption(subtitle))
        .spacing(4)
        .width(Length::Fill);

    widget::container(col)
        .padding([10, 12])
        .width(Length::FillPortion(1))
        .class(cosmic::theme::Container::Secondary)
        .into()
}
