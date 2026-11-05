use cosmic::iced::{Alignment, Background, Border, Color, Length};
use cosmic::widget;
use cosmic::Element;

use crate::app::{KanbanCardViewModel, KanbanColumn, Message, NebulousApp};
use backend_core::{ItemStatus, ItemType, Severity};

/// Renders the central 4-column visual Kanban card board (`Open`, `In Progress`, `Pending`, `Closed / Merged`).
///
/// Cards are designed with high visual clarity and zero spreadsheet clutter:
/// - Item ID (`BUG-0001` / `FEAT-0001`), Type & Severity badge
/// - Prominent title
/// - Scope tag (`backend_core`, `cosmic_interface`, etc.)
/// - Stagnation flag (`⚠ Xd`), Recurrence badge (`↻ N`), and Effort badge (`Xm`)
/// - Quick status progression & timer toggle controls
pub fn view_kanban_board(app: &NebulousApp) -> Element<'_, Message> {
    let cards = app.filtered_kanban_cards();

    let columns = [
        KanbanColumn::Open,
        KanbanColumn::InProgress,
        KanbanColumn::Pending,
        KanbanColumn::ClosedOrMerged,
    ];

    let mut board_row = widget::row::with_capacity(4)
        .spacing(12)
        .width(Length::Fill)
        .height(Length::Fill);

    for column_kind in columns {
        let col_cards: Vec<KanbanCardViewModel> = cards
            .iter()
            .filter(|c| column_kind.matches(c.status))
            .cloned()
            .collect();

        board_row = board_row.push(view_kanban_column(app, column_kind, col_cards));
    }

    widget::container(board_row)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn view_kanban_column<'a>(
    app: &'a NebulousApp,
    column_kind: KanbanColumn,
    cards: Vec<KanbanCardViewModel>,
) -> Element<'a, Message> {
    let header_title = format!("{} ({})", column_kind.title(), cards.len());
    let accent_color = column_accent_color(column_kind);

    let col_header = widget::container(
        widget::row::with_capacity(2)
            .push(widget::text::heading(header_title))
            .push(widget::space::horizontal().width(Length::Fill))
            .align_y(Alignment::Center),
    )
    .padding([8, 10])
    .width(Length::Fill)
    .class(cosmic::theme::Container::custom(move |theme| {
        let cosmic = theme.cosmic();
        cosmic::iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgba(
                accent_color.r,
                accent_color.g,
                accent_color.b,
                0.14,
            ))),
            border: Border {
                color: accent_color,
                width: 1.0,
                radius: cosmic.corner_radii.radius_xs.into(),
            },
            ..Default::default()
        }
    }));

    let mut cards_col = widget::column::with_capacity(cards.len().max(1))
        .spacing(10)
        .width(Length::Fill);

    if cards.is_empty() {
        let empty_box = widget::container(widget::text::caption("No items in column"))
            .padding([18, 12])
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .class(cosmic::theme::Container::Secondary);
        cards_col = cards_col.push(empty_box);
    } else {
        for card in cards {
            cards_col = cards_col.push(view_kanban_card(app, card));
        }
    }

    let scrollable_cards = widget::scrollable(cards_col)
        .width(Length::Fill)
        .height(Length::Fill);

    let column_container = widget::column::with_capacity(2)
        .push(col_header)
        .push(scrollable_cards)
        .spacing(10)
        .width(Length::FillPortion(1))
        .height(Length::Fill);

    widget::container(column_container)
        .padding([10, 10])
        .width(Length::FillPortion(1))
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

fn view_kanban_card<'a>(app: &'a NebulousApp, card: KanbanCardViewModel) -> Element<'a, Message> {
    let is_selected = app.selected_item_id.as_deref() == Some(card.id.as_str());
    let is_timer_running = app.is_timer_running_for(&card.id);

    // Top row: ID + Type pill + Severity badge
    let type_prefix = match card.item_type {
        ItemType::Bug => "● BUG",
        ItemType::Feature => "◆ FEAT",
    };
    let id_label = widget::text::monotext(card.id.clone());
    let type_badge = make_pill_badge(
        type_prefix.to_string(),
        match card.item_type {
            ItemType::Bug => Color::from_rgb(0.86, 0.34, 0.34),
            ItemType::Feature => Color::from_rgb(0.28, 0.62, 0.92),
        },
    );
    let sev_badge = make_pill_badge(
        severity_label(card.severity).to_string(),
        severity_color(card.severity),
    );

    let top_row = widget::row::with_capacity(4)
        .push(id_label)
        .push(type_badge)
        .push(widget::space::horizontal().width(Length::Fill))
        .push(sev_badge)
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    // Title button (clicking selects card and lazily loads item.toml + desc.md)
    let select_btn = if is_selected {
        widget::button::suggested(card.title.clone())
            .width(Length::Fill)
            .on_press(Message::SelectCard(card.id.clone()))
    } else {
        widget::button::text(card.title.clone())
            .width(Length::Fill)
            .on_press(Message::SelectCard(card.id.clone()))
    };

    // Scope & signals row (stagnation warning, recurrence badge, effort minutes)
    let mut signals_row = widget::row::with_capacity(5)
        .push(make_pill_badge(
            card.scope_id.clone(),
            Color::from_rgb(0.45, 0.52, 0.64),
        ))
        .push(widget::space::horizontal().width(Length::Fill))
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    if let Some(days) = card.stagnant_days {
        signals_row = signals_row.push(make_pill_badge(
            format!("⚠ {days}d"),
            Color::from_rgb(0.92, 0.60, 0.18),
        ));
    }

    if card.recurrence_count > 0 {
        signals_row = signals_row.push(make_pill_badge(
            format!("↻ {}", card.recurrence_count),
            Color::from_rgb(0.88, 0.30, 0.42),
        ));
    }

    if card.logged_time_minutes > 0 {
        signals_row = signals_row.push(make_pill_badge(
            format!("{}m", card.logged_time_minutes),
            Color::from_rgb(0.32, 0.68, 0.54),
        ));
    }

    // Bottom quick-action bar: Open Details, Timer toggle, Quick Status transition
    let inspect_btn = widget::button::standard(if is_selected { "Selected" } else { "Inspect" })
        .padding([2, 8])
        .height(Length::Fixed(24.0))
        .on_press(Message::SelectCard(card.id.clone()));

    let timer_btn = if is_timer_running {
        widget::button::destructive("■ Timer")
            .padding([2, 8])
            .height(Length::Fixed(24.0))
            .on_press(Message::StopTimer)
    } else {
        widget::button::standard("⏱ Start")
            .padding([2, 8])
            .height(Length::Fixed(24.0))
            .on_press(Message::StartTimer(card.id.clone()))
    };

    let (next_status, next_label) = next_status_transition(card.status);
    let advance_btn = widget::button::standard(next_label)
        .padding([2, 8])
        .height(Length::Fixed(24.0))
        .on_press(Message::QuickStatusTransition {
            item_id: card.id.clone(),
            new_status: next_status,
        });

    let actions_row = widget::row::with_capacity(4)
        .push(inspect_btn)
        .push(timer_btn)
        .push(widget::space::horizontal().width(Length::Fill))
        .push(advance_btn)
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let card_col = widget::column::with_capacity(6)
        .push(top_row)
        .push(select_btn)
        .push(widget::divider::horizontal::light())
        .push(signals_row)
        .push(actions_row)
        .spacing(8)
        .width(Length::Fill);

    let border_color = if is_selected {
        Color::from_rgb(0.35, 0.72, 0.98)
    } else if card.recurrence_count > 0 {
        Color::from_rgb(0.88, 0.32, 0.42)
    } else if card.stagnant_days.is_some() {
        Color::from_rgb(0.90, 0.58, 0.18)
    } else {
        severity_color(card.severity)
    };
    let border_width = if is_selected { 2.0 } else { 1.0 };

    widget::container(card_col)
        .padding([10, 12])
        .width(Length::Fill)
        .class(cosmic::theme::Container::custom(move |theme| {
            let cosmic = theme.cosmic();
            cosmic::iced::widget::container::Style {
                background: Some(Background::Color(
                    cosmic.secondary(theme.transparent).base.into(),
                )),
                border: Border {
                    color: border_color,
                    width: border_width,
                    radius: cosmic.corner_radii.radius_s.into(),
                },
                ..Default::default()
            }
        }))
        .into()
}

fn make_pill_badge<'a>(text: String, accent: Color) -> Element<'a, Message> {
    widget::container(widget::text::caption(text))
        .padding([2, 7])
        .class(cosmic::theme::Container::custom(move |theme| {
            let cosmic = theme.cosmic();
            cosmic::iced::widget::container::Style {
                background: Some(Background::Color(Color::from_rgba(
                    accent.r, accent.g, accent.b, 0.22,
                ))),
                border: Border {
                    color: accent,
                    width: 1.0,
                    radius: cosmic.corner_radii.radius_xs.into(),
                },
                ..Default::default()
            }
        }))
        .into()
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Low => "LOW",
        Severity::Medium => "MED",
        Severity::High => "HIGH",
        Severity::Critical => "CRIT",
    }
}

fn severity_color(severity: Severity) -> Color {
    match severity {
        Severity::Low => Color::from_rgb(0.42, 0.62, 0.78),
        Severity::Medium => Color::from_rgb(0.32, 0.68, 0.52),
        Severity::High => Color::from_rgb(0.92, 0.56, 0.20),
        Severity::Critical => Color::from_rgb(0.90, 0.26, 0.28),
    }
}

fn column_accent_color(column: KanbanColumn) -> Color {
    match column {
        KanbanColumn::Open => Color::from_rgb(0.34, 0.64, 0.94),
        KanbanColumn::InProgress => Color::from_rgb(0.94, 0.66, 0.22),
        KanbanColumn::Pending => Color::from_rgb(0.68, 0.48, 0.90),
        KanbanColumn::ClosedOrMerged => Color::from_rgb(0.28, 0.74, 0.52),
    }
}

fn next_status_transition(current: ItemStatus) -> (ItemStatus, &'static str) {
    match current {
        ItemStatus::Open => (ItemStatus::InProgress, "→ Start"),
        ItemStatus::InProgress => (ItemStatus::Closed, "✓ Close"),
        ItemStatus::Pending => (ItemStatus::InProgress, "→ Resume"),
        ItemStatus::Closed | ItemStatus::Merged => (ItemStatus::Open, "↻ Reopen"),
    }
}
