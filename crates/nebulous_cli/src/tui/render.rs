use backend_core::{ItemStatus, Severity, CANONICAL_PHASES};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Wrap};
use ratatui::Frame;

use crate::tui::state::{
    input_scroll_window, ActiveModal, CloseField, CloseModalState, CreateField, CreateModalState,
    MoveModalState, PaneFocus, TuiApp,
};
use crate::tui::theme::{
    select_nebula_banner, style_alert, style_base, style_border, style_bright, style_dark,
    style_dim, style_selected, AMBER_ALERT, AMBER_BRIGHT, AMBER_DIM, AMBER_PRIMARY, BG_BLACK,
    PANEL_BG,
};

/// Minimum locked terminal width before the non-deforming scaling guard engages.
pub const MIN_TERM_WIDTH: u16 = 64;
/// Minimum locked terminal height before the non-deforming scaling guard engages.
pub const MIN_TERM_HEIGHT: u16 = 20;
/// Width threshold for the 3-pane wide workspace layout (`>= 110` cols).
pub const WIDE_LAYOUT_MIN_WIDTH: u16 = 110;

/// Computes a centered modal `Rect` clamped with minimum and maximum width/height locks
/// so modal fields are never crushed by naive percentage sizing on compact screens.
pub fn locked_centered_rect(min_width: u16, min_height: u16, max_width: u16, area: Rect) -> Rect {
    let target_w = max_width.min(area.width).max(min_width.min(area.width));
    let target_h = min_height.min(area.height);
    let x = area.x + area.width.saturating_sub(target_w) / 2;
    let y = area.y + area.height.saturating_sub(target_h) / 2;
    Rect::new(x, y, target_w, target_h)
}

/// Paints pure CRT obsidian (`BG_BLACK`) across every cell of `area` so host terminal
/// colors never bleed through.
fn paint_crt_background(frame: &mut Frame, area: Rect, bg_style: Style) {
    let buf = frame.buffer_mut();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_style(bg_style);
            }
        }
    }
}

/// Top-level frame renderer for the retro Amber-on-Black Nebulous TUI.
pub fn render_frame(frame: &mut Frame, app: &mut TuiApp) {
    let area = frame.area();
    paint_crt_background(frame, area, style_base());

    if area.width < MIN_TERM_WIDTH || area.height < MIN_TERM_HEIGHT {
        render_scaling_lock_guard(frame, app, area);
        return;
    }

    let banner = select_nebula_banner(area.width, area.height);
    let header_height = banner.height + 2;

    let root_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(9),
            Constraint::Length(1),
        ])
        .split(area);

    render_nebula_header(frame, banner, root_chunks[0]);
    render_stage_and_timer_bar(frame, app, root_chunks[1]);
    render_whats_next_bar(frame, app, root_chunks[2]);
    render_main_workspace(frame, app, root_chunks[3]);
    render_footer_bar(frame, app, root_chunks[4]);

    if let Some(modal) = app.modal.clone() {
        match modal {
            ActiveModal::Create(ref create) => render_create_modal(frame, create, area),
            ActiveModal::Close(ref close) => render_close_modal(frame, close, area),
            ActiveModal::Move(ref mv) => render_move_modal(frame, mv, area),
        }
    }
}

/// Renders the non-deforming minimum-size scaling lock view when the terminal is resized below `64×20`.
pub fn render_scaling_lock_guard(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " [ SCALING LOCK GUARD — AMBER CRT ] ",
            style_alert(),
        ))
        .borders(Borders::ALL)
        .border_style(style_alert())
        .style(style_base());

    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let compact = select_nebula_banner(60, 20);
    let active_focus = app
        .active_timer
        .as_ref()
        .map(|t| format!("⏱ {} ({}m)", t.timer.item_id, t.elapsed_minutes))
        .or_else(|| app.whats_next.top_item_id().map(|id| format!("Next: {id}")))
        .unwrap_or_else(|| "Idle".to_string());

    let mut lines = Vec::new();
    for line in compact.lines {
        lines.push(line);
    }
    lines.push(Line::from(Span::styled(
        "[ MINIMUM VIEWPORT SCALING LOCK ENGAGED ]",
        style_alert(),
    )));
    lines.push(Line::from(Span::styled(
        format!(
            "Current: {}x{} | Minimum Locked: {}x{}",
            area.width, area.height, MIN_TERM_WIDTH, MIN_TERM_HEIGHT
        ),
        style_bright(),
    )));
    lines.push(Line::from(Span::styled(
        format!(
            "Stage: {} | Scope: {} | {}",
            app.active_stage, app.inferred_scope, active_focus
        ),
        style_base(),
    )));
    lines.push(Line::from(Span::styled(
        "Resize terminal to >= 64x20 to unlock full workspace.",
        style_dim(),
    )));

    // Wrap is strictly disabled so the ASCII Nebula header never deforms.
    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .style(style_base());
    frame.render_widget(paragraph, inner);
}

fn render_nebula_header(frame: &mut Frame, banner: crate::tui::theme::NebulaBanner, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(AMBER_DIM).bg(BG_BLACK))
        .style(style_base());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Strictly centered with Wrap disabled so the ASCII Nebula + NEBULOUS art never clips or deforms.
    let header_para = Paragraph::new(banner.lines)
        .alignment(Alignment::Center)
        .style(style_base());
    frame.render_widget(header_para, inner);
}

fn render_stage_and_timer_bar(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let mut spans = Vec::new();
    spans.push(Span::styled(" STAGES: ", style_dim()));

    for (idx, (stage_id, stage_title)) in CANONICAL_PHASES.iter().enumerate() {
        let num = idx + 1;
        let is_active = app.active_stage == *stage_id;
        if idx > 0 {
            spans.push(Span::styled(" │ ", style_dark()));
        }
        if is_active {
            spans.push(Span::styled(
                format!("▶ [{num}] {stage_title} ◀"),
                style_bright(),
            ));
        } else {
            spans.push(Span::styled(format!("[{num}] {stage_title}"), style_dim()));
        }
    }

    spans.push(Span::styled("   ║   ", style_dark()));
    spans.push(Span::styled("Scope: ", style_dim()));
    spans.push(Span::styled(app.inferred_scope.clone(), style_base()));

    spans.push(Span::styled("   ║   ", style_dark()));
    if let Some(ref timer) = app.active_timer {
        spans.push(Span::styled(
            format!("⏱ {} {}m", timer.timer.item_id, timer.elapsed_minutes),
            style_alert(),
        ));
    } else {
        spans.push(Span::styled("⏱ IDLE", style_dim()));
    }

    let divider = "─".repeat(area.width as usize);
    let lines = vec![
        Line::from(spans),
        Line::from(Span::styled(divider, style_dark())),
    ];

    let para = Paragraph::new(lines).style(style_base());
    frame.render_widget(para, area);
}

fn render_whats_next_bar(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let next_summary = if let Some(ref hint) = app.whats_next.stage_advancement_hint {
        hint.clone()
    } else if let Some(rec) = app.whats_next.recommendations.first() {
        format!("{} ({})", rec.summary, rec.suggested_command)
    } else {
        "All clear in active stage — press [b] New Bug or [f] New Feature.".to_string()
    };

    let divider = "═".repeat(area.width as usize);
    let lines = vec![
        Line::from(vec![
            Span::styled(" ✦ WHAT'S NEXT: ", style_bright()),
            Span::styled(next_summary, style_base()),
        ]),
        Line::from(Span::styled(divider, style_dark())),
    ];
    let para = Paragraph::new(lines).style(style_base());
    frame.render_widget(para, area);
}

fn render_main_workspace(frame: &mut Frame, app: &mut TuiApp, area: Rect) {
    if area.width >= WIDE_LAYOUT_MIN_WIDTH {
        // Wide Mode (`width >= 110`): 3-pane horizontal layout with minimum width locks
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(26),
                Constraint::Min(44),
                Constraint::Min(38),
            ])
            .split(area);

        render_scopes_pane(frame, app, panes[0]);
        render_items_pane(frame, app, panes[1]);
        render_details_pane(frame, app, panes[2]);
    } else {
        // Medium Mode (`64 <= width < 110`): 2-pane top row + bottom inspector drawer
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(6), Constraint::Min(7)])
            .split(area);

        let top_panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(22), Constraint::Min(40)])
            .split(rows[0]);

        render_scopes_pane(frame, app, top_panes[0]);
        render_items_pane(frame, app, top_panes[1]);
        render_details_pane(frame, app, rows[1]);
    }
}

fn render_scopes_pane(frame: &mut Frame, app: &mut TuiApp, area: Rect) {
    let focused = app.focus == PaneFocus::Scopes && app.modal.is_none();
    let block = Block::default()
        .title(Span::styled(
            " Domain Scopes ",
            if focused {
                style_bright()
            } else {
                style_base()
            },
        ))
        .borders(Borders::ALL)
        .border_style(style_border(focused))
        .style(style_base());

    let list_items: Vec<ListItem> = app
        .scopes
        .iter()
        .enumerate()
        .map(|(idx, sc)| {
            let prefix = if idx == app.selected_scope_idx {
                "▸ "
            } else {
                "  "
            };
            let text = format!(
                "{}{} ({}/{})",
                prefix, sc.label, sc.open_items, sc.total_items
            );
            let st = if idx == app.selected_scope_idx {
                style_selected()
            } else if sc.scope_id.as_deref() == Some(app.inferred_scope.as_str()) {
                style_bright()
            } else {
                style_base()
            };
            ListItem::new(Line::from(Span::styled(text, st)))
        })
        .collect();

    let list = List::new(list_items)
        .block(block)
        .highlight_style(style_selected());
    frame.render_stateful_widget(list, area, &mut app.scope_list_state);
}

fn render_items_pane(frame: &mut Frame, app: &mut TuiApp, area: Rect) {
    let focused = app.focus == PaneFocus::Items && app.modal.is_none();
    let title = if app.filter_query.is_empty() && !app.filter_editing {
        format!(" Stage Items ({}) ", app.items.len())
    } else {
        format!(
            " Stage Items ({}) [Filter: {}{}] ",
            app.items.len(),
            app.filter_query,
            if app.filter_editing { "█" } else { "" }
        )
    };

    let block = Block::default()
        .title(Span::styled(
            title,
            if focused {
                style_bright()
            } else {
                style_base()
            },
        ))
        .borders(Borders::ALL)
        .border_style(style_border(focused))
        .style(style_base());

    let header = Row::new(vec![
        Cell::from(Span::styled("ID", style_bright())),
        Cell::from(Span::styled("Sev", style_bright())),
        Cell::from(Span::styled("Status", style_bright())),
        Cell::from(Span::styled("Title", style_bright())),
    ])
    .style(Style::default().fg(AMBER_BRIGHT).bg(BG_BLACK))
    .bottom_margin(0);

    let rows: Vec<Row> = if app.items.is_empty() {
        vec![Row::new(vec![
            Cell::from(Span::styled("-", style_dim())),
            Cell::from(Span::styled("-", style_dim())),
            Cell::from(Span::styled("empty", style_dim())),
            Cell::from(Span::styled(
                "No items in this view — press [b] Bug or [f] Feature",
                style_dim(),
            )),
        ])]
    } else {
        app.items
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let is_selected = idx == app.selected_item_idx;
                let sev_label = match item.severity {
                    Severity::Critical => "CRIT",
                    Severity::High => "HIGH",
                    Severity::Medium => "MED",
                    Severity::Low => "LOW",
                };
                let row_bg = if is_selected { PANEL_BG } else { BG_BLACK };
                let sev_style = match item.severity {
                    Severity::Critical | Severity::High => style_alert().bg(row_bg),
                    Severity::Medium => style_base().bg(row_bg),
                    Severity::Low => style_dim().bg(row_bg),
                };
                let status_style = match item.status {
                    ItemStatus::InProgress => style_bright().bg(row_bg),
                    ItemStatus::Open => style_base().bg(row_bg),
                    ItemStatus::Pending => style_dim().bg(row_bg),
                    ItemStatus::Closed | ItemStatus::Merged => style_dim().bg(row_bg),
                };
                let mut title_str = item.title.clone();
                if item.recurrence_count > 0 {
                    title_str.push_str(&format!(" [↻{}]", item.recurrence_count));
                }
                let row_style = if is_selected {
                    style_selected()
                } else {
                    style_base()
                };

                Row::new(vec![
                    Cell::from(Span::styled(item.id.clone(), row_style)),
                    Cell::from(Span::styled(sev_label, sev_style)),
                    Cell::from(Span::styled(item.status.to_string(), status_style)),
                    Cell::from(Span::styled(title_str, row_style)),
                ])
                .style(row_style)
            })
            .collect()
    };

    // Column width locks: ID Length(9), Sev Length(6), Status Length(11), Title Min(18)
    let widths = [
        Constraint::Length(9),
        Constraint::Length(6),
        Constraint::Length(11),
        Constraint::Min(18),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(style_selected());

    frame.render_stateful_widget(table, area, &mut app.item_table_state);
}

fn render_details_pane(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let focused = app.focus == PaneFocus::Details && app.modal.is_none();
    let block = Block::default()
        .title(Span::styled(
            " Inspector & Bounded Agent Context [J/K Scroll] ",
            if focused {
                style_bright()
            } else {
                style_base()
            },
        ))
        .borders(Borders::ALL)
        .border_style(style_border(focused))
        .style(style_base());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = Vec::new();
    if let Some((ref loc, ref detail)) = app.selected_item_detail {
        let meta = &detail.manifest.item;
        let effort = &detail.manifest.effort;
        let hist = &detail.manifest.history;

        lines.push(Line::from(vec![
            Span::styled(format!("{} ", meta.id), style_bright()),
            Span::styled(
                format!("[{} / {}] ", meta.item_type, meta.severity),
                style_alert(),
            ),
            Span::styled(meta.title.clone(), style_base()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Stage: ", style_dim()),
            Span::styled(format!("{}  ", loc.phase_id), style_base()),
            Span::styled("Scope: ", style_dim()),
            Span::styled(format!("{}  ", loc.scope_id), style_base()),
            Span::styled("Effort: ", style_dim()),
            Span::styled(
                format!(
                    "{}m ({} iter, {} recur)",
                    effort.logged_time_minutes, effort.iteration_count, hist.recurrence_count
                ),
                style_base(),
            ),
        ]));

        if !hist.previous_fixes.is_empty() {
            lines.push(Line::from(Span::styled(
                "── Previous Fixes Ledger ──",
                style_dim(),
            )));
            for fix in &hist.previous_fixes {
                lines.push(Line::from(Span::styled(
                    format!("• [{}] {} — {}", fix.date, fix.commit, fix.summary),
                    style_base(),
                )));
            }
        }

        lines.push(Line::from(Span::styled(
            "── Specification (desc.md) ──",
            style_dim(),
        )));
        for raw_line in detail.description_markdown.lines() {
            lines.push(Line::from(Span::styled(raw_line.to_string(), style_base())));
        }

        if let Some(ref prompt) = app.selected_agent_prompt {
            lines.push(Line::from(Span::styled(
                "── Bounded Agent Context ([c] Copy) ──",
                style_dim(),
            )));
            for raw_line in prompt.lines() {
                lines.push(Line::from(Span::styled(raw_line.to_string(), style_base())));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "No item selected in current stage/scope.",
            style_dim(),
        )));
        lines.push(Line::from(Span::styled(
            "Press [b] to capture a Bug, [f] to capture a Feature, or [1/2/3] to switch stages.",
            style_base(),
        )));
    }

    let para = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll, 0))
        .style(style_base());
    frame.render_widget(para, inner);
}

fn render_footer_bar(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let line = if let Some(ref msg) = app.status_message {
        let msg_style = if msg.contains("[REQUIRED FIELD LOCK]") {
            style_alert()
        } else {
            style_bright()
        };
        Line::from(vec![
            Span::styled(" STATUS: ", style_dim()),
            Span::styled(msg.clone(), msg_style),
            Span::styled(
                "  │  [1/2/3] Stage [b] Bug [f] Feat [s] Start [d] Done [m] Move [q] Quit",
                style_dim(),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(" [1/2/3]", style_bright()),
            Span::styled(" Stage ", style_base()),
            Span::styled("[b]", style_bright()),
            Span::styled(" Bug ", style_base()),
            Span::styled("[f]", style_bright()),
            Span::styled(" Feat ", style_base()),
            Span::styled("[s]", style_bright()),
            Span::styled(" Start ", style_base()),
            Span::styled("[d]", style_bright()),
            Span::styled(" Done ", style_base()),
            Span::styled("[m]", style_bright()),
            Span::styled(" Move ", style_base()),
            Span::styled("[c]", style_bright()),
            Span::styled(" Context ", style_base()),
            Span::styled("[i]", style_bright()),
            Span::styled(" Ingest ", style_base()),
            Span::styled("[n]", style_bright()),
            Span::styled(" Next ", style_base()),
            Span::styled("[/]", style_bright()),
            Span::styled(" Filter ", style_base()),
            Span::styled("[q]", style_bright()),
            Span::styled(" Quit", style_base()),
        ])
    };

    let para = Paragraph::new(line).style(style_base());
    frame.render_widget(para, area);
}

/// Renders the `New Bug` / `New Feature` creation modal with dedicated locked rows
/// (`Constraint::Length(3)` per single-line field, `Constraint::Min(4)` for description),
/// horizontal cursor-window scrolling, and inline `[REQUIRED FIELD LOCK]` validation alerts.
pub fn render_create_modal(frame: &mut Frame, create: &CreateModalState, area: Rect) {
    let modal_rect = locked_centered_rect(68, 22, 78, area);
    frame.render_widget(Clear, modal_rect);
    paint_crt_background(
        frame,
        modal_rect,
        Style::default().fg(AMBER_PRIMARY).bg(PANEL_BG),
    );

    let title_str = format!(
        " ✦ NEW {} CAPTURE (PREDICTIVE ASSIST) ✦ ",
        create.item_type.label()
    );

    let outer_block = Block::default()
        .title(Span::styled(title_str, style_bright()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(AMBER_PRIMARY).bg(PANEL_BG))
        .style(Style::default().fg(AMBER_PRIMARY).bg(PANEL_BG));

    let inner = outer_block.inner(modal_rect);
    frame.render_widget(outer_block, modal_rect);

    // Determine visible field window if terminal height is very tight (< 22 rows)
    let all_fields = CreateField::ALL;
    let visible_fields: &[CreateField] = if inner.height >= 18 {
        &all_fields
    } else {
        let max_fields =
            ((inner.height.saturating_sub(2)) / 3).clamp(1, all_fields.len() as u16) as usize;
        let focused_idx = create.focused_field.index();
        let half = max_fields / 2;
        let start = focused_idx
            .saturating_sub(half)
            .min(all_fields.len().saturating_sub(max_fields));
        let end = (start + max_fields).min(all_fields.len());
        &all_fields[start..end]
    };

    let mut constraints = Vec::with_capacity(visible_fields.len() + 1);
    for field in visible_fields {
        match field {
            CreateField::Description => constraints.push(Constraint::Min(4)),
            _ => constraints.push(Constraint::Length(3)),
        }
    }
    constraints.push(Constraint::Length(2));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let field_inner_width = inner.width.saturating_sub(4) as usize;

    for (idx, field) in visible_fields.iter().enumerate() {
        let is_focused = create.focused_field == *field;
        let is_locked = create.required_lock_field == Some(*field);

        let (label, raw_value) = match field {
            CreateField::Title => (
                if is_locked {
                    " [1] Title *REQUIRED* [REQUIRED FIELD LOCK] "
                } else {
                    " [1] Title *REQUIRED* "
                },
                create.title.clone(),
            ),
            CreateField::Scope => (
                if is_locked {
                    " [2] Scope *REQUIRED* [REQUIRED FIELD LOCK] "
                } else {
                    " [2] Scope *REQUIRED* (←/→ or type) "
                },
                create.scope.clone(),
            ),
            CreateField::Stage => (
                if is_locked {
                    " [3] Stage *REQUIRED* [REQUIRED FIELD LOCK] "
                } else {
                    " [3] Stage *REQUIRED* (←/→ or type) "
                },
                create.stage.clone(),
            ),
            CreateField::Severity => (
                " [4] Severity *REQUIRED* (←/→ or 1-4) ",
                format!(
                    "{}  ( [1] low  [2] medium  [3] high  [4] critical )",
                    create.severity
                ),
            ),
            CreateField::Description => (
                " [5] Description (optional Markdown) ",
                create.description.clone(),
            ),
        };

        let border_style = if is_locked {
            style_alert().bg(PANEL_BG)
        } else if is_focused {
            Style::default()
                .fg(AMBER_BRIGHT)
                .bg(PANEL_BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(AMBER_DIM).bg(PANEL_BG)
        };

        let (scrolled_val, offset) =
            input_scroll_window(&raw_value, field_inner_width.saturating_sub(2));
        let display_val = if is_locked && raw_value.trim().is_empty() {
            "[REQUIRED FIELD LOCK] Enter non-empty value".to_string()
        } else if is_focused {
            if offset > 0 {
                format!("«{scrolled_val}█")
            } else {
                format!("{scrolled_val}█")
            }
        } else {
            scrolled_val
        };

        let val_style = if is_locked {
            style_alert().bg(PANEL_BG)
        } else if is_focused {
            Style::default()
                .fg(AMBER_BRIGHT)
                .bg(PANEL_BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(AMBER_PRIMARY).bg(PANEL_BG)
        };

        let field_block = Block::default()
            .title(Span::styled(label, border_style))
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Style::default().fg(AMBER_PRIMARY).bg(PANEL_BG));

        let field_para =
            Paragraph::new(Line::from(Span::styled(display_val, val_style))).block(field_block);
        frame.render_widget(field_para, chunks[idx]);
    }

    let footer_chunk = chunks[visible_fields.len()];
    let footer_line = if create.required_lock_field.is_some() {
        Line::from(vec![
            Span::styled(
                " [REQUIRED FIELD LOCK] ",
                Style::default()
                    .fg(AMBER_ALERT)
                    .bg(PANEL_BG)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Complete required field before pressing [Enter] │ [Esc] Cancel",
                Style::default().fg(AMBER_BRIGHT).bg(PANEL_BG),
            ),
        ])
    } else {
        Line::from(vec![Span::styled(
            " [Tab/↑/↓] Field  [←/→] Cycle Scope/Stage/Sev  [Enter] Create  [Esc] Cancel",
            Style::default().fg(AMBER_DIM).bg(PANEL_BG),
        )])
    };
    frame.render_widget(
        Paragraph::new(footer_line).style(Style::default().fg(AMBER_PRIMARY).bg(PANEL_BG)),
        footer_chunk,
    );
}

/// Renders the `Done / Close` modal with pre-filled summary, Git `HEAD` SHA, and `[REQUIRED FIELD LOCK]`.
pub fn render_close_modal(frame: &mut Frame, close: &CloseModalState, area: Rect) {
    let modal_rect = locked_centered_rect(62, 14, 74, area);
    frame.render_widget(Clear, modal_rect);
    paint_crt_background(
        frame,
        modal_rect,
        Style::default().fg(AMBER_PRIMARY).bg(PANEL_BG),
    );

    let panel_base = Style::default().fg(AMBER_PRIMARY).bg(PANEL_BG);
    let panel_bright = Style::default()
        .fg(AMBER_BRIGHT)
        .bg(PANEL_BG)
        .add_modifier(Modifier::BOLD);
    let panel_dim = Style::default().fg(AMBER_DIM).bg(PANEL_BG);
    let panel_alert = Style::default()
        .fg(AMBER_ALERT)
        .bg(PANEL_BG)
        .add_modifier(Modifier::BOLD);

    let outer_block = Block::default()
        .title(Span::styled(
            format!(" ✦ COMPLETE / CLOSE {} ✦ ", close.item_id),
            panel_bright,
        ))
        .borders(Borders::ALL)
        .border_style(panel_base)
        .style(panel_base);

    let inner = outer_block.inner(modal_rect);
    frame.render_widget(outer_block, modal_rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .split(inner);

    let field_width = inner.width.saturating_sub(4) as usize;

    // 1. Summary *REQUIRED*
    let sum_locked = close.required_lock_field == Some(CloseField::Summary);
    let sum_focused = close.focused_field == CloseField::Summary;
    let sum_style = if sum_locked {
        panel_alert
    } else if sum_focused {
        panel_bright
    } else {
        panel_dim
    };
    let max_sum_chars = field_width.saturating_sub(2);
    let (sum_win, sum_offset) = if !close.summary_edited {
        (
            close
                .summary
                .chars()
                .take(max_sum_chars)
                .collect::<String>(),
            0,
        )
    } else {
        input_scroll_window(&close.summary, max_sum_chars)
    };
    let sum_text = if sum_locked && close.summary.trim().is_empty() {
        "[REQUIRED FIELD LOCK] Summary cannot be empty".to_string()
    } else if sum_focused {
        if sum_offset > 0 {
            format!("«{sum_win}█")
        } else {
            format!("{sum_win}█")
        }
    } else {
        sum_win
    };
    let sum_block = Block::default()
        .title(Span::styled(
            if sum_locked {
                " [1] Fix Summary *REQUIRED* [REQUIRED FIELD LOCK] "
            } else {
                " [1] Fix Summary *REQUIRED* "
            },
            sum_style,
        ))
        .borders(Borders::ALL)
        .border_style(sum_style)
        .style(panel_base);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(sum_text, sum_style)))
            .block(sum_block)
            .style(panel_base),
        chunks[0],
    );

    // 2. Commit SHA
    let sha_focused = close.focused_field == CloseField::Commit;
    let sha_style = if sha_focused { panel_bright } else { panel_dim };
    let (sha_win, sha_offset) = input_scroll_window(&close.commit, field_width.saturating_sub(2));
    let sha_block = Block::default()
        .title(Span::styled(" [2] Commit SHA (Git HEAD) ", sha_style))
        .borders(Borders::ALL)
        .border_style(sha_style)
        .style(panel_base);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            if sha_focused {
                if sha_offset > 0 {
                    format!("«{sha_win}█")
                } else {
                    format!("{sha_win}█")
                }
            } else {
                sha_win
            },
            panel_base,
        )))
        .block(sha_block)
        .style(panel_base),
        chunks[1],
    );

    // 3. Mode (Closed vs Merged)
    let mode_focused = close.focused_field == CloseField::Mode;
    let mode_style = if mode_focused {
        panel_bright
    } else {
        panel_dim
    };
    let mode_label = if close.merged {
        "Status -> MERGED (press Space/←/→ to toggle CLOSED)"
    } else {
        "Status -> CLOSED (press Space/←/→ to toggle MERGED)"
    };
    let mode_block = Block::default()
        .title(Span::styled(" [3] Completion Status ", mode_style))
        .borders(Borders::ALL)
        .border_style(mode_style)
        .style(panel_base);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(mode_label, panel_base)))
            .block(mode_block)
            .style(panel_base),
        chunks[2],
    );

    let footer = Line::from(Span::styled(
        " [Tab/↑/↓] Field  [Enter] Confirm Close  [Esc] Cancel",
        panel_dim,
    ));
    frame.render_widget(Paragraph::new(footer).style(panel_base), chunks[3]);
}

/// Renders the 1-key stage move modal (`[1] Pre-Production`, `[2] Production`, `[3] Post-Production`).
pub fn render_move_modal(frame: &mut Frame, mv: &MoveModalState, area: Rect) {
    let modal_rect = locked_centered_rect(56, 11, 68, area);
    frame.render_widget(Clear, modal_rect);
    let panel_base = Style::default().fg(AMBER_PRIMARY).bg(PANEL_BG);
    let panel_bright = Style::default()
        .fg(AMBER_BRIGHT)
        .bg(PANEL_BG)
        .add_modifier(Modifier::BOLD);
    let panel_dim = Style::default().fg(AMBER_DIM).bg(PANEL_BG);

    paint_crt_background(frame, modal_rect, panel_base);

    let outer_block = Block::default()
        .title(Span::styled(
            format!(" ✦ MOVE {} TO STAGE ✦ ", mv.item_id),
            panel_bright,
        ))
        .borders(Borders::ALL)
        .border_style(panel_base)
        .style(panel_base);

    let inner = outer_block.inner(modal_rect);
    frame.render_widget(outer_block, modal_rect);

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        format!("Current Stage: {} — {}", mv.current_stage, mv.item_title),
        panel_dim,
    )));
    lines.push(Line::from(""));

    for (idx, (stage_id, stage_title)) in CANONICAL_PHASES.iter().enumerate() {
        let num = idx + 1;
        let is_selected = idx == mv.selected_stage_idx;
        let prefix = if is_selected { "▶ " } else { "  " };
        let st = if is_selected {
            panel_bright
        } else {
            panel_base
        };
        lines.push(Line::from(Span::styled(
            format!("{prefix}[{num}] {stage_title} ({stage_id})"),
            st,
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press [1/2/3] or [Enter] to Move │ [Esc] Cancel",
        panel_dim,
    )));

    frame.render_widget(Paragraph::new(lines).style(panel_base), inner);
}
