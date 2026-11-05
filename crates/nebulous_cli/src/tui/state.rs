use std::path::PathBuf;

use backend_core::{
    resolve_phase_alias, CreateItemInput, IngestionOptions, ItemDetail, ItemLocation, ItemStatus,
    ItemType, Result, Severity, TimerStatus, TrackerStorage, WhatsNextReport, CANONICAL_PHASES,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::{ListState, TableState};

/// Active pane focus in the main TUI workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneFocus {
    Scopes,
    Items,
    Details,
}

impl PaneFocus {
    pub fn next(self) -> Self {
        match self {
            Self::Scopes => Self::Items,
            Self::Items => Self::Details,
            Self::Details => Self::Scopes,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Scopes => Self::Details,
            Self::Items => Self::Scopes,
            Self::Details => Self::Items,
        }
    }
}

/// Focused field inside the `New Bug` / `New Feature` modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateField {
    Title,
    Scope,
    Stage,
    Severity,
    Description,
}

impl CreateField {
    pub const ALL: [Self; 5] = [
        Self::Title,
        Self::Scope,
        Self::Stage,
        Self::Severity,
        Self::Description,
    ];

    pub fn index(self) -> usize {
        match self {
            Self::Title => 0,
            Self::Scope => 1,
            Self::Stage => 2,
            Self::Severity => 3,
            Self::Description => 4,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Title => Self::Scope,
            Self::Scope => Self::Stage,
            Self::Stage => Self::Severity,
            Self::Severity => Self::Description,
            Self::Description => Self::Title,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Title => Self::Description,
            Self::Scope => Self::Title,
            Self::Stage => Self::Scope,
            Self::Severity => Self::Stage,
            Self::Description => Self::Severity,
        }
    }
}

/// Predictive creation modal state (`b` for Bug, `f` for Feature).
#[derive(Debug, Clone)]
pub struct CreateModalState {
    pub item_type: ItemType,
    pub title: String,
    pub scope: String,
    pub stage: String,
    pub severity: Severity,
    pub description: String,
    pub focused_field: CreateField,
    pub required_lock_field: Option<CreateField>,
}

/// Focused field inside the `Done / Close` modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseField {
    Summary,
    Commit,
    Mode,
}

impl CloseField {
    pub fn next(self) -> Self {
        match self {
            Self::Summary => Self::Commit,
            Self::Commit => Self::Mode,
            Self::Mode => Self::Summary,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Summary => Self::Mode,
            Self::Commit => Self::Summary,
            Self::Mode => Self::Commit,
        }
    }
}

/// Close/merge item modal state (`d`).
#[derive(Debug, Clone)]
pub struct CloseModalState {
    pub item_id: String,
    pub item_title: String,
    pub summary: String,
    pub commit: String,
    pub merged: bool,
    pub focused_field: CloseField,
    pub required_lock_field: Option<CloseField>,
    pub summary_edited: bool,
}

/// 1-key stage move modal state (`m`).
#[derive(Debug, Clone)]
pub struct MoveModalState {
    pub item_id: String,
    pub item_title: String,
    pub current_stage: String,
    pub selected_stage_idx: usize,
}

/// Active modal overlay in the TUI.
#[derive(Debug, Clone)]
pub enum ActiveModal {
    Create(CreateModalState),
    Close(CloseModalState),
    Move(MoveModalState),
}

/// Row entry in the left Scopes menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeRow {
    /// `None` represents `"ALL SCOPES"` across the active stage.
    pub scope_id: Option<String>,
    pub label: String,
    pub target: String,
    pub total_items: usize,
    pub open_items: usize,
}

/// Row entry in the center Items table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRow {
    pub id: String,
    pub item_type: ItemType,
    pub title: String,
    pub status: ItemStatus,
    pub severity: Severity,
    pub phase_id: String,
    pub scope_id: String,
    pub recurrence_count: u32,
}

/// Computes a horizontally scrolled visible window for an input string so long values
/// (including multi-byte UTF-8 and double-width Unicode glyphs) never clip off-screen
/// inside a fixed-width modal field.
pub fn input_scroll_window(text: &str, max_visible_chars: usize) -> (String, usize) {
    if max_visible_chars == 0 {
        return (String::new(), 0);
    }
    let total_width = ratatui::text::Span::raw(text).width();
    if total_width <= max_visible_chars {
        return (text.to_string(), 0);
    }
    let chars: Vec<char> = text.chars().collect();
    let mut used_cols = 0usize;
    let mut start_idx = chars.len();
    while start_idx > 0 {
        let ch = chars[start_idx - 1];
        let mut buf = [0u8; 4];
        let ch_str = ch.encode_utf8(&mut buf);
        let ch_width = ratatui::text::Span::raw(&*ch_str).width().max(1);
        if used_cols + ch_width > max_visible_chars {
            break;
        }
        used_cols += ch_width;
        start_idx -= 1;
    }
    let visible: String = chars[start_idx..].iter().collect();
    (visible, start_idx)
}

/// Stateful controller for the retro Amber-on-Black Nebulous TUI.
pub struct TuiApp {
    pub storage: TrackerStorage,
    pub cwd: Option<PathBuf>,
    pub project_name: String,
    pub active_stage: String,
    pub inferred_scope: String,
    pub available_scopes: Vec<String>,
    pub scopes: Vec<ScopeRow>,
    pub selected_scope_idx: usize,
    pub scope_list_state: ListState,
    pub items: Vec<ItemRow>,
    pub selected_item_idx: usize,
    pub item_table_state: TableState,
    pub selected_item_detail: Option<(ItemLocation, ItemDetail)>,
    pub selected_agent_prompt: Option<String>,
    pub detail_scroll: u16,
    pub focus: PaneFocus,
    pub whats_next: WhatsNextReport,
    pub active_timer: Option<TimerStatus>,
    pub filter_query: String,
    pub filter_editing: bool,
    pub modal: Option<ActiveModal>,
    pub status_message: Option<String>,
    pub should_quit: bool,
}

impl TuiApp {
    /// Initializes a new [`TuiApp`] from [`TrackerStorage`] and loads the current index and recommendations.
    pub fn new(storage: TrackerStorage, cwd: Option<PathBuf>) -> Result<Self> {
        let whats_next = storage.compute_whats_next(cwd.as_deref())?;
        let active_timer = whats_next.active_timer.clone();
        let active_stage = whats_next.active_phase.clone();
        let inferred_scope = whats_next.inferred_scope.clone();

        let mut scope_list_state = ListState::default();
        scope_list_state.select(Some(0));
        let mut item_table_state = TableState::default();
        item_table_state.select(Some(0));

        let mut app = Self {
            storage,
            cwd,
            project_name: "Nebulous".to_string(),
            active_stage,
            inferred_scope,
            available_scopes: vec!["inbox".to_string()],
            scopes: Vec::new(),
            selected_scope_idx: 0,
            scope_list_state,
            items: Vec::new(),
            selected_item_idx: 0,
            item_table_state,
            selected_item_detail: None,
            selected_agent_prompt: None,
            detail_scroll: 0,
            focus: PaneFocus::Items,
            whats_next,
            active_timer,
            filter_query: String::new(),
            filter_editing: false,
            modal: None,
            status_message: None,
            should_quit: false,
        };
        app.refresh()?;
        Ok(app)
    }

    /// Reloads the `.tracker/` index, scopes, items, "What's Next" report, and selected item inspector.
    pub fn refresh(&mut self) -> Result<()> {
        let index = self.storage.load_index()?;
        self.project_name = index.project.project.name.clone();
        self.whats_next = self.storage.compute_whats_next(self.cwd.as_deref())?;
        self.active_timer = self.whats_next.active_timer.clone();
        self.inferred_scope = self.whats_next.inferred_scope.clone();

        let resolved_active = resolve_phase_alias(&index.project.project.active_phase).to_string();
        if self.active_stage.trim().is_empty() {
            self.active_stage = resolved_active;
        }

        let phase_idx = index
            .phase(&self.active_stage)
            .or_else(|| index.phase(resolve_phase_alias(&self.active_stage)))
            .or_else(|| index.active_phase())
            .or_else(|| index.phases.first());

        let prev_selected_scope = self
            .scopes
            .get(self.selected_scope_idx)
            .and_then(|s| s.scope_id.clone());
        let prev_selected_item_id = self.items.get(self.selected_item_idx).map(|i| i.id.clone());

        self.scopes.clear();
        self.available_scopes.clear();

        if let Some(phase) = phase_idx {
            self.active_stage = phase.phase_id.clone();
            let mut stage_total = 0usize;
            let mut stage_open = 0usize;
            let mut individual_rows = Vec::new();

            for sc in &phase.scopes {
                self.available_scopes.push(sc.scope_id.clone());
                let total = sc.manifest.items.bugs.len() + sc.manifest.items.features.len();
                let open = sc
                    .manifest
                    .items
                    .bugs
                    .iter()
                    .chain(sc.manifest.items.features.iter())
                    .filter(|i| !i.status.is_completed())
                    .count();
                stage_total += total;
                stage_open += open;
                individual_rows.push(ScopeRow {
                    scope_id: Some(sc.scope_id.clone()),
                    label: sc.scope_id.clone(),
                    target: sc.manifest.scope.target.clone(),
                    total_items: total,
                    open_items: open,
                });
            }

            self.scopes.push(ScopeRow {
                scope_id: None,
                label: "ALL SCOPES".to_string(),
                target: "*".to_string(),
                total_items: stage_total,
                open_items: stage_open,
            });
            self.scopes.extend(individual_rows);
        } else {
            self.scopes.push(ScopeRow {
                scope_id: None,
                label: "ALL SCOPES".to_string(),
                target: "*".to_string(),
                total_items: 0,
                open_items: 0,
            });
        }

        if self.available_scopes.is_empty() {
            self.available_scopes.push("inbox".to_string());
        }

        if let Some(ref prev_scope) = prev_selected_scope {
            if let Some(pos) = self
                .scopes
                .iter()
                .position(|r| r.scope_id.as_deref() == Some(prev_scope.as_str()))
            {
                self.selected_scope_idx = pos;
            } else {
                self.selected_scope_idx = 0;
            }
        } else if self.selected_scope_idx >= self.scopes.len() {
            self.selected_scope_idx = 0;
        }
        self.scope_list_state.select(Some(self.selected_scope_idx));

        // Populate items in active stage filtered by selected scope and filter query
        let selected_scope_filter = self
            .scopes
            .get(self.selected_scope_idx)
            .and_then(|s| s.scope_id.as_deref());
        let query_lower = self.filter_query.trim().to_ascii_lowercase();

        self.items.clear();
        if let Some(phase) = phase_idx {
            for sc in &phase.scopes {
                if let Some(filter_sc) = selected_scope_filter {
                    if sc.scope_id != filter_sc {
                        continue;
                    }
                }
                let stubs = sc
                    .manifest
                    .items
                    .bugs
                    .iter()
                    .map(|b| (ItemType::Bug, b))
                    .chain(
                        sc.manifest
                            .items
                            .features
                            .iter()
                            .map(|f| (ItemType::Feature, f)),
                    );

                for (item_type, stub) in stubs {
                    if !query_lower.is_empty() {
                        let matches_id = stub.id.to_ascii_lowercase().contains(&query_lower);
                        let matches_title = stub.title.to_ascii_lowercase().contains(&query_lower);
                        let matches_scope = sc.scope_id.to_ascii_lowercase().contains(&query_lower);
                        if !matches_id && !matches_title && !matches_scope {
                            continue;
                        }
                    }
                    let recurrence_count = self
                        .storage
                        .read_item_manifest(&phase.phase_id, &sc.scope_id, &stub.id)
                        .map(|m| m.history.recurrence_count)
                        .unwrap_or(0);

                    self.items.push(ItemRow {
                        id: stub.id.clone(),
                        item_type,
                        title: stub.title.clone(),
                        status: stub.status,
                        severity: stub.severity,
                        phase_id: phase.phase_id.clone(),
                        scope_id: sc.scope_id.clone(),
                        recurrence_count,
                    });
                }
            }
        }

        // Sort items: in_progress first, then open/pending by severity DESC, then completed
        self.items.sort_by(|a, b| {
            let rank_status = |s: ItemStatus| -> u8 {
                match s {
                    ItemStatus::InProgress => 0,
                    ItemStatus::Open => 1,
                    ItemStatus::Pending => 2,
                    ItemStatus::Closed | ItemStatus::Merged => 3,
                }
            };
            rank_status(a.status)
                .cmp(&rank_status(b.status))
                .then_with(|| b.recurrence_count.cmp(&a.recurrence_count))
                .then_with(|| b.severity.cmp(&a.severity))
                .then_with(|| a.id.cmp(&b.id))
        });

        if self.items.is_empty() {
            self.selected_item_idx = 0;
            self.item_table_state.select(None);
            self.selected_item_detail = None;
            self.selected_agent_prompt = None;
        } else {
            if let Some(ref prev_id) = prev_selected_item_id {
                if let Some(pos) = self.items.iter().position(|i| &i.id == prev_id) {
                    self.selected_item_idx = pos;
                } else if self.selected_item_idx >= self.items.len() {
                    self.selected_item_idx = self.items.len() - 1;
                }
            } else if self.selected_item_idx >= self.items.len() {
                self.selected_item_idx = self.items.len() - 1;
            }
            self.item_table_state.select(Some(self.selected_item_idx));
            self.load_selected_item_detail();
        }

        Ok(())
    }

    fn load_selected_item_detail(&mut self) {
        let Some(row) = self.items.get(self.selected_item_idx) else {
            self.selected_item_detail = None;
            self.selected_agent_prompt = None;
            return;
        };
        self.selected_item_detail = self.storage.load_item_detail_by_id(&row.id).ok();
        self.selected_agent_prompt = self
            .storage
            .export_agent_context(&row.id)
            .ok()
            .map(|b| b.bounded_prompt);
    }

    /// Switches to one of the 3 canonical stages (`pre_production`, `production`, `post_production`).
    pub fn switch_stage(&mut self, stage_alias: &str) -> Result<()> {
        let canonical = resolve_phase_alias(stage_alias).to_string();
        if self.storage.set_active_phase(&canonical).is_err() {
            if let Some((_, title)) = CANONICAL_PHASES.iter().find(|(id, _)| *id == canonical) {
                let _ = self.storage.create_phase(&canonical, *title, true);
            }
        }
        self.active_stage = canonical.clone();
        self.detail_scroll = 0;
        *self.item_table_state.offset_mut() = 0;
        *self.scope_list_state.offset_mut() = 0;
        self.refresh()?;
        self.status_message = Some(format!("Switched active stage -> {canonical}"));
        Ok(())
    }

    /// Opens the predictive `New Bug` or `New Feature` creation modal with pre-filled `Scope`, `Stage`, and `Severity`.
    pub fn open_create_modal(&mut self, item_type: ItemType) {
        let predicted_scope = self
            .scopes
            .get(self.selected_scope_idx)
            .and_then(|s| s.scope_id.clone())
            .or_else(|| {
                self.storage
                    .infer_scope_from_context(self.cwd.as_deref())
                    .ok()
            })
            .unwrap_or_else(|| self.inferred_scope.clone());

        self.modal = Some(ActiveModal::Create(CreateModalState {
            item_type,
            title: String::new(),
            scope: predicted_scope,
            stage: self.active_stage.clone(),
            severity: Severity::Medium,
            description: String::new(),
            focused_field: CreateField::Title,
            required_lock_field: None,
        }));
    }

    /// Opens the predictive `Done / Close` modal pre-filled with `"Resolved <ID>: <title>"` and Git `HEAD` SHA.
    pub fn open_close_modal(&mut self) {
        let target = self
            .items
            .get(self.selected_item_idx)
            .map(|r| (r.id.clone(), r.title.clone()))
            .or_else(|| {
                let id = self.storage.resolve_active_or_inferred_item_id(None).ok()?;
                let (_, detail) = self.storage.load_item_detail_by_id(&id).ok()?;
                Some((detail.manifest.item.id, detail.manifest.item.title))
            });

        let Some((item_id, item_title)) = target else {
            self.status_message = Some("No active or selected item to close.".to_string());
            return;
        };

        let commit = self.storage.git_head_short_sha();
        let summary = format!("Resolved {item_id}: {item_title}");
        self.modal = Some(ActiveModal::Close(CloseModalState {
            item_id,
            item_title,
            summary,
            commit,
            merged: false,
            focused_field: CloseField::Summary,
            required_lock_field: None,
            summary_edited: false,
        }));
    }

    /// Opens the 1-key stage move modal (`1` Pre-Production, `2` Production, `3` Post-Production).
    pub fn open_move_modal(&mut self) {
        let target = self
            .items
            .get(self.selected_item_idx)
            .map(|r| (r.id.clone(), r.title.clone(), r.phase_id.clone()))
            .or_else(|| {
                let id = self.storage.resolve_active_or_inferred_item_id(None).ok()?;
                let (loc, detail) = self.storage.load_item_detail_by_id(&id).ok()?;
                Some((
                    detail.manifest.item.id,
                    detail.manifest.item.title,
                    loc.phase_id,
                ))
            });

        let Some((item_id, item_title, current_stage)) = target else {
            self.status_message = Some("No active or selected item to move.".to_string());
            return;
        };

        let selected_stage_idx = match current_stage.as_str() {
            "pre_production" => 1,
            "production" => 2,
            _ => 0,
        };

        self.modal = Some(ActiveModal::Move(MoveModalState {
            item_id,
            item_title,
            current_stage,
            selected_stage_idx,
        }));
    }

    /// Submits the currently active modal, enforcing `[REQUIRED FIELD LOCK]` when a required field is empty.
    pub fn submit_active_modal(&mut self) -> Result<()> {
        let Some(modal) = self.modal.clone() else {
            return Ok(());
        };

        match modal {
            ActiveModal::Create(mut create) => {
                if create.title.trim().is_empty() {
                    create.focused_field = CreateField::Title;
                    create.required_lock_field = Some(CreateField::Title);
                    self.modal = Some(ActiveModal::Create(create));
                    self.status_message = Some(
                        "[REQUIRED FIELD LOCK] Title is required before creating an item."
                            .to_string(),
                    );
                    return Ok(());
                }
                if create.scope.trim().is_empty() {
                    create.focused_field = CreateField::Scope;
                    create.required_lock_field = Some(CreateField::Scope);
                    self.modal = Some(ActiveModal::Create(create));
                    self.status_message = Some(
                        "[REQUIRED FIELD LOCK] Scope is required before creating an item."
                            .to_string(),
                    );
                    return Ok(());
                }
                if create.stage.trim().is_empty() {
                    create.focused_field = CreateField::Stage;
                    create.required_lock_field = Some(CreateField::Stage);
                    self.modal = Some(ActiveModal::Create(create));
                    self.status_message = Some(
                        "[REQUIRED FIELD LOCK] Stage is required before creating an item."
                            .to_string(),
                    );
                    return Ok(());
                }

                let stage_id = resolve_phase_alias(create.stage.trim()).to_string();
                let scope_id = create.scope.trim().to_string();
                if self.storage.read_phase(&stage_id).is_err() {
                    if let Some((_, title)) =
                        CANONICAL_PHASES.iter().find(|(id, _)| *id == stage_id)
                    {
                        let _ = self.storage.create_phase(&stage_id, *title, false);
                    }
                }
                if scope_id != "inbox" && self.storage.read_scope(&stage_id, &scope_id).is_err() {
                    let _ = self.storage.create_scope(
                        Some(&stage_id),
                        &scope_id,
                        &scope_id,
                        Vec::new(),
                        None,
                    );
                }
                let desc_opt = {
                    let trimmed = create.description.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                };

                let (loc, detail) = match self.storage.create_item(CreateItemInput {
                    item_type: create.item_type,
                    title: create.title.trim().to_string(),
                    phase_id: Some(stage_id.clone()),
                    scope_id: Some(scope_id),
                    severity: create.severity,
                    description: desc_opt,
                }) {
                    Ok(res) => res,
                    Err(err) => {
                        let err_msg = err.to_string();
                        let lock_field = if err_msg.to_ascii_lowercase().contains("stage")
                            || err_msg.to_ascii_lowercase().contains("phase")
                        {
                            CreateField::Stage
                        } else {
                            CreateField::Scope
                        };
                        create.focused_field = lock_field;
                        create.required_lock_field = Some(lock_field);
                        self.modal = Some(ActiveModal::Create(create));
                        self.status_message = Some(format!("[REQUIRED FIELD LOCK] {err_msg}"));
                        return Ok(());
                    }
                };

                self.modal = None;
                self.active_stage = loc.phase_id.clone();
                let _ = self.storage.set_active_phase(&self.active_stage);
                self.refresh()?;
                if let Some(pos) = self
                    .items
                    .iter()
                    .position(|i| i.id == detail.manifest.item.id)
                {
                    self.selected_item_idx = pos;
                    self.item_table_state.select(Some(pos));
                    self.load_selected_item_detail();
                }
                self.status_message = Some(format!(
                    "Created {} {} in {}/{}",
                    detail.manifest.item.item_type,
                    detail.manifest.item.id,
                    loc.phase_id,
                    loc.scope_id
                ));
            }
            ActiveModal::Close(mut close) => {
                if close.summary.trim().is_empty() {
                    close.focused_field = CloseField::Summary;
                    close.required_lock_field = Some(CloseField::Summary);
                    self.modal = Some(ActiveModal::Close(close));
                    self.status_message = Some(
                        "[REQUIRED FIELD LOCK] Fix summary is required to close an item."
                            .to_string(),
                    );
                    return Ok(());
                }

                let commit_opt = {
                    let trimmed = close.commit.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed)
                    }
                };
                let detail = self.storage.close_item(
                    &close.item_id,
                    close.summary.trim(),
                    commit_opt,
                    close.merged,
                )?;
                self.modal = None;
                self.refresh()?;
                self.status_message = Some(format!(
                    "Closed {} as {}",
                    detail.manifest.item.id, detail.manifest.item.status
                ));
            }
            ActiveModal::Move(mv) => {
                let target_stage = CANONICAL_PHASES
                    .get(mv.selected_stage_idx)
                    .map(|(id, _)| *id)
                    .unwrap_or("production");
                let (loc, _) = self.storage.move_item_to_phase(&mv.item_id, target_stage)?;
                self.modal = None;
                self.active_stage = loc.phase_id.clone();
                let _ = self.storage.set_active_phase(&self.active_stage);
                self.refresh()?;
                if let Some(pos) = self.items.iter().position(|i| i.id == mv.item_id) {
                    self.selected_item_idx = pos;
                    self.item_table_state.select(Some(pos));
                    self.load_selected_item_detail();
                }
                self.status_message = Some(format!("Moved {} -> {}", mv.item_id, loc.phase_id));
            }
        }

        Ok(())
    }

    /// Processes a single keyboard event in the TUI state machine.
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return Ok(());
        }

        if self.modal.is_some() {
            return self.handle_modal_key(key);
        }

        if self.filter_editing {
            return self.handle_filter_key(key);
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char('1') => {
                self.switch_stage("pre_production")?;
            }
            KeyCode::Char('2') => {
                self.switch_stage("production")?;
            }
            KeyCode::Char('3') => {
                self.switch_stage("post_production")?;
            }
            KeyCode::Tab | KeyCode::Char('l') | KeyCode::Right => {
                self.focus = self.focus.next();
            }
            KeyCode::BackTab | KeyCode::Char('h') | KeyCode::Left => {
                self.focus = self.focus.prev();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_selection_down()?;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_selection_up()?;
            }
            KeyCode::PageDown | KeyCode::Char('J') => {
                self.detail_scroll = self.detail_scroll.saturating_add(4);
            }
            KeyCode::PageUp | KeyCode::Char('K') => {
                self.detail_scroll = self.detail_scroll.saturating_sub(4);
            }
            KeyCode::Char('b') => {
                self.open_create_modal(ItemType::Bug);
            }
            KeyCode::Char('f') => {
                self.open_create_modal(ItemType::Feature);
            }
            KeyCode::Char('s') => {
                self.action_start_work()?;
            }
            KeyCode::Char('d') => {
                self.open_close_modal();
            }
            KeyCode::Char('m') => {
                self.open_move_modal();
            }
            KeyCode::Char('c') => {
                self.action_copy_context();
            }
            KeyCode::Char('i') => {
                self.action_ingest_workspace()?;
            }
            KeyCode::Char('n') => {
                self.action_jump_whats_next()?;
            }
            KeyCode::Char('/') => {
                self.filter_editing = true;
            }
            _ => {}
        }

        Ok(())
    }

    /// Convenience helper for headless testing: dispatches a character key press.
    pub fn press_char(&mut self, ch: char) -> Result<()> {
        self.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE))
    }

    /// Convenience helper for headless testing: dispatches a special `KeyCode`.
    pub fn press_key(&mut self, code: KeyCode) -> Result<()> {
        self.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    /// Convenience helper for headless testing: types a string of characters sequentially.
    pub fn type_text(&mut self, text: &str) -> Result<()> {
        for ch in text.chars() {
            self.press_char(ch)?;
        }
        Ok(())
    }

    fn handle_filter_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => {
                self.filter_editing = false;
                self.refresh()?;
            }
            KeyCode::Esc => {
                self.filter_editing = false;
                self.filter_query.clear();
                self.refresh()?;
            }
            KeyCode::Backspace => {
                self.filter_query.pop();
                self.refresh()?;
            }
            KeyCode::Char(ch) => {
                self.filter_query.push(ch);
                self.refresh()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_modal_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.code == KeyCode::Esc {
            self.modal = None;
            return Ok(());
        }
        if key.code == KeyCode::Enter {
            return self.submit_active_modal();
        }

        let Some(modal) = self.modal.as_mut() else {
            return Ok(());
        };

        match modal {
            ActiveModal::Create(create) => match key.code {
                KeyCode::Tab | KeyCode::Down => {
                    create.focused_field = create.focused_field.next();
                }
                KeyCode::BackTab | KeyCode::Up => {
                    create.focused_field = create.focused_field.prev();
                }
                KeyCode::Left => match create.focused_field {
                    CreateField::Severity => {
                        create.severity = match create.severity {
                            Severity::Low => Severity::Critical,
                            Severity::Medium => Severity::Low,
                            Severity::High => Severity::Medium,
                            Severity::Critical => Severity::High,
                        };
                    }
                    CreateField::Stage => {
                        create.stage = match create.stage.as_str() {
                            "pre_production" => "post_production".to_string(),
                            "production" => "pre_production".to_string(),
                            _ => "production".to_string(),
                        };
                        if create.required_lock_field == Some(CreateField::Stage)
                            && !create.stage.trim().is_empty()
                        {
                            create.required_lock_field = None;
                        }
                    }
                    CreateField::Scope if !self.available_scopes.is_empty() => {
                        let idx = self
                            .available_scopes
                            .iter()
                            .position(|s| s == &create.scope)
                            .unwrap_or(0);
                        let prev = if idx == 0 {
                            self.available_scopes.len() - 1
                        } else {
                            idx - 1
                        };
                        create.scope = self.available_scopes[prev].clone();
                        if create.required_lock_field == Some(CreateField::Scope)
                            && !create.scope.trim().is_empty()
                        {
                            create.required_lock_field = None;
                        }
                    }
                    _ => {}
                },
                KeyCode::Right => match create.focused_field {
                    CreateField::Severity => {
                        create.severity = match create.severity {
                            Severity::Low => Severity::Medium,
                            Severity::Medium => Severity::High,
                            Severity::High => Severity::Critical,
                            Severity::Critical => Severity::Low,
                        };
                    }
                    CreateField::Stage => {
                        create.stage = match create.stage.as_str() {
                            "pre_production" => "production".to_string(),
                            "production" => "post_production".to_string(),
                            _ => "pre_production".to_string(),
                        };
                        if create.required_lock_field == Some(CreateField::Stage)
                            && !create.stage.trim().is_empty()
                        {
                            create.required_lock_field = None;
                        }
                    }
                    CreateField::Scope if !self.available_scopes.is_empty() => {
                        let idx = self
                            .available_scopes
                            .iter()
                            .position(|s| s == &create.scope)
                            .unwrap_or(0);
                        let next = (idx + 1) % self.available_scopes.len();
                        create.scope = self.available_scopes[next].clone();
                        if create.required_lock_field == Some(CreateField::Scope)
                            && !create.scope.trim().is_empty()
                        {
                            create.required_lock_field = None;
                        }
                    }
                    _ => {}
                },
                KeyCode::Backspace => match create.focused_field {
                    CreateField::Title => {
                        create.title.pop();
                    }
                    CreateField::Scope => {
                        create.scope.pop();
                    }
                    CreateField::Stage => {
                        create.stage.pop();
                    }
                    CreateField::Description => {
                        create.description.pop();
                    }
                    CreateField::Severity => {}
                },
                KeyCode::Char(ch) => match create.focused_field {
                    CreateField::Title => {
                        create.title.push(ch);
                        if create.required_lock_field == Some(CreateField::Title)
                            && !create.title.trim().is_empty()
                        {
                            create.required_lock_field = None;
                        }
                    }
                    CreateField::Scope => {
                        create.scope.push(ch);
                        if create.required_lock_field == Some(CreateField::Scope)
                            && !create.scope.trim().is_empty()
                        {
                            create.required_lock_field = None;
                        }
                    }
                    CreateField::Stage => {
                        create.stage.push(ch);
                        if create.required_lock_field == Some(CreateField::Stage)
                            && !create.stage.trim().is_empty()
                        {
                            create.required_lock_field = None;
                        }
                    }
                    CreateField::Severity => match ch {
                        '1' | 'l' | 'L' => create.severity = Severity::Low,
                        '2' | 'm' | 'M' => create.severity = Severity::Medium,
                        '3' | 'h' | 'H' => create.severity = Severity::High,
                        '4' | 'c' | 'C' => create.severity = Severity::Critical,
                        ' ' => {
                            create.severity = match create.severity {
                                Severity::Low => Severity::Medium,
                                Severity::Medium => Severity::High,
                                Severity::High => Severity::Critical,
                                Severity::Critical => Severity::Low,
                            };
                        }
                        _ => {}
                    },
                    CreateField::Description => {
                        create.description.push(ch);
                    }
                },
                _ => {}
            },
            ActiveModal::Close(close) => match key.code {
                KeyCode::Tab | KeyCode::Down => {
                    close.focused_field = close.focused_field.next();
                }
                KeyCode::BackTab | KeyCode::Up => {
                    close.focused_field = close.focused_field.prev();
                }
                KeyCode::Left | KeyCode::Right => {
                    if close.focused_field == CloseField::Mode {
                        close.merged = !close.merged;
                    }
                }
                KeyCode::Backspace => match close.focused_field {
                    CloseField::Summary => {
                        close.summary.pop();
                        close.summary_edited = true;
                    }
                    CloseField::Commit => {
                        close.commit.pop();
                    }
                    CloseField::Mode => {}
                },
                KeyCode::Char(ch) => match close.focused_field {
                    CloseField::Summary => {
                        close.summary.push(ch);
                        close.summary_edited = true;
                        if close.required_lock_field == Some(CloseField::Summary)
                            && !close.summary.trim().is_empty()
                        {
                            close.required_lock_field = None;
                        }
                    }
                    CloseField::Commit => {
                        close.commit.push(ch);
                    }
                    CloseField::Mode => {
                        if ch == ' ' || ch == 'm' || ch == 'c' {
                            close.merged = !close.merged;
                        }
                    }
                },
                _ => {}
            },
            ActiveModal::Move(mv) => match key.code {
                KeyCode::Char('1') => {
                    mv.selected_stage_idx = 0;
                    return self.submit_active_modal();
                }
                KeyCode::Char('2') => {
                    mv.selected_stage_idx = 1;
                    return self.submit_active_modal();
                }
                KeyCode::Char('3') => {
                    mv.selected_stage_idx = 2;
                    return self.submit_active_modal();
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                    mv.selected_stage_idx = (mv.selected_stage_idx + 1) % CANONICAL_PHASES.len();
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => {
                    mv.selected_stage_idx = if mv.selected_stage_idx == 0 {
                        CANONICAL_PHASES.len() - 1
                    } else {
                        mv.selected_stage_idx - 1
                    };
                }
                _ => {}
            },
        }

        Ok(())
    }

    fn move_selection_down(&mut self) -> Result<()> {
        match self.focus {
            PaneFocus::Scopes => {
                if !self.scopes.is_empty() {
                    self.selected_scope_idx = (self.selected_scope_idx + 1) % self.scopes.len();
                    self.scope_list_state.select(Some(self.selected_scope_idx));
                    self.selected_item_idx = 0;
                    self.detail_scroll = 0;
                    self.refresh()?;
                }
            }
            PaneFocus::Items => {
                if !self.items.is_empty() {
                    self.selected_item_idx = (self.selected_item_idx + 1) % self.items.len();
                    self.item_table_state.select(Some(self.selected_item_idx));
                    self.detail_scroll = 0;
                    self.load_selected_item_detail();
                }
            }
            PaneFocus::Details => {
                self.detail_scroll = self.detail_scroll.saturating_add(1);
            }
        }
        Ok(())
    }

    fn move_selection_up(&mut self) -> Result<()> {
        match self.focus {
            PaneFocus::Scopes => {
                if !self.scopes.is_empty() {
                    self.selected_scope_idx = if self.selected_scope_idx == 0 {
                        self.scopes.len() - 1
                    } else {
                        self.selected_scope_idx - 1
                    };
                    self.scope_list_state.select(Some(self.selected_scope_idx));
                    self.selected_item_idx = 0;
                    self.detail_scroll = 0;
                    self.refresh()?;
                }
            }
            PaneFocus::Items => {
                if !self.items.is_empty() {
                    self.selected_item_idx = if self.selected_item_idx == 0 {
                        self.items.len() - 1
                    } else {
                        self.selected_item_idx - 1
                    };
                    self.item_table_state.select(Some(self.selected_item_idx));
                    self.detail_scroll = 0;
                    self.load_selected_item_detail();
                }
            }
            PaneFocus::Details => {
                self.detail_scroll = self.detail_scroll.saturating_sub(1);
            }
        }
        Ok(())
    }

    fn action_start_work(&mut self) -> Result<()> {
        let explicit_id = self.items.get(self.selected_item_idx).map(|r| r.id.clone());
        let Ok((loc, detail, _, bundle)) =
            self.storage.start_work_item(explicit_id.as_deref(), None)
        else {
            self.status_message = Some("No open or selected item to start.".to_string());
            return Ok(());
        };
        let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(bundle.bounded_prompt));
        self.active_stage = loc.phase_id.clone();
        self.refresh()?;
        self.status_message = Some(format!(
            "Started {} [in_progress] in {}/{} (timer running, agent context copied)",
            detail.manifest.item.id, loc.phase_id, loc.scope_id
        ));
        Ok(())
    }

    fn action_copy_context(&mut self) {
        let item_id = self
            .items
            .get(self.selected_item_idx)
            .map(|r| r.id.clone())
            .or_else(|| self.storage.resolve_active_or_inferred_item_id(None).ok());

        let Some(id) = item_id else {
            self.status_message = Some("No item selected to export context.".to_string());
            return;
        };

        if let Ok(bundle) = self.storage.export_agent_context(&id) {
            let copied = arboard::Clipboard::new()
                .and_then(|mut cb| cb.set_text(bundle.bounded_prompt.clone()))
                .is_ok();
            self.selected_agent_prompt = Some(bundle.bounded_prompt);
            self.status_message = Some(if copied {
                format!("Copied bounded agent context for {id} to clipboard.")
            } else {
                format!("Prepared bounded agent context for {id} (clipboard unavailable).")
            });
        }
    }

    fn action_ingest_workspace(&mut self) -> Result<()> {
        let report = self.storage.ingest_project_directory(IngestionOptions {
            write_manifests: true,
            promote_all: true,
            promote_phase_id: Some(self.active_stage.clone()),
            subpath: None,
        })?;
        self.refresh()?;
        self.status_message = Some(format!(
            "Ingested & promoted {} component(s) across stages.",
            report.components.len()
        ));
        Ok(())
    }

    fn action_jump_whats_next(&mut self) -> Result<()> {
        let Some(top_rec) = self
            .whats_next
            .recommendations
            .iter()
            .find(|r| r.item_id.is_some())
            .cloned()
        else {
            self.status_message =
                Some("No recommended work item in What's Next queue.".to_string());
            return Ok(());
        };

        let target_id = top_rec.item_id.unwrap();
        if let Some(ref phase_id) = top_rec.phase_id {
            if phase_id != &self.active_stage {
                self.active_stage = phase_id.clone();
                let _ = self.storage.set_active_phase(&self.active_stage);
            }
        }
        self.selected_scope_idx = 0; // ALL SCOPES
        self.refresh()?;
        if let Some(pos) = self.items.iter().position(|i| i.id == target_id) {
            self.selected_item_idx = pos;
            self.item_table_state.select(Some(pos));
            self.focus = PaneFocus::Items;
            self.load_selected_item_detail();
            self.status_message = Some(format!("Jumped to What's Next focus: {target_id}"));
        }
        Ok(())
    }
}
