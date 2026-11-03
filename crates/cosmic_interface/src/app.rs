use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use backend_core::{
    CreateItemInput, GitRepoStatus, GitSyncMode, ItemDetail, ItemLocation, ItemManifest,
    ItemStatus, ItemType, PhaseIndex, ProjectMetricsReport, Result, Severity, TimerStatus,
    TrackerError, TrackerIndex, TrackerStorage,
};
use chrono::Utc;
use cosmic::app::{Core, Task};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::{executor, widget, Application, Element};

use crate::subscriptions;
use crate::views::{
    detail_drawer, header_bar, kanban_board, metrics_view, nav_sidebar, settings_view,
};

/// Startup flags passed into [`NebulousApp::init`].
#[derive(Debug, Clone, Default)]
pub struct AppFlags {
    pub workspace_path: Option<PathBuf>,
}

/// Primary navigation view modes in the left sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Board,
    Metrics,
    PhasesAndScopes,
    Settings,
}

/// Top header bar filter pills for narrowing cards on the Kanban board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterPill {
    #[default]
    All,
    Bugs,
    Features,
    HighOrCritical,
    Stagnant,
    Recurring,
}

/// The 4 simplified columns of the central Kanban card board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KanbanColumn {
    Open,
    InProgress,
    Pending,
    ClosedOrMerged,
}

impl KanbanColumn {
    /// Display title for the column header.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Open => "OPEN",
            Self::InProgress => "IN PROGRESS",
            Self::Pending => "PENDING",
            Self::ClosedOrMerged => "CLOSED / MERGED",
        }
    }

    /// Returns `true` if `status` belongs to this Kanban column.
    pub const fn matches(self, status: ItemStatus) -> bool {
        match self {
            Self::Open => matches!(status, ItemStatus::Open),
            Self::InProgress => matches!(status, ItemStatus::InProgress),
            Self::Pending => matches!(status, ItemStatus::Pending),
            Self::ClosedOrMerged => matches!(status, ItemStatus::Closed | ItemStatus::Merged),
        }
    }
}

/// Counts displayed inside the top bar filter pills.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FilterPillCounts {
    pub all: usize,
    pub bugs: usize,
    pub features: usize,
    pub high_or_critical: usize,
    pub stagnant: usize,
    pub recurring: usize,
}

/// Uncluttered card view-model rendered inside a [`KanbanColumn`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KanbanCardViewModel {
    pub id: String,
    pub item_type: ItemType,
    pub title: String,
    pub status: ItemStatus,
    pub severity: Severity,
    pub phase_id: String,
    pub scope_id: String,
    pub stagnant_days: Option<i64>,
    pub recurrence_count: u32,
    pub logged_time_minutes: u32,
    pub previous_fixes_count: usize,
}

/// Cached per-item signal badges (stagnation, recurrence, effort) without loading any `desc.md` files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CardSignalSummary {
    pub stagnant_days: Option<i64>,
    pub recurrence_count: u32,
    pub logged_time_minutes: u32,
    pub iteration_count: u32,
    pub previous_fixes_count: usize,
}

/// Inline quick-add bar state in the top header bar.
#[derive(Debug, Clone)]
pub struct QuickAddState {
    pub open: bool,
    pub item_type: ItemType,
    pub severity: Severity,
    pub title: String,
}

impl Default for QuickAddState {
    fn default() -> Self {
        Self {
            open: false,
            item_type: ItemType::Bug,
            severity: Severity::Medium,
            title: String::new(),
        }
    }
}

/// Form state for the Phases & Scopes Manager and Project/Git/Keyring Settings view.
#[derive(Debug, Clone)]
pub struct SettingsFormState {
    pub new_phase_id: String,
    pub new_phase_title: String,
    pub new_phase_set_active: bool,
    pub new_scope_id: String,
    pub new_scope_target: String,
    pub new_scope_tech_stack: String,
    pub new_scope_description: String,
    pub target_daily_hours_input: String,
    pub stagnant_threshold_days_input: String,
    pub default_scope_input: String,
    pub git_sync_mode_input: GitSyncMode,
    pub git_sync_interval_input: String,
    pub git_remote_input: String,
    pub git_branch_input: String,
    pub pat_input: String,
    pub has_stored_pat: bool,
}

impl SettingsFormState {
    pub fn from_index_and_storage(index: &TrackerIndex, storage: &TrackerStorage) -> Self {
        let project = &index.project;
        let has_stored_pat = storage.get_pat().is_ok();
        Self {
            new_phase_id: String::new(),
            new_phase_title: String::new(),
            new_phase_set_active: false,
            new_scope_id: String::new(),
            new_scope_target: String::new(),
            new_scope_tech_stack: String::new(),
            new_scope_description: String::new(),
            target_daily_hours_input: format!("{:.1}", project.metrics.target_daily_hours),
            stagnant_threshold_days_input: project.metrics.stagnant_item_threshold_days.to_string(),
            default_scope_input: project.project.default_scope.clone(),
            git_sync_mode_input: project.git.sync_mode,
            git_sync_interval_input: project.git.sync_interval_minutes.to_string(),
            git_remote_input: project.git.remote.clone(),
            git_branch_input: project.git.branch.clone(),
            pat_input: String::new(),
            has_stored_pat,
        }
    }

    pub fn sync_from_index(&mut self, index: &TrackerIndex, storage: &TrackerStorage) {
        let project = &index.project;
        self.target_daily_hours_input = format!("{:.1}", project.metrics.target_daily_hours);
        self.stagnant_threshold_days_input =
            project.metrics.stagnant_item_threshold_days.to_string();
        self.default_scope_input = project.project.default_scope.clone();
        self.git_sync_mode_input = project.git.sync_mode;
        self.git_sync_interval_input = project.git.sync_interval_minutes.to_string();
        self.git_remote_input = project.git.remote.clone();
        self.git_branch_input = project.git.branch.clone();
        self.has_stored_pat = storage.get_pat().is_ok();
    }
}

/// Application messages dispatched by the native `libcosmic` GUI.
#[derive(Debug, Clone)]
pub enum Message {
    SelectPhaseIndex(usize),
    SelectPhase(String),
    SelectScope(Option<String>),
    SelectScopeAndOpenBoard(String),
    SelectPhaseScopeAndOpenBoard {
        phase_id: String,
        scope_id: String,
    },
    SwitchView(ViewMode),
    SetFilterPill(FilterPill),
    SelectCard(String),
    ClearSelectedCard,
    OpenQuickAdd(ItemType),
    CloseQuickAdd,
    SetQuickAddType(ItemType),
    SetQuickAddSeverity(Severity),
    QuickAddTitleChanged(String),
    SubmitQuickAdd,
    QuickStatusTransition {
        item_id: String,
        new_status: ItemStatus,
    },
    UpdateSelectedSeverity(Severity),
    StartTimer(String),
    StopTimer,
    TimerTick,
    TriggerGitSync,
    CommitTrackerNow,
    ReconcileIndexes,
    DismissBanner,

    // Context Drawer messages
    DrawerTitleChanged(String),
    SaveDrawerTitle,
    DescEditorAction(widget::text_editor::Action),
    SetDescMarkdown(String),
    AppendDescTemplate(&'static str),
    SaveDescMarkdown,
    OpenInExternalEditor,
    ManualMinutesInputChanged(String),
    LogManualMinutes(u32),
    SubmitCustomManualMinutes,
    CloseSummaryInputChanged(String),
    CloseCommitInputChanged(String),
    CloseSelectedWithSummary {
        merged: bool,
    },
    ReopenReasonInputChanged(String),
    ReopenSelectedItem,
    CopyAgentFileRefs,
    CopyBoundedAgentPrompt,

    // Settings & Phase/Scope Manager messages
    NewPhaseIdChanged(String),
    NewPhaseTitleChanged(String),
    ToggleNewPhaseSetActive(bool),
    CreatePhase,
    NewScopeIdChanged(String),
    NewScopeTargetChanged(String),
    NewScopeTechStackChanged(String),
    NewScopeDescriptionChanged(String),
    CreateScope,
    TargetDailyHoursChanged(String),
    StagnantThresholdDaysChanged(String),
    DefaultScopeChanged(String),
    SaveProjectAndMetricsConfig,
    SetGitSyncMode(GitSyncMode),
    GitSyncIntervalChanged(String),
    GitRemoteChanged(String),
    GitBranchChanged(String),
    SaveGitConfig,
    PatInputChanged(String),
    SavePatToKeyring,
    DeletePatFromKeyring,

    // Reactive background subscription messages
    TrackerFilesChanged(Vec<PathBuf>),
    ScheduledGitSyncTick,
}

/// Native `libcosmic` application state for Nebulous (`nebulous` binary).
pub struct NebulousApp {
    pub core: Core,
    pub storage: TrackerStorage,
    /// Top-level `.tracker/` index (`project.toml`, `phase.toml`, `scope.toml`) loaded at startup.
    pub index: TrackerIndex,
    /// Currently selected phase ID (`project.active_phase`).
    pub active_phase_id: String,
    /// Ordered phase IDs for the top-bar dropdown.
    pub phase_ids: Vec<String>,
    /// Formatted phase labels (`01_core_engine — Core Engine & Persistence`) for the dropdown.
    pub phase_labels: Vec<String>,
    /// Selected domain scope filter (`None` = `All Scopes`, `Some(id)` = single scope).
    pub selected_scope: Option<String>,
    /// Active top-bar filter pill.
    pub active_filter: FilterPill,
    /// Active central view mode (`Board`, `Metrics`, `PhasesAndScopes`, `Settings`).
    pub view_mode: ViewMode,
    /// Currently selected card item ID.
    pub selected_item_id: Option<String>,
    /// Location of the currently selected card (`phase_id`, `scope_id`).
    pub selected_item_location: Option<ItemLocation>,
    /// Lazily loaded [`ItemDetail`] (`item.toml` + `desc.md`) populated ONLY when a card is selected.
    pub selected_item_detail: Option<ItemDetail>,
    /// Audit counter tracking how many times `load_item_detail` (`item.toml` + `desc.md`) has been invoked.
    /// Remains `0` on startup until a card is explicitly selected.
    pub lazy_detail_loads_count: usize,
    /// Editable title input in the right Context Drawer.
    pub drawer_title_input: String,
    /// Native `cosmic::widget::text_editor` state for `desc.md` in the right Context Drawer.
    pub desc_editor_content: widget::text_editor::Content,
    /// Whether the `desc.md` editor buffer has unsaved edits.
    pub desc_editor_dirty: bool,
    /// Custom manual effort minutes input in the right Context Drawer.
    pub drawer_manual_minutes_input: String,
    /// Fix summary input for the "Close with Fix Summary" workflow in the right Context Drawer.
    pub drawer_close_summary_input: String,
    /// Optional commit SHA override for the "Close with Fix Summary" workflow.
    pub drawer_close_commit_input: String,
    /// Optional reopen reason note when reopening a closed or merged item.
    pub drawer_reopen_reason_input: String,
    /// Most recent payload copied via `Copy @file Refs` or `Copy Bounded Agent Prompt`.
    pub last_copied_clipboard: Option<String>,
    /// Persistent OS clipboard handle so Linux X11/Wayland clipboard managers retain copied text.
    pub clipboard_handle: Option<arboard::Clipboard>,
    /// Optional signal badges (`recurrence_count`, `stagnant_days`, `logged_time_minutes`) populated from `item.toml` metadata only (never `desc.md`).
    pub card_signals: HashMap<String, CardSignalSummary>,
    /// Active session timer status from `.tracker/.timer.toml`.
    pub active_timer: Option<TimerStatus>,
    /// Current Git repository status (HEAD short SHA & dirty `.tracker/` indicator).
    pub git_status: Option<GitRepoStatus>,
    /// Inline quick-add bug/feature state.
    pub quick_add: QuickAddState,
    /// Form state for Phases/Scopes creation, Metrics thresholds, and Git/Keyring settings.
    pub settings_form: SettingsFormState,
    /// Transient status/error notification banner.
    pub status_banner: Option<String>,
}

impl NebulousApp {
    /// Resolves `TrackerStorage` from an optional workspace path or current working directory,
    /// and initializes a new `.tracker/` if none exists yet.
    pub fn resolve_or_init_storage(workspace_path: Option<&Path>) -> Result<TrackerStorage> {
        let base_dir = match workspace_path {
            Some(p) => p.to_path_buf(),
            None => std::env::current_dir().map_err(|source| TrackerError::Io {
                path: PathBuf::from("."),
                source,
            })?,
        };

        match TrackerStorage::discover(&base_dir) {
            Ok(storage) => Ok(storage),
            Err(TrackerError::TrackerRootNotFound(_)) => {
                TrackerStorage::init(&base_dir, "Nebulous", "01_core_engine", "Core Engine")
            }
            Err(err) => Err(err),
        }
    }

    /// Bootstraps `NebulousApp` reading ONLY top-level `.tracker/` indexes (`project.toml`,
    /// `phase.toml`, `scope.toml`) without eagerly loading any `item.toml` + `desc.md` detail pairs.
    pub fn new_from_storage(core: Core, storage: TrackerStorage) -> Result<Self> {
        // Startup strictly reads top-level indexes via `load_index()`.
        let index = storage.load_index()?;
        let active_phase_id = index.project.project.active_phase.clone();
        let (phase_ids, phase_labels) = build_phase_dropdown_lists(&index);
        let active_timer = storage.timer_status().ok().flatten();
        let git_status = storage.git_status().ok();
        let settings_form = SettingsFormState::from_index_and_storage(&index, &storage);

        Ok(Self {
            core,
            storage,
            index,
            active_phase_id,
            phase_ids,
            phase_labels,
            selected_scope: None,
            active_filter: FilterPill::All,
            view_mode: ViewMode::Board,
            selected_item_id: None,
            selected_item_location: None,
            selected_item_detail: None,
            lazy_detail_loads_count: 0,
            drawer_title_input: String::new(),
            desc_editor_content: widget::text_editor::Content::new(),
            desc_editor_dirty: false,
            drawer_manual_minutes_input: "15".to_string(),
            drawer_close_summary_input: String::new(),
            drawer_close_commit_input: String::new(),
            drawer_reopen_reason_input: String::new(),
            last_copied_clipboard: None,
            clipboard_handle: None,
            card_signals: HashMap::new(),
            active_timer,
            git_status,
            quick_add: QuickAddState::default(),
            settings_form,
            status_banner: None,
        })
    }

    /// Returns the project name from `project.toml`.
    pub fn project_name(&self) -> &str {
        &self.index.project.project.name
    }

    /// Returns the configured default scope from `project.toml`.
    pub fn default_scope(&self) -> &str {
        let s = self.index.project.project.default_scope.trim();
        if s.is_empty() {
            "inbox"
        } else {
            s
        }
    }

    /// Returns the configured target daily effort hours from `project.toml`.
    pub fn target_daily_hours(&self) -> f64 {
        self.index.project.metrics.target_daily_hours
    }

    /// Returns the configured stagnant threshold days from `project.toml`.
    pub fn stagnant_threshold_days(&self) -> u32 {
        self.index.project.metrics.stagnant_item_threshold_days
    }

    /// Returns the currently active [`PhaseIndex`].
    pub fn active_phase(&self) -> Option<&PhaseIndex> {
        self.index
            .phase(&self.active_phase_id)
            .or_else(|| self.index.phases.first())
    }

    /// Returns the total item count across all scopes in the active phase.
    pub fn active_phase_total_items(&self) -> usize {
        self.active_phase()
            .map(|p| p.scopes.iter().map(|s| s.manifest.total_items()).sum())
            .unwrap_or(0)
    }

    /// Returns `(completed_items, total_items)` for the active phase from `phase.toml` summary.
    pub fn active_phase_completion_counts(&self) -> (u32, u32) {
        self.active_phase()
            .map(|p| {
                (
                    p.manifest.summary.completed_items,
                    p.manifest.summary.total_items,
                )
            })
            .unwrap_or((0, 0))
    }

    /// Returns the total logged minutes across items in the active phase.
    pub fn active_phase_logged_minutes(&self) -> u32 {
        let Some(phase) = self.active_phase() else {
            return 0;
        };
        let mut total = 0u32;
        for scope in &phase.scopes {
            for stub in scope
                .manifest
                .items
                .bugs
                .iter()
                .chain(scope.manifest.items.features.iter())
            {
                if let Some(sig) = self.card_signals.get(&stub.id) {
                    total = total.saturating_add(sig.logged_time_minutes);
                } else if let Ok(manifest) =
                    self.storage
                        .read_item_manifest(&phase.phase_id, &scope.scope_id, &stub.id)
                {
                    total = total.saturating_add(manifest.effort.logged_time_minutes);
                }
            }
        }
        total
    }

    /// Computes the full [`ProjectMetricsReport`] for the Metrics & Hotspots dashboard.
    pub fn compute_current_metrics(&self) -> ProjectMetricsReport {
        self.storage
            .compute_metrics()
            .unwrap_or_else(|_| ProjectMetricsReport {
                target_daily_hours: self.target_daily_hours(),
                stagnant_threshold_days: self.stagnant_threshold_days(),
                total_logged_minutes: 0,
                total_logged_hours: 0.0,
                target_days_equivalent: 0.0,
                total_items: 0,
                completed_items: 0,
                phases: Vec::new(),
                stagnant_items: Vec::new(),
                item_hotspots: Vec::new(),
                scope_hotspots: Vec::new(),
            })
    }

    /// Returns the current text inside the `desc.md` Markdown editor buffer.
    pub fn current_desc_editor_text(&self) -> String {
        self.desc_editor_content.text()
    }

    /// Returns the number of items in the active phase that are stagnant or have recurred (`recurrence_count > 0`).
    pub fn hotspot_alert_count(&self) -> usize {
        self.active_phase_cards()
            .into_iter()
            .filter(|c| c.stagnant_days.is_some() || c.recurrence_count > 0)
            .count()
    }

    /// Returns `true` if the active timer is currently running for `item_id`.
    pub fn is_timer_running_for(&self, item_id: &str) -> bool {
        self.active_timer
            .as_ref()
            .is_some_and(|t| t.timer.item_id == item_id)
    }

    /// Builds all [`KanbanCardViewModel`]s in the active phase (before scope or filter pill narrowing).
    pub fn active_phase_cards(&self) -> Vec<KanbanCardViewModel> {
        let Some(phase) = self.active_phase() else {
            return Vec::new();
        };

        let now = Utc::now();
        let threshold_days = i64::from(self.stagnant_threshold_days());
        let mut cards = Vec::new();

        for scope in &phase.scopes {
            let bug_iter = scope
                .manifest
                .items
                .bugs
                .iter()
                .map(|stub| (ItemType::Bug, stub));
            let feat_iter = scope
                .manifest
                .items
                .features
                .iter()
                .map(|stub| (ItemType::Feature, stub));

            for (item_type, stub) in bug_iter.chain(feat_iter) {
                let signal = self.card_signals.get(&stub.id).cloned().unwrap_or_else(|| {
                    // Read lightweight signal badges if present on disk without reading desc.md
                    if let Ok(manifest) =
                        self.storage
                            .read_item_manifest(&phase.phase_id, &scope.scope_id, &stub.id)
                    {
                        compute_signal_from_manifest(&manifest, now, threshold_days)
                    } else {
                        CardSignalSummary::default()
                    }
                });

                cards.push(KanbanCardViewModel {
                    id: stub.id.clone(),
                    item_type,
                    title: stub.title.clone(),
                    status: stub.status,
                    severity: stub.severity,
                    phase_id: phase.phase_id.clone(),
                    scope_id: scope.scope_id.clone(),
                    stagnant_days: signal.stagnant_days,
                    recurrence_count: signal.recurrence_count,
                    logged_time_minutes: signal.logged_time_minutes,
                    previous_fixes_count: signal.previous_fixes_count,
                });
            }
        }

        cards
    }

    /// Returns cards in the active phase filtered by `selected_scope` (if any) for computing filter pill counts.
    pub fn scope_filtered_cards(&self) -> Vec<KanbanCardViewModel> {
        let cards = self.active_phase_cards();
        match self.selected_scope.as_deref() {
            Some(scope_id) => cards
                .into_iter()
                .filter(|c| c.scope_id == scope_id)
                .collect(),
            None => cards,
        }
    }

    /// Computes item counts for each top-bar [`FilterPill`] within the active phase and scope selection.
    pub fn filter_counts(&self) -> FilterPillCounts {
        let cards = self.scope_filtered_cards();
        let mut counts = FilterPillCounts {
            all: cards.len(),
            ..Default::default()
        };

        for card in &cards {
            match card.item_type {
                ItemType::Bug => counts.bugs += 1,
                ItemType::Feature => counts.features += 1,
            }
            if matches!(card.severity, Severity::High | Severity::Critical) {
                counts.high_or_critical += 1;
            }
            if card.stagnant_days.is_some() {
                counts.stagnant += 1;
            }
            if card.recurrence_count > 0 {
                counts.recurring += 1;
            }
        }

        counts
    }

    /// Returns the Kanban cards matching the active phase, selected domain scope, and active filter pill.
    pub fn filtered_kanban_cards(&self) -> Vec<KanbanCardViewModel> {
        self.scope_filtered_cards()
            .into_iter()
            .filter(|card| match self.active_filter {
                FilterPill::All => true,
                FilterPill::Bugs => card.item_type == ItemType::Bug,
                FilterPill::Features => card.item_type == ItemType::Feature,
                FilterPill::HighOrCritical => {
                    matches!(card.severity, Severity::High | Severity::Critical)
                }
                FilterPill::Stagnant => card.stagnant_days.is_some(),
                FilterPill::Recurring => card.recurrence_count > 0,
            })
            .collect()
    }

    /// Reloads `.tracker/` top-level indexes (`project.toml`, `phase.toml`, `scope.toml`), timer, and Git status.
    /// If a card is currently selected, refreshes its lazily loaded detail as well (preserving unsaved `desc.md`
    /// editor buffer if `desc_editor_dirty` is true and preserving unsaved `drawer_title_input` if the disk title
    /// has not changed).
    pub fn reload_indexes(&mut self) -> Result<()> {
        let prev_disk_active_phase = self.index.project.project.active_phase.clone();
        self.index = self.storage.load_index()?;
        let new_disk_active_phase = &self.index.project.project.active_phase;
        if self.index.phase(&self.active_phase_id).is_none()
            || (self.active_phase_id == prev_disk_active_phase
                && new_disk_active_phase != &prev_disk_active_phase
                && self.index.phase(new_disk_active_phase).is_some())
        {
            self.active_phase_id = new_disk_active_phase.clone();
        }
        if let Some(ref scope_id) = self.selected_scope {
            let scope_exists = self
                .active_phase()
                .is_some_and(|p| p.scope(scope_id).is_some());
            if !scope_exists {
                self.selected_scope = None;
            }
        }
        let (phase_ids, phase_labels) = build_phase_dropdown_lists(&self.index);
        self.phase_ids = phase_ids;
        self.phase_labels = phase_labels;
        self.active_timer = self.storage.timer_status().ok().flatten();
        self.git_status = self.storage.git_status().ok();
        self.settings_form.has_stored_pat = self.storage.get_pat().is_ok();

        if let Some(selected_id) = self.selected_item_id.clone() {
            let prev_disk_title = self
                .selected_item_detail
                .as_ref()
                .map(|d| d.manifest.item.title.clone());
            if let Ok((loc, detail)) = self.storage.load_item_detail_by_id(&selected_id) {
                let signal = compute_signal_from_manifest(
                    &detail.manifest,
                    Utc::now(),
                    i64::from(self.stagnant_threshold_days()),
                );
                self.card_signals.insert(loc.stub.id.clone(), signal);
                if prev_disk_title.as_deref() != Some(detail.manifest.item.title.as_str()) {
                    self.drawer_title_input = detail.manifest.item.title.clone();
                }
                if !self.desc_editor_dirty {
                    self.desc_editor_content =
                        widget::text_editor::Content::with_text(&detail.description_markdown);
                }
                self.selected_item_location = Some(loc);
                self.selected_item_detail = Some(detail);
            } else {
                self.selected_item_id = None;
                self.selected_item_location = None;
                self.selected_item_detail = None;
                self.desc_editor_dirty = false;
            }
        }

        Ok(())
    }

    /// Lazily loads `item.toml` and `desc.md` for `item_id` when the user clicks a card.
    pub fn select_card_and_lazy_load(&mut self, item_id: &str) -> Result<()> {
        let (loc, detail) = self.storage.load_item_detail_by_id(item_id)?;
        self.lazy_detail_loads_count += 1;

        if loc.phase_id != self.active_phase_id && self.index.phase(&loc.phase_id).is_some() {
            self.active_phase_id = loc.phase_id.clone();
        }

        let signal = compute_signal_from_manifest(
            &detail.manifest,
            Utc::now(),
            i64::from(self.stagnant_threshold_days()),
        );
        self.card_signals.insert(loc.stub.id.clone(), signal);
        self.drawer_title_input = detail.manifest.item.title.clone();
        self.desc_editor_content =
            widget::text_editor::Content::with_text(&detail.description_markdown);
        self.desc_editor_dirty = false;
        self.drawer_close_summary_input.clear();
        self.drawer_close_commit_input.clear();
        self.drawer_reopen_reason_input.clear();
        self.selected_item_id = Some(loc.stub.id.clone());
        self.selected_item_location = Some(loc);
        self.selected_item_detail = Some(detail);
        Ok(())
    }

    fn flush_dirty_desc_if_needed(&mut self, item_id: &str) -> Result<()> {
        if self.desc_editor_dirty {
            let updated_md = self.desc_editor_content.text();
            let detail = self.storage.update_item_description(item_id, updated_md)?;
            self.desc_editor_dirty = false;
            self.selected_item_detail = Some(detail);
        }
        Ok(())
    }

    fn copy_text_to_clipboard(&mut self, payload: String, toast_msg: String) -> Task<Message> {
        self.last_copied_clipboard = Some(payload.clone());
        if self.clipboard_handle.is_none() {
            self.clipboard_handle = arboard::Clipboard::new().ok();
        }
        if let Some(ref mut cb) = self.clipboard_handle {
            let _ = cb.set_text(payload.clone());
        }
        self.status_banner = Some(toast_msg);
        cosmic::iced::clipboard::write(payload)
    }
}

impl Application for NebulousApp {
    type Executor = executor::Default;
    type Flags = AppFlags;
    type Message = Message;

    const APP_ID: &'static str = "com.nebulous.Tracker";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, flags: Self::Flags) -> (Self, Task<Self::Message>) {
        core.window.header_title = "Nebulous — Project & Bug Tracker".to_string();

        let storage = Self::resolve_or_init_storage(flags.workspace_path.as_deref())
            .unwrap_or_else(|_| TrackerStorage::new("."));

        let app = Self::new_from_storage(core, storage).unwrap_or_else(|err| {
            let fallback_storage = TrackerStorage::new(".");
            let empty_project = backend_core::ProjectManifest {
                project: backend_core::ProjectMetadata {
                    name: "Nebulous".to_string(),
                    version: "0.1.0".to_string(),
                    active_phase: "01_core_engine".to_string(),
                    default_scope: "inbox".to_string(),
                    created_at: "2026-11-01T00:00:00Z".to_string(),
                },
                counters: Default::default(),
                phases: Default::default(),
                metrics: Default::default(),
                git: Default::default(),
            };
            let empty_index = TrackerIndex {
                project: empty_project,
                phases: Vec::new(),
            };
            let settings_form =
                SettingsFormState::from_index_and_storage(&empty_index, &fallback_storage);
            Self {
                core: Core::default(),
                storage: fallback_storage,
                index: empty_index,
                active_phase_id: "01_core_engine".to_string(),
                phase_ids: Vec::new(),
                phase_labels: Vec::new(),
                selected_scope: None,
                active_filter: FilterPill::All,
                view_mode: ViewMode::Board,
                selected_item_id: None,
                selected_item_location: None,
                selected_item_detail: None,
                lazy_detail_loads_count: 0,
                drawer_title_input: String::new(),
                desc_editor_content: widget::text_editor::Content::new(),
                desc_editor_dirty: false,
                drawer_manual_minutes_input: "15".to_string(),
                drawer_close_summary_input: String::new(),
                drawer_close_commit_input: String::new(),
                drawer_reopen_reason_input: String::new(),
                last_copied_clipboard: None,
                clipboard_handle: None,
                card_signals: HashMap::new(),
                active_timer: None,
                git_status: None,
                quick_add: QuickAddState::default(),
                settings_form,
                status_banner: Some(format!("Failed to load .tracker index: {err}")),
            }
        });

        (app, Task::none())
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        subscriptions::subscription(self)
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::SelectPhaseIndex(idx) => {
                if let Some(phase_id) = self.phase_ids.get(idx).cloned() {
                    return self.update(Message::SelectPhase(phase_id));
                }
            }
            Message::SelectPhase(phase_id) => {
                if self.index.phase(&phase_id).is_some() {
                    let _ = self.storage.set_active_phase(&phase_id);
                    self.active_phase_id = phase_id;
                    self.selected_scope = None;
                    let _ = self.reload_indexes();
                }
            }
            Message::SelectScope(scope_opt) => {
                self.selected_scope = scope_opt;
            }
            Message::SelectScopeAndOpenBoard(scope_id) => {
                let in_active = self
                    .active_phase()
                    .is_some_and(|p| p.scope(&scope_id).is_some());
                if !in_active {
                    if let Some(found_phase) = self
                        .index
                        .phases
                        .iter()
                        .find(|p| p.scope(&scope_id).is_some())
                        .map(|p| p.phase_id.clone())
                    {
                        let _ = self.storage.set_active_phase(&found_phase);
                        self.active_phase_id = found_phase;
                        let _ = self.reload_indexes();
                    }
                }
                self.selected_scope = Some(scope_id);
                self.view_mode = ViewMode::Board;
            }
            Message::SelectPhaseScopeAndOpenBoard { phase_id, scope_id } => {
                if self.index.phase(&phase_id).is_some() && self.active_phase_id != phase_id {
                    let _ = self.storage.set_active_phase(&phase_id);
                    self.active_phase_id = phase_id;
                    let _ = self.reload_indexes();
                }
                self.selected_scope = Some(scope_id);
                self.view_mode = ViewMode::Board;
            }
            Message::SwitchView(mode) => {
                self.view_mode = mode;
            }
            Message::SetFilterPill(pill) => {
                self.active_filter = pill;
            }
            Message::SelectCard(item_id) => {
                if let Err(err) = self.select_card_and_lazy_load(&item_id) {
                    self.status_banner = Some(format!("Failed to load {item_id}: {err}"));
                }
            }
            Message::ClearSelectedCard => {
                self.selected_item_id = None;
                self.selected_item_location = None;
                self.selected_item_detail = None;
                self.desc_editor_dirty = false;
            }
            Message::OpenQuickAdd(item_type) => {
                self.quick_add.open = true;
                self.quick_add.item_type = item_type;
            }
            Message::CloseQuickAdd => {
                self.quick_add.open = false;
                self.quick_add.title.clear();
            }
            Message::SetQuickAddType(item_type) => {
                self.quick_add.item_type = item_type;
            }
            Message::SetQuickAddSeverity(severity) => {
                self.quick_add.severity = severity;
            }
            Message::QuickAddTitleChanged(title) => {
                self.quick_add.title = title;
            }
            Message::SubmitQuickAdd => {
                let title = self.quick_add.title.trim().to_string();
                if title.is_empty() {
                    self.status_banner = Some("Item title cannot be empty.".to_string());
                    return Task::none();
                }

                let input = CreateItemInput {
                    item_type: self.quick_add.item_type,
                    title,
                    phase_id: Some(self.active_phase_id.clone()),
                    scope_id: self.selected_scope.clone(),
                    severity: self.quick_add.severity,
                    description: None,
                };

                match self.storage.create_item(input) {
                    Ok((loc, detail)) => {
                        let created_id = loc.stub.id.clone();
                        self.quick_add.open = false;
                        self.quick_add.title.clear();
                        self.drawer_title_input = detail.manifest.item.title.clone();
                        self.desc_editor_content =
                            widget::text_editor::Content::with_text(&detail.description_markdown);
                        self.desc_editor_dirty = false;
                        self.selected_item_id = Some(created_id.clone());
                        self.selected_item_location = Some(loc.clone());
                        self.selected_item_detail = Some(detail);
                        let _ = self.reload_indexes();
                        self.status_banner = Some(format!(
                            "Created {} in {}/{}",
                            created_id, loc.phase_id, loc.scope_id
                        ));
                    }
                    Err(err) => {
                        self.status_banner = Some(format!("Failed to create item: {err}"));
                    }
                }
            }
            Message::QuickStatusTransition {
                item_id,
                new_status,
            } => {
                if self.selected_item_detail.as_ref().is_some_and(|d| {
                    d.manifest.item.id == item_id && d.manifest.item.status == new_status
                }) {
                    return Task::none();
                }
                if new_status.is_completed() && self.is_timer_running_for(&item_id) {
                    let _ = self.storage.stop_timer();
                }
                match self.storage.update_item_status(&item_id, new_status) {
                    Ok(detail) => {
                        let signal = compute_signal_from_manifest(
                            &detail.manifest,
                            Utc::now(),
                            i64::from(self.stagnant_threshold_days()),
                        );
                        self.card_signals.insert(item_id.clone(), signal);
                        let _ = self.reload_indexes();
                        self.status_banner =
                            Some(format!("Updated {item_id} status to {new_status}"));
                    }
                    Err(err) => {
                        self.status_banner =
                            Some(format!("Failed to update {item_id} status: {err}"));
                    }
                }
            }
            Message::UpdateSelectedSeverity(new_severity) => {
                if self
                    .selected_item_detail
                    .as_ref()
                    .is_some_and(|d| d.manifest.item.severity == new_severity)
                {
                    return Task::none();
                }
                if let Some(item_id) = self.selected_item_id.clone() {
                    match self
                        .storage
                        .update_item_metadata(&item_id, None, Some(new_severity))
                    {
                        Ok(_) => {
                            let _ = self.reload_indexes();
                            self.status_banner =
                                Some(format!("Updated {item_id} severity to {new_severity}"));
                        }
                        Err(err) => {
                            self.status_banner =
                                Some(format!("Failed to update {item_id} severity: {err}"));
                        }
                    }
                }
            }
            Message::StartTimer(item_id) => {
                if self.active_timer.is_some() {
                    let _ = self.storage.stop_timer();
                }
                match self.storage.start_timer(&item_id, None) {
                    Ok(_) => {
                        let _ = self.reload_indexes();
                        self.status_banner = Some(format!("Started session timer for {item_id}"));
                    }
                    Err(err) => {
                        self.status_banner = Some(format!("Could not start timer: {err}"));
                    }
                }
            }
            Message::StopTimer => match self.storage.stop_timer() {
                Ok(stopped) => {
                    let _ = self.reload_indexes();
                    self.status_banner = Some(format!(
                        "Stopped timer for {} (+{}m logged, total {}m)",
                        stopped.timer.item_id,
                        stopped.elapsed_minutes,
                        stopped.total_logged_minutes
                    ));
                }
                Err(err) => {
                    self.status_banner = Some(format!("Could not stop timer: {err}"));
                }
            },
            Message::TimerTick => {
                self.active_timer = self.storage.timer_status().ok().flatten();
            }
            Message::TriggerGitSync => match self.storage.git_sync(None, None) {
                Ok(Some(sha)) => {
                    let _ = self.reload_indexes();
                    self.status_banner = Some(format!("Git synced .tracker/ at commit {sha}"));
                }
                Ok(None) => {
                    let _ = self.reload_indexes();
                    self.status_banner =
                        Some("Git sync complete (.tracker/ already clean)".to_string());
                }
                Err(err) => {
                    self.status_banner = Some(format!("Git sync note: {err}"));
                }
            },
            Message::CommitTrackerNow => match self.storage.git_commit_tracker(None) {
                Ok(Some(sha)) => {
                    let _ = self.reload_indexes();
                    self.status_banner = Some(format!("Committed .tracker/ changes at {sha}"));
                }
                Ok(None) => {
                    let _ = self.reload_indexes();
                    self.status_banner = Some("No dirty .tracker/ changes to commit.".to_string());
                }
                Err(err) => {
                    self.status_banner = Some(format!("Failed to commit .tracker/: {err}"));
                }
            },
            Message::ReconcileIndexes => match self.storage.reconcile_indexes() {
                Ok(report) => {
                    let _ = self.reload_indexes();
                    self.status_banner = Some(format!(
                        "Reconciled {} phases, {} scopes, {} items",
                        report.phases_reconciled, report.scopes_reconciled, report.total_items
                    ));
                }
                Err(err) => {
                    self.status_banner = Some(format!("Index reconciliation failed: {err}"));
                }
            },
            Message::DismissBanner => {
                self.status_banner = None;
            }

            // Context Drawer updates
            Message::DrawerTitleChanged(title) => {
                self.drawer_title_input = title;
            }
            Message::SaveDrawerTitle => {
                if let Some(item_id) = self.selected_item_id.clone() {
                    let new_title = self.drawer_title_input.trim().to_string();
                    match self
                        .storage
                        .update_item_metadata(&item_id, Some(new_title), None)
                    {
                        Ok(_) => {
                            let _ = self.reload_indexes();
                            self.status_banner = Some(format!("Saved title for {item_id}"));
                        }
                        Err(err) => {
                            self.status_banner =
                                Some(format!("Failed to update title for {item_id}: {err}"));
                        }
                    }
                }
            }
            Message::DescEditorAction(action) => {
                let is_edit = action.is_edit();
                self.desc_editor_content.perform(action);
                if is_edit {
                    self.desc_editor_dirty = true;
                }
            }
            Message::SetDescMarkdown(markdown) => {
                self.desc_editor_content = widget::text_editor::Content::with_text(&markdown);
                self.desc_editor_dirty = true;
            }
            Message::AppendDescTemplate(snippet) => {
                let mut current = self.desc_editor_content.text();
                if !current.ends_with('\n') && !current.is_empty() {
                    current.push('\n');
                }
                current.push_str(snippet.trim_start_matches('\n'));
                current.push('\n');
                self.desc_editor_content = widget::text_editor::Content::with_text(&current);
                self.desc_editor_dirty = true;
            }
            Message::SaveDescMarkdown => {
                if let Some(item_id) = self.selected_item_id.clone() {
                    let updated_md = self.desc_editor_content.text();
                    match self.storage.update_item_description(&item_id, updated_md) {
                        Ok(detail) => {
                            self.desc_editor_dirty = false;
                            self.selected_item_detail = Some(detail);
                            let _ = self.reload_indexes();
                            self.status_banner =
                                Some(format!("Atomically saved desc.md for {item_id}"));
                        }
                        Err(err) => {
                            self.status_banner =
                                Some(format!("Failed to save desc.md for {item_id}: {err}"));
                        }
                    }
                }
            }
            Message::OpenInExternalEditor => {
                if let Some(item_id) = self.selected_item_id.clone() {
                    let _ = self.flush_dirty_desc_if_needed(&item_id);
                }
                if let Some(ref loc) = self.selected_item_location {
                    if let Ok(desc_path) =
                        self.storage
                            .item_desc_path(&loc.phase_id, &loc.scope_id, &loc.stub.id)
                    {
                        let editor = std::env::var("VISUAL")
                            .or_else(|_| std::env::var("EDITOR"))
                            .unwrap_or_else(|_| "xdg-open".to_string());
                        let mut parts = editor.split_whitespace();
                        if let Some(program) = parts.next() {
                            let args: Vec<&str> = parts.collect();
                            match Command::new(program).args(args).arg(&desc_path).spawn() {
                                Ok(_) => {
                                    self.status_banner = Some(format!(
                                        "Launched '{editor}' for {}",
                                        desc_path.display()
                                    ));
                                }
                                Err(err) => {
                                    self.status_banner =
                                        Some(format!("Could not launch editor '{editor}': {err}"));
                                }
                            }
                        }
                    }
                }
            }
            Message::ManualMinutesInputChanged(val) => {
                self.drawer_manual_minutes_input = val;
            }
            Message::LogManualMinutes(minutes) => {
                if let Some(item_id) = self.selected_item_id.clone() {
                    match self.storage.log_effort(&item_id, minutes) {
                        Ok(detail) => {
                            let total = detail.manifest.effort.logged_time_minutes;
                            let signal = compute_signal_from_manifest(
                                &detail.manifest,
                                Utc::now(),
                                i64::from(self.stagnant_threshold_days()),
                            );
                            self.card_signals.insert(item_id.clone(), signal);
                            let _ = self.reload_indexes();
                            self.status_banner =
                                Some(format!("Logged +{minutes}m on {item_id} (total: {total}m)"));
                        }
                        Err(err) => {
                            self.status_banner =
                                Some(format!("Failed to log effort on {item_id}: {err}"));
                        }
                    }
                }
            }
            Message::SubmitCustomManualMinutes => {
                match self.drawer_manual_minutes_input.trim().parse::<u32>() {
                    Ok(mins) if mins > 0 => {
                        return self.update(Message::LogManualMinutes(mins));
                    }
                    _ => {
                        self.status_banner =
                            Some("Please enter a positive integer number of minutes.".to_string());
                    }
                }
            }
            Message::CloseSummaryInputChanged(val) => {
                self.drawer_close_summary_input = val;
            }
            Message::CloseCommitInputChanged(val) => {
                self.drawer_close_commit_input = val;
            }
            Message::CloseSelectedWithSummary { merged } => {
                if let Some(item_id) = self.selected_item_id.clone() {
                    let summary = self.drawer_close_summary_input.trim().to_string();
                    let commit_override = {
                        let trimmed = self.drawer_close_commit_input.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_string())
                        }
                    };

                    if !summary.is_empty() && self.is_timer_running_for(&item_id) {
                        let _ = self.storage.stop_timer();
                    }

                    match self.storage.close_item(
                        &item_id,
                        summary,
                        commit_override.as_deref(),
                        merged,
                    ) {
                        Ok(detail) => {
                            let recorded_sha = detail
                                .manifest
                                .history
                                .previous_fixes
                                .last()
                                .map(|f| f.commit.clone())
                                .unwrap_or_else(|| "uncommitted".to_string());
                            self.drawer_close_summary_input.clear();
                            self.drawer_close_commit_input.clear();
                            let _ = self.reload_indexes();
                            let verb = if merged { "Merged" } else { "Closed" };
                            self.status_banner = Some(format!(
                                "{verb} {item_id} and recorded fix in ledger (commit {recorded_sha})"
                            ));
                        }
                        Err(err) => {
                            self.status_banner = Some(format!("Cannot close {item_id}: {err}"));
                        }
                    }
                }
            }
            Message::ReopenReasonInputChanged(val) => {
                self.drawer_reopen_reason_input = val;
            }
            Message::ReopenSelectedItem => {
                if let Some(item_id) = self.selected_item_id.clone() {
                    let is_completed = self
                        .selected_item_detail
                        .as_ref()
                        .is_some_and(|d| d.manifest.item.status.is_completed());
                    if is_completed {
                        let _ = self.flush_dirty_desc_if_needed(&item_id);
                    }
                    let reason = {
                        let trimmed = self.drawer_reopen_reason_input.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_string())
                        }
                    };
                    match self.storage.reopen_item(&item_id, reason.as_deref()) {
                        Ok(detail) => {
                            let recurrences = detail.manifest.history.recurrence_count;
                            let iterations = detail.manifest.effort.iteration_count;
                            self.drawer_reopen_reason_input.clear();
                            self.desc_editor_dirty = false;
                            let _ = self.reload_indexes();
                            self.status_banner = Some(format!(
                                "Reopened {item_id} (recurrence_count = {recurrences}, iteration_count = {iterations})"
                            ));
                        }
                        Err(err) => {
                            self.status_banner = Some(format!("Cannot reopen {item_id}: {err}"));
                        }
                    }
                }
            }
            Message::CopyAgentFileRefs => {
                if let Some(item_id) = self.selected_item_id.clone() {
                    match self.storage.export_file_refs(&item_id) {
                        Ok(refs) => {
                            let toast = format!("Copied @file references for {item_id}: {refs}");
                            return self.copy_text_to_clipboard(refs, toast);
                        }
                        Err(err) => {
                            self.status_banner =
                                Some(format!("Failed to export @file refs for {item_id}: {err}"));
                        }
                    }
                }
            }
            Message::CopyBoundedAgentPrompt => {
                if let Some(item_id) = self.selected_item_id.clone() {
                    if let Err(err) = self.flush_dirty_desc_if_needed(&item_id) {
                        self.status_banner =
                            Some(format!("Failed to save desc.md for {item_id}: {err}"));
                        return Task::none();
                    }
                    match self.storage.export_bounded_prompt(&item_id) {
                        Ok(prompt) => {
                            let char_len = prompt.len();
                            let toast = format!(
                                "Copied Bounded Agent Prompt for {item_id} ({char_len} bytes) to clipboard"
                            );
                            return self.copy_text_to_clipboard(prompt, toast);
                        }
                        Err(err) => {
                            self.status_banner = Some(format!(
                                "Failed to export Bounded Agent Prompt for {item_id}: {err}"
                            ));
                        }
                    }
                }
            }

            // Settings & Phase/Scope Manager updates
            Message::NewPhaseIdChanged(val) => {
                self.settings_form.new_phase_id = val;
            }
            Message::NewPhaseTitleChanged(val) => {
                self.settings_form.new_phase_title = val;
            }
            Message::ToggleNewPhaseSetActive(active) => {
                self.settings_form.new_phase_set_active = active;
            }
            Message::CreatePhase => {
                let phase_id = self.settings_form.new_phase_id.trim().to_string();
                let title = self.settings_form.new_phase_title.trim().to_string();
                let set_active = self.settings_form.new_phase_set_active;
                match self.storage.create_phase(&phase_id, &title, set_active) {
                    Ok(_) => {
                        if set_active {
                            self.active_phase_id = phase_id.clone();
                        }
                        self.settings_form.new_phase_id.clear();
                        self.settings_form.new_phase_title.clear();
                        self.settings_form.new_phase_set_active = false;
                        let _ = self.reload_indexes();
                        self.status_banner = Some(format!("Created phase '{phase_id}' ({title})"));
                    }
                    Err(err) => {
                        self.status_banner = Some(format!("Failed to create phase: {err}"));
                    }
                }
            }
            Message::NewScopeIdChanged(val) => {
                self.settings_form.new_scope_id = val;
            }
            Message::NewScopeTargetChanged(val) => {
                self.settings_form.new_scope_target = val;
            }
            Message::NewScopeTechStackChanged(val) => {
                self.settings_form.new_scope_tech_stack = val;
            }
            Message::NewScopeDescriptionChanged(val) => {
                self.settings_form.new_scope_description = val;
            }
            Message::CreateScope => {
                let scope_id = self.settings_form.new_scope_id.trim().to_string();
                let target = self.settings_form.new_scope_target.trim().to_string();
                let tech_stack: Vec<String> = self
                    .settings_form
                    .new_scope_tech_stack
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
                let description = {
                    let trimmed = self.settings_form.new_scope_description.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                };
                let phase_id = self.active_phase_id.clone();
                match self.storage.create_scope(
                    Some(&phase_id),
                    &scope_id,
                    target,
                    tech_stack,
                    description,
                ) {
                    Ok(_) => {
                        self.settings_form.new_scope_id.clear();
                        self.settings_form.new_scope_target.clear();
                        self.settings_form.new_scope_tech_stack.clear();
                        self.settings_form.new_scope_description.clear();
                        let _ = self.reload_indexes();
                        self.status_banner = Some(format!(
                            "Created domain scope '{scope_id}' in phase '{phase_id}'"
                        ));
                    }
                    Err(err) => {
                        self.status_banner = Some(format!("Failed to create scope: {err}"));
                    }
                }
            }
            Message::TargetDailyHoursChanged(val) => {
                self.settings_form.target_daily_hours_input = val;
            }
            Message::StagnantThresholdDaysChanged(val) => {
                self.settings_form.stagnant_threshold_days_input = val;
            }
            Message::DefaultScopeChanged(val) => {
                self.settings_form.default_scope_input = val;
            }
            Message::SaveProjectAndMetricsConfig => {
                let Ok(target_hours) = self
                    .settings_form
                    .target_daily_hours_input
                    .trim()
                    .parse::<f64>()
                else {
                    self.status_banner =
                        Some("Target daily hours must be a valid positive number.".to_string());
                    return Task::none();
                };
                if !target_hours.is_finite() || target_hours <= 0.0 {
                    self.status_banner = Some(
                        "Target daily hours must be a finite number greater than 0.".to_string(),
                    );
                    return Task::none();
                }

                let Ok(threshold_days) = self
                    .settings_form
                    .stagnant_threshold_days_input
                    .trim()
                    .parse::<u32>()
                else {
                    self.status_banner =
                        Some("Stagnant threshold days must be a valid integer.".to_string());
                    return Task::none();
                };

                let default_scope = self.settings_form.default_scope_input.trim().to_string();
                if default_scope.is_empty() {
                    self.status_banner = Some("Default scope must not be empty.".to_string());
                    return Task::none();
                }

                match self.storage.read_project() {
                    Ok(mut project) => {
                        project.metrics.target_daily_hours = target_hours;
                        project.metrics.stagnant_item_threshold_days = threshold_days;
                        project.project.default_scope = default_scope.clone();
                        match self.storage.write_project(&project) {
                            Ok(()) => {
                                let _ = self.reload_indexes();
                                self.settings_form
                                    .sync_from_index(&self.index, &self.storage);
                                self.status_banner = Some(format!(
                                    "Saved metrics config (target={target_hours:.1}h, stagnant={threshold_days}d, default_scope='{default_scope}')"
                                ));
                            }
                            Err(err) => {
                                self.status_banner =
                                    Some(format!("Failed to write project.toml: {err}"));
                            }
                        }
                    }
                    Err(err) => {
                        self.status_banner = Some(format!("Failed to read project.toml: {err}"));
                    }
                }
            }
            Message::SetGitSyncMode(mode) => {
                self.settings_form.git_sync_mode_input = mode;
            }
            Message::GitSyncIntervalChanged(val) => {
                self.settings_form.git_sync_interval_input = val;
            }
            Message::GitRemoteChanged(val) => {
                self.settings_form.git_remote_input = val;
            }
            Message::GitBranchChanged(val) => {
                self.settings_form.git_branch_input = val;
            }
            Message::SaveGitConfig => {
                let Ok(interval_mins) = self
                    .settings_form
                    .git_sync_interval_input
                    .trim()
                    .parse::<u32>()
                else {
                    self.status_banner =
                        Some("Sync interval minutes must be a positive integer.".to_string());
                    return Task::none();
                };
                if interval_mins == 0 {
                    self.status_banner =
                        Some("Sync interval minutes must be greater than 0.".to_string());
                    return Task::none();
                }
                let remote = self.settings_form.git_remote_input.trim().to_string();
                let branch = self.settings_form.git_branch_input.trim().to_string();
                if remote.is_empty() || branch.is_empty() {
                    self.status_banner =
                        Some("Git remote and branch names must not be empty.".to_string());
                    return Task::none();
                }

                match self.storage.read_project() {
                    Ok(mut project) => {
                        project.git.sync_mode = self.settings_form.git_sync_mode_input;
                        project.git.sync_interval_minutes = interval_mins;
                        project.git.remote = remote.clone();
                        project.git.branch = branch.clone();
                        match self.storage.write_project(&project) {
                            Ok(()) => {
                                let _ = self.reload_indexes();
                                self.settings_form
                                    .sync_from_index(&self.index, &self.storage);
                                self.status_banner = Some(format!(
                                    "Saved Git sync settings (mode={}, interval={}m, {remote}/{branch})",
                                    project.git.sync_mode, project.git.sync_interval_minutes
                                ));
                            }
                            Err(err) => {
                                self.status_banner =
                                    Some(format!("Failed to save Git settings: {err}"));
                            }
                        }
                    }
                    Err(err) => {
                        self.status_banner = Some(format!("Failed to read project.toml: {err}"));
                    }
                }
            }
            Message::PatInputChanged(val) => {
                self.settings_form.pat_input = val;
            }
            Message::SavePatToKeyring => {
                let token = self.settings_form.pat_input.trim().to_string();
                match self.storage.store_pat(&token) {
                    Ok(()) => {
                        self.settings_form.pat_input.clear();
                        self.settings_form.has_stored_pat = true;
                        self.status_banner =
                            Some("Saved Personal Access Token (PAT) in OS keyring.".to_string());
                    }
                    Err(err) => {
                        self.status_banner = Some(format!("Failed to store PAT in keyring: {err}"));
                    }
                }
            }
            Message::DeletePatFromKeyring => match self.storage.delete_pat() {
                Ok(()) => {
                    self.settings_form.pat_input.clear();
                    self.settings_form.has_stored_pat = false;
                    self.status_banner =
                        Some("Cleared Personal Access Token (PAT) from OS keyring.".to_string());
                }
                Err(err) => {
                    self.settings_form.has_stored_pat = false;
                    self.status_banner = Some(format!("Keyring PAT note: {err}"));
                }
            },

            // Reactive background subscription handlers
            Message::TrackerFilesChanged(_changed_paths) => {
                let _ = self.storage.reconcile_indexes();
                self.card_signals.clear();
                let _ = self.reload_indexes();
                self.settings_form
                    .sync_from_index(&self.index, &self.storage);
            }
            Message::ScheduledGitSyncTick => {
                if self.index.project.git.sync_mode == GitSyncMode::Scheduled {
                    if let Ok(Some(sha)) = self
                        .storage
                        .git_sync(Some("chore(tracker): scheduled background sync"), None)
                    {
                        self.status_banner = Some(format!("Scheduled Git sync completed at {sha}"));
                    }
                    let _ = self.reload_indexes();
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let top_bar = header_bar::view_header_bar(self);
        let left_sidebar = nav_sidebar::view_nav_sidebar(self);

        let central_pane: Element<'_, Self::Message> = match self.view_mode {
            ViewMode::Board => kanban_board::view_kanban_board(self),
            ViewMode::Metrics => metrics_view::view_metrics_dashboard(self),
            ViewMode::PhasesAndScopes | ViewMode::Settings => {
                settings_view::view_settings_or_phases(self)
            }
        };

        let mut body_row = widget::row::with_capacity(3)
            .push(left_sidebar)
            .push(central_pane)
            .spacing(12)
            .width(Length::Fill)
            .height(Length::Fill);

        if let Some(drawer) = detail_drawer::view_detail_drawer(self) {
            body_row = body_row.push(drawer);
        }

        let mut root_col = widget::column::with_capacity(3)
            .push(top_bar)
            .spacing(10)
            .width(Length::Fill)
            .height(Length::Fill);

        if let Some(ref banner_msg) = self.status_banner {
            let banner = widget::container(
                widget::row::with_capacity(3)
                    .push(widget::text::caption(banner_msg.clone()))
                    .push(widget::space::horizontal().width(Length::Fill))
                    .push(
                        widget::button::text("Dismiss")
                            .padding([2, 8])
                            .height(Length::Fixed(22.0))
                            .on_press(Message::DismissBanner),
                    )
                    .align_y(Alignment::Center)
                    .width(Length::Fill),
            )
            .padding([6, 12])
            .width(Length::Fill)
            .class(cosmic::theme::Container::Secondary);

            root_col = root_col.push(banner);
        }

        root_col = root_col.push(body_row);

        widget::container(root_col)
            .padding([10, 12])
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn build_phase_dropdown_lists(index: &TrackerIndex) -> (Vec<String>, Vec<String>) {
    let mut ids = Vec::with_capacity(index.phases.len());
    let mut labels = Vec::with_capacity(index.phases.len());
    for phase in &index.phases {
        ids.push(phase.phase_id.clone());
        labels.push(format!(
            "{} — {}",
            phase.phase_id, phase.manifest.phase.title
        ));
    }
    (ids, labels)
}

fn compute_signal_from_manifest(
    manifest: &ItemManifest,
    now: chrono::DateTime<Utc>,
    stagnant_threshold_days: i64,
) -> CardSignalSummary {
    let stagnant_days = if !manifest.item.status.is_completed() {
        manifest.item.parsed_updated_at().ok().and_then(|updated| {
            let days = (now - updated).num_days();
            (days >= stagnant_threshold_days).then_some(days)
        })
    } else {
        None
    };

    CardSignalSummary {
        stagnant_days,
        recurrence_count: manifest.history.recurrence_count,
        logged_time_minutes: manifest.effort.logged_time_minutes,
        iteration_count: manifest.effort.iteration_count,
        previous_fixes_count: manifest.history.previous_fixes.len(),
    }
}

/// Runs the headless `--self-test` verification suite without requiring a Wayland/X11 display server.
///
/// Verifies:
/// 1. Startup loads only top-level `.tracker/` indexes (`project.toml`, `phase.toml`, `scope.toml`)
///    with `selected_item_detail == None` and `lazy_detail_loads_count == 0`.
/// 2. Filtering by domain scope (`backend_core`, `cosmic_interface`, etc.) and top-bar filter pills
///    (`All`, `Bugs`, `Features`, `HighOrCritical`, `Stagnant`, `Recurring`).
/// 3. Selecting a card lazily loads `item.toml` + `desc.md` into `selected_item_detail` and initializes
///    the `cosmic::widget::text_editor` Markdown buffer (`desc_editor_content`).
/// 4. Markdown editing state (`AppendDescTemplate` / `SetDescMarkdown`) updates `desc_editor_content`
///    and marks `desc_editor_dirty = true`.
/// 5. Agent Context Handoff buttons (`CopyAgentFileRefs` & `CopyBoundedAgentPrompt`) synthesize valid
///    `@file` reference strings and Bounded Agent Prompts containing crate scope boundaries and
///    historical fix ledger warnings.
/// 6. Metrics view (`compute_current_metrics()` & `ViewMode::Metrics`), Settings view (`ViewMode::Settings`),
///    and reactive background subscriptions (`subscriptions::subscription(&app)`) construct cleanly in memory.
struct SelfTestTempGuard {
    path: PathBuf,
}

impl Drop for SelfTestTempGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn seed_self_test_temp_workspace() -> Result<(SelfTestTempGuard, TrackerStorage)> {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("nebulous_self_test_{pid}_{nanos}"));
    if path.exists() {
        let _ = std::fs::remove_dir_all(&path);
    }
    std::fs::create_dir_all(&path).map_err(|source| TrackerError::Io {
        path: path.clone(),
        source,
    })?;
    let guard = SelfTestTempGuard { path: path.clone() };

    let storage = TrackerStorage::init(
        &path,
        "Nebulous",
        "01_core_engine",
        "Core Engine & Persistence",
    )?;
    storage.create_scope(
        Some("01_core_engine"),
        "backend_core",
        "crates/backend_core",
        vec!["rust".to_string(), "serde".to_string()],
        Some("Core persistence and workflow engine".to_string()),
    )?;
    storage.create_scope(
        Some("01_core_engine"),
        "cosmic_interface",
        "crates/cosmic_interface",
        vec!["rust".to_string(), "libcosmic".to_string()],
        Some("Native libcosmic desktop GUI".to_string()),
    )?;
    storage.create_bug(
        "Empty array deserialization failure",
        Some("01_core_engine"),
        Some("backend_core"),
        Severity::High,
        Some("Parser fails when bugs = [] in scope.toml"),
    )?;
    storage.create_feature(
        "4-column Kanban card board",
        Some("01_core_engine"),
        Some("cosmic_interface"),
        Severity::Critical,
        Some("Render Open, In Progress, Pending, Closed / Merged columns"),
    )?;

    Ok((guard, storage))
}

pub fn run_headless_self_test(workspace_path: Option<&Path>) -> Result<()> {
    let (_temp_guard, storage) = match workspace_path {
        Some(path) => match TrackerStorage::discover(path) {
            Ok(s) => (None, s),
            Err(TrackerError::TrackerRootNotFound(_)) => {
                let (guard, s) = seed_self_test_temp_workspace()?;
                (Some(guard), s)
            }
            Err(err) => return Err(err),
        },
        None => {
            let (guard, s) = seed_self_test_temp_workspace()?;
            (Some(guard), s)
        }
    };
    let mut app = NebulousApp::new_from_storage(Core::default(), storage)?;

    // 1. Verify startup reads only top-level indexes and does NOT eagerly load item detail (`desc.md`).
    if app.selected_item_detail.is_some() || app.lazy_detail_loads_count != 0 {
        return Err(TrackerError::ValidationError(
            "Startup must not eagerly load item.toml + desc.md details".to_string(),
        ));
    }
    if app.index.phases.is_empty() {
        return Err(TrackerError::ValidationError(
            "Expected at least one phase in .tracker/project.toml".to_string(),
        ));
    }

    // Build initial Board view tree and subscription in memory (without drawer)
    {
        let _board_view_initial = app.view();
        let _initial_sub = app.subscription();
    }

    let all_cards = app.filtered_kanban_cards();
    if all_cards.is_empty() {
        return Err(TrackerError::ValidationError(
            "Expected Kanban board to contain tracked items from .tracker/".to_string(),
        ));
    }

    // 2. Exercise top-bar filter pills
    for pill in [
        FilterPill::Bugs,
        FilterPill::Features,
        FilterPill::HighOrCritical,
        FilterPill::Stagnant,
        FilterPill::Recurring,
        FilterPill::All,
    ] {
        let _ = app.update(Message::SetFilterPill(pill));
        let _filtered = app.filtered_kanban_cards();
        let _view = app.view();
    }

    // 3. Exercise left-sidebar domain scope filtering
    if let Some(first_scope) = app
        .active_phase()
        .and_then(|p| p.scopes.first())
        .map(|s| s.scope_id.clone())
    {
        let _ = app.update(Message::SelectScope(Some(first_scope.clone())));
        let scoped_cards = app.filtered_kanban_cards();
        if scoped_cards.iter().any(|c| c.scope_id != first_scope) {
            return Err(TrackerError::ValidationError(
                "Scope filter returned cards from a different scope".to_string(),
            ));
        }
        let _ = app.update(Message::SelectScope(None));
    }

    // 4. Exercise card selection and verify lazy loading of `item.toml` + `desc.md`
    let first_card_id = all_cards[0].id.clone();
    let _ = app.update(Message::SelectCard(first_card_id.clone()));

    if app.lazy_detail_loads_count != 1 {
        return Err(TrackerError::ValidationError(format!(
            "Expected lazy_detail_loads_count == 1 after selecting card, got {}",
            app.lazy_detail_loads_count
        )));
    }

    let original_desc = {
        let detail = app.selected_item_detail.as_ref().ok_or_else(|| {
            TrackerError::ValidationError(
                "Expected selected_item_detail to be populated after selecting card".to_string(),
            )
        })?;
        if detail.manifest.item.id != first_card_id {
            return Err(TrackerError::ValidationError(format!(
                "Selected item detail ID '{}' did not match '{}'",
                detail.manifest.item.id, first_card_id
            )));
        }
        if detail.description_markdown.trim().is_empty() {
            return Err(TrackerError::ValidationError(
                "Lazily loaded desc.md content was unexpectedly empty".to_string(),
            ));
        }
        detail.description_markdown.clone()
    };

    if app.desc_editor_dirty {
        return Err(TrackerError::ValidationError(
            "desc_editor_dirty should be false immediately after lazy loading".to_string(),
        ));
    }
    if app.current_desc_editor_text().trim().is_empty() {
        return Err(TrackerError::ValidationError(
            "desc_editor_content was not initialized from desc.md".to_string(),
        ));
    }

    // 5. Exercise Markdown editing state in the Context Drawer
    let _ = app.update(Message::AppendDescTemplate(
        "\n- [Verification] Self-test template check",
    ));
    if !app.desc_editor_dirty {
        return Err(TrackerError::ValidationError(
            "Expected desc_editor_dirty == true after editing Markdown".to_string(),
        ));
    }
    if !app
        .current_desc_editor_text()
        .contains("Self-test template check")
    {
        return Err(TrackerError::ValidationError(
            "Markdown editor content did not include appended template text".to_string(),
        ));
    }

    // Render view with dirty Markdown editor and detail drawer open
    {
        let _drawer_view = app.view();
    }

    // Restore clean editor buffer so we don't mutate the live workspace on disk
    app.desc_editor_content = widget::text_editor::Content::with_text(&original_desc);
    app.desc_editor_dirty = false;

    // 6. Exercise Agent Context Handoff (`Copy @file Refs` & `Copy Bounded Agent Prompt`)
    let _ = app.update(Message::CopyAgentFileRefs);
    let copied_refs = app.last_copied_clipboard.clone().ok_or_else(|| {
        TrackerError::ValidationError("CopyAgentFileRefs did not populate clipboard".to_string())
    })?;
    if !copied_refs.contains("@.tracker/") || !copied_refs.contains("desc.md") {
        return Err(TrackerError::ValidationError(format!(
            "Invalid @file refs generated: {copied_refs}"
        )));
    }

    let _ = app.update(Message::CopyBoundedAgentPrompt);
    let copied_prompt = app.last_copied_clipboard.clone().ok_or_else(|| {
        TrackerError::ValidationError(
            "CopyBoundedAgentPrompt did not populate clipboard".to_string(),
        )
    })?;
    if !copied_prompt.contains("# Bounded Agent Task:")
        || !copied_prompt.contains(&first_card_id)
        || !copied_prompt.contains("## 1. Strict Domain Scope & Crate Boundary")
    {
        return Err(TrackerError::ValidationError(
            "Synthesized Bounded Agent Prompt is missing required sections".to_string(),
        ));
    }

    // 7. Exercise Metrics computation and all ViewModes (Metrics, PhasesAndScopes, Settings, Board)
    let metrics_report = app.compute_current_metrics();
    if metrics_report.phases.is_empty() || metrics_report.total_items == 0 {
        return Err(TrackerError::ValidationError(
            "Metrics report should include phases and tracked items".to_string(),
        ));
    }

    for mode in [
        ViewMode::Metrics,
        ViewMode::PhasesAndScopes,
        ViewMode::Settings,
        ViewMode::Board,
    ] {
        let _ = app.update(Message::SwitchView(mode));
        let _view = app.view();
    }

    println!(
        "nebulous --self-test OK: project='{}', active_phase='{}', phases={}, cards={}, lazy_loaded='{}' ({} bytes desc.md, {} bytes prompt)",
        app.project_name(),
        app.active_phase_id,
        app.index.phases.len(),
        all_cards.len(),
        first_card_id,
        original_desc.len(),
        copied_prompt.len()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_sample_tracker_workspace() -> (TempDir, TrackerStorage) {
        let temp = TempDir::new().expect("create temp dir");
        let storage = TrackerStorage::init(
            temp.path(),
            "NebulousTest",
            "01_core_engine",
            "Core Engine & Persistence",
        )
        .expect("init tracker");

        storage
            .create_scope(
                Some("01_core_engine"),
                "backend_core",
                "crates/backend_core",
                vec!["rust".to_string(), "serde".to_string()],
                Some("Core persistence crate".to_string()),
            )
            .expect("create backend_core scope");

        storage
            .create_scope(
                Some("01_core_engine"),
                "cosmic_interface",
                "crates/cosmic_interface",
                vec!["rust".to_string(), "libcosmic".to_string()],
                Some("Native libcosmic GUI crate".to_string()),
            )
            .expect("create cosmic_interface scope");

        let (_loc_bug, _) = storage
            .create_bug(
                "Empty array deserialization failure",
                Some("01_core_engine"),
                Some("backend_core"),
                Severity::High,
                Some("Parser fails when bugs = []"),
            )
            .expect("create BUG-0001");

        // Close and reopen BUG-0001 so it has recurrence_count = 1 and previous_fixes = 1
        storage
            .close_item(
                "BUG-0001",
                "Added serde default fallback",
                Some("a4f89c1"),
                false,
            )
            .expect("close BUG-0001");
        storage
            .reopen_item("BUG-0001", Some("Regression on nested tables"))
            .expect("reopen BUG-0001");
        storage
            .log_effort("BUG-0001", 45)
            .expect("log effort on BUG-0001");

        // Make BUG-0002 stagnant by backdating updated_at by 10 days
        let (loc_bug2, mut detail_bug2) = storage
            .create_bug(
                "Keyring fallback warning on headless session",
                Some("01_core_engine"),
                Some("backend_core"),
                Severity::Medium,
                Some("Verify fallback store works cleanly"),
            )
            .expect("create BUG-0002");
        detail_bug2.manifest.item.updated_at = "2020-01-01T00:00:00Z".to_string();
        storage
            .write_item_manifest(
                &loc_bug2.phase_id,
                &loc_bug2.scope_id,
                "BUG-0002",
                &detail_bug2.manifest,
            )
            .expect("backdate BUG-0002");

        let _ = storage
            .create_feature(
                "4-column Kanban card board",
                Some("01_core_engine"),
                Some("cosmic_interface"),
                Severity::Critical,
                Some("Render Open, In Progress, Pending, Closed / Merged columns"),
            )
            .expect("create FEAT-0001");
        storage
            .update_item_status("FEAT-0001", ItemStatus::InProgress)
            .expect("set FEAT-0001 in_progress");

        let _ = storage
            .create_feature(
                "Phase switcher dropdown",
                Some("01_core_engine"),
                Some("cosmic_interface"),
                Severity::Low,
                Some("Switch phases from header bar"),
            )
            .expect("create FEAT-0002");
        storage
            .close_item("FEAT-0002", "Implemented dropdown", Some("b7c91e2"), true)
            .expect("merge FEAT-0002");

        storage
            .create_phase("02_git_and_polish", "Git Sync & Polish", false)
            .expect("create phase 02");

        (temp, storage)
    }

    #[test]
    fn test_startup_reads_only_indexes_without_desc_md_and_lazy_loads_on_select() {
        let (_temp, storage) = create_sample_tracker_workspace();

        // Remove `desc.md` temporarily for BUG-0002 to prove startup does NOT read desc.md
        let bug2_desc = storage
            .item_desc_path("01_core_engine", "backend_core", "BUG-0002")
            .unwrap();
        let saved_desc = fs::read_to_string(&bug2_desc).unwrap();
        fs::remove_file(&bug2_desc).unwrap();

        // Startup must succeed because it only loads top-level indexes (`project.toml`, `phase.toml`, `scope.toml`)
        let mut app =
            NebulousApp::new_from_storage(Core::default(), storage.clone()).expect("boot app");
        assert!(app.selected_item_id.is_none());
        assert!(app.selected_item_detail.is_none());
        assert_eq!(app.lazy_detail_loads_count, 0);
        assert_eq!(app.index.phases.len(), 2);
        assert_eq!(app.active_phase_total_items(), 4);

        // Restore BUG-0002 desc.md before selecting it
        fs::write(&bug2_desc, &saved_desc).unwrap();

        // Selecting BUG-0001 lazily loads its `item.toml` + `desc.md`
        let _ = app.update(Message::SelectCard("BUG-0001".to_string()));
        assert_eq!(app.lazy_detail_loads_count, 1);
        assert_eq!(app.selected_item_id.as_deref(), Some("BUG-0001"));
        let detail = app
            .selected_item_detail
            .as_ref()
            .expect("detail should be loaded");
        assert_eq!(detail.manifest.item.id, "BUG-0001");
        assert_eq!(detail.manifest.history.recurrence_count, 1);
        assert_eq!(detail.manifest.effort.logged_time_minutes, 45);
        assert!(detail
            .description_markdown
            .contains("Parser fails when bugs = []"));
        assert!(app
            .current_desc_editor_text()
            .contains("Parser fails when bugs = []"));

        // Clearing selection hides drawer
        let _ = app.update(Message::ClearSelectedCard);
        assert!(app.selected_item_id.is_none());
        assert!(app.selected_item_detail.is_none());
    }

    #[test]
    fn test_kanban_columns_scope_filtering_and_filter_pills() {
        let (_temp, storage) = create_sample_tracker_workspace();
        let mut app = NebulousApp::new_from_storage(Core::default(), storage).expect("boot app");

        let counts = app.filter_counts();
        assert_eq!(counts.all, 4);
        assert_eq!(counts.bugs, 2);
        assert_eq!(counts.features, 2);
        assert_eq!(counts.high_or_critical, 2); // BUG-0001 (High), FEAT-0001 (Critical)
        assert_eq!(counts.stagnant, 1); // BUG-0002 backdated to 2020
        assert_eq!(counts.recurring, 1); // BUG-0001 reopened once

        // Verify KanbanColumn matching across all 4 columns
        let all_cards = app.filtered_kanban_cards();
        assert_eq!(
            all_cards
                .iter()
                .filter(|c| KanbanColumn::Open.matches(c.status))
                .count(),
            2
        );
        assert_eq!(
            all_cards
                .iter()
                .filter(|c| KanbanColumn::InProgress.matches(c.status))
                .count(),
            1
        );
        assert_eq!(
            all_cards
                .iter()
                .filter(|c| KanbanColumn::Pending.matches(c.status))
                .count(),
            0
        );
        assert_eq!(
            all_cards
                .iter()
                .filter(|c| KanbanColumn::ClosedOrMerged.matches(c.status))
                .count(),
            1
        );

        // Filter by Bugs pill
        let _ = app.update(Message::SetFilterPill(FilterPill::Bugs));
        let bug_cards = app.filtered_kanban_cards();
        assert_eq!(bug_cards.len(), 2);
        assert!(bug_cards.iter().all(|c| c.item_type == ItemType::Bug));

        // Filter by Stagnant pill
        let _ = app.update(Message::SetFilterPill(FilterPill::Stagnant));
        let stagnant_cards = app.filtered_kanban_cards();
        assert_eq!(stagnant_cards.len(), 1);
        assert_eq!(stagnant_cards[0].id, "BUG-0002");
        assert!(stagnant_cards[0].stagnant_days.unwrap() >= 4);

        // Filter by Recurring pill
        let _ = app.update(Message::SetFilterPill(FilterPill::Recurring));
        let recurring_cards = app.filtered_kanban_cards();
        assert_eq!(recurring_cards.len(), 1);
        assert_eq!(recurring_cards[0].id, "BUG-0001");
        assert_eq!(recurring_cards[0].recurrence_count, 1);

        // Reset filter pill to All and filter by domain scope `cosmic_interface`
        let _ = app.update(Message::SetFilterPill(FilterPill::All));
        let _ = app.update(Message::SelectScope(Some("cosmic_interface".to_string())));
        let ui_cards = app.filtered_kanban_cards();
        assert_eq!(ui_cards.len(), 2);
        assert!(ui_cards.iter().all(|c| c.scope_id == "cosmic_interface"));
    }

    #[test]
    fn test_quick_add_timer_status_transition_and_phase_switching() {
        let (temp, storage) = create_sample_tracker_workspace();
        let mut app = NebulousApp::new_from_storage(Core::default(), storage).expect("boot app");

        // Quick-add a bug into `inbox` (when selected_scope is None)
        let _ = app.update(Message::OpenQuickAdd(ItemType::Bug));
        assert!(app.quick_add.open);
        let _ = app.update(Message::SetQuickAddSeverity(Severity::Critical));
        let _ = app.update(Message::QuickAddTitleChanged(
            "Wayland popup focus loss".to_string(),
        ));
        let _ = app.update(Message::SubmitQuickAdd);
        assert!(!app.quick_add.open);
        assert_eq!(app.selected_item_id.as_deref(), Some("BUG-0003"));
        assert_eq!(app.active_phase_total_items(), 5);

        // Start and stop active timer via GUI messages
        let _ = app.update(Message::StartTimer("BUG-0003".to_string()));
        assert!(app.is_timer_running_for("BUG-0003"));
        let _ = app.update(Message::StopTimer);
        assert!(app.active_timer.is_none());

        // Quick status transition from Open -> InProgress
        let _ = app.update(Message::QuickStatusTransition {
            item_id: "BUG-0003".to_string(),
            new_status: ItemStatus::InProgress,
        });
        assert_eq!(
            app.selected_item_detail
                .as_ref()
                .unwrap()
                .manifest
                .item
                .status,
            ItemStatus::InProgress
        );

        // Switch phase to `02_git_and_polish`
        let _ = app.update(Message::SelectPhaseIndex(1));
        assert_eq!(app.active_phase_id, "02_git_and_polish");
        assert_eq!(app.active_phase_total_items(), 0);

        // Switch back to `01_core_engine` and run headless self-test against temp workspace
        let _ = app.update(Message::SelectPhaseIndex(0));
        run_headless_self_test(Some(temp.path())).expect("headless self-test on temp workspace");
    }

    #[test]
    fn test_drawer_markdown_save_fix_ledger_close_reopen_effort_and_agent_prompts() {
        let (_temp, storage) = create_sample_tracker_workspace();
        let mut app =
            NebulousApp::new_from_storage(Core::default(), storage.clone()).expect("boot app");

        // Select BUG-0001
        let _ = app.update(Message::SelectCard("BUG-0001".to_string()));
        assert!(!app.desc_editor_dirty);

        // Edit title and save
        let _ = app.update(Message::DrawerTitleChanged(
            "Empty TOML array parser regression".to_string(),
        ));
        let _ = app.update(Message::SaveDrawerTitle);
        assert_eq!(
            app.selected_item_detail
                .as_ref()
                .unwrap()
                .manifest
                .item
                .title,
            "Empty TOML array parser regression"
        );

        // Edit Markdown via SetDescMarkdown + AppendDescTemplate and save atomically
        let _ = app.update(Message::SetDescMarkdown(
            "## Problem Description\nUpdated via cosmic text_editor.\n".to_string(),
        ));
        assert!(app.desc_editor_dirty);
        let _ = app.update(Message::AppendDescTemplate(
            "\n- [Fix Note] Added #[serde(default)] on ScopeItems",
        ));
        let _ = app.update(Message::SaveDescMarkdown);
        assert!(!app.desc_editor_dirty);

        let disk_desc = storage
            .read_item_desc("01_core_engine", "backend_core", "BUG-0001")
            .expect("read saved desc.md");
        assert!(disk_desc.contains("Updated via cosmic text_editor."));
        assert!(disk_desc.contains("Added #[serde(default)] on ScopeItems"));

        // Log manual effort (+15m quick button and +25m custom input)
        let _ = app.update(Message::LogManualMinutes(15));
        let _ = app.update(Message::ManualMinutesInputChanged("25".to_string()));
        let _ = app.update(Message::SubmitCustomManualMinutes);
        assert_eq!(
            app.selected_item_detail
                .as_ref()
                .unwrap()
                .manifest
                .effort
                .logged_time_minutes,
            45 + 15 + 25
        );

        // Close with Fix Summary and explicit commit override
        let _ = app.update(Message::CloseSummaryInputChanged(
            "Fixed nested table empty array handling".to_string(),
        ));
        let _ = app.update(Message::CloseCommitInputChanged("c0ffee7".to_string()));
        let _ = app.update(Message::CloseSelectedWithSummary { merged: false });

        let detail_after_close = app.selected_item_detail.as_ref().unwrap();
        assert_eq!(detail_after_close.manifest.item.status, ItemStatus::Closed);
        assert_eq!(detail_after_close.manifest.history.previous_fixes.len(), 2);
        assert_eq!(
            detail_after_close.manifest.history.previous_fixes[1].commit,
            "c0ffee7"
        );
        assert_eq!(
            detail_after_close.manifest.history.previous_fixes[1].summary,
            "Fixed nested table empty array handling"
        );

        // Reopen with Reason note -> increments recurrence_count (1 -> 2) and iteration_count (2 -> 3)
        let _ = app.update(Message::ReopenReasonInputChanged(
            "Found edge case with inline tables".to_string(),
        ));
        let _ = app.update(Message::ReopenSelectedItem);

        let detail_after_reopen = app.selected_item_detail.as_ref().unwrap();
        assert_eq!(detail_after_reopen.manifest.item.status, ItemStatus::Open);
        assert_eq!(detail_after_reopen.manifest.history.recurrence_count, 2);
        assert_eq!(detail_after_reopen.manifest.effort.iteration_count, 3);
        assert!(detail_after_reopen
            .description_markdown
            .contains("Found edge case with inline tables"));

        // Export @file Refs and Bounded Agent Prompt
        let _ = app.update(Message::CopyAgentFileRefs);
        let refs = app.last_copied_clipboard.clone().unwrap();
        assert!(refs.contains("@.tracker/phases/01_core_engine/domains/backend_core/scope.toml"));
        assert!(refs.contains(
            "@.tracker/phases/01_core_engine/domains/backend_core/items/BUG-0001/item.toml"
        ));
        assert!(refs.contains(
            "@.tracker/phases/01_core_engine/domains/backend_core/items/BUG-0001/desc.md"
        ));

        let _ = app.update(Message::CopyBoundedAgentPrompt);
        let prompt = app.last_copied_clipboard.clone().unwrap();
        assert!(prompt.contains("crates/backend_core"));
        assert!(prompt.contains("recurrence_count = 2"));
        assert!(prompt.contains("c0ffee7"));
        assert!(prompt.contains("Fixed nested table empty array handling"));
    }

    #[test]
    fn test_settings_phase_scope_creation_metrics_thresholds_git_and_pat_keyring() {
        let (_temp, storage) = create_sample_tracker_workspace();
        let mut app =
            NebulousApp::new_from_storage(Core::default(), storage.clone()).expect("boot app");

        // 1. Create a new Phase and set it active
        let _ = app.update(Message::NewPhaseIdChanged("03_cloud_sync".to_string()));
        let _ = app.update(Message::NewPhaseTitleChanged(
            "Cloud Sync & Keyring".to_string(),
        ));
        let _ = app.update(Message::ToggleNewPhaseSetActive(true));
        let _ = app.update(Message::CreatePhase);
        assert_eq!(app.active_phase_id, "03_cloud_sync");
        assert_eq!(app.index.phases.len(), 3);

        // 2. Create a new Domain Scope in `03_cloud_sync`
        let _ = app.update(Message::NewScopeIdChanged("sync_daemon".to_string()));
        let _ = app.update(Message::NewScopeTargetChanged(
            "crates/sync_daemon".to_string(),
        ));
        let _ = app.update(Message::NewScopeTechStackChanged(
            "rust, git2, keyring".to_string(),
        ));
        let _ = app.update(Message::NewScopeDescriptionChanged(
            "Background sync worker".to_string(),
        ));
        let _ = app.update(Message::CreateScope);
        assert!(app.active_phase().unwrap().scope("sync_daemon").is_some());

        // 3. Update Metrics thresholds & default_scope
        let _ = app.update(Message::TargetDailyHoursChanged("7.5".to_string()));
        let _ = app.update(Message::StagnantThresholdDaysChanged("3".to_string()));
        let _ = app.update(Message::DefaultScopeChanged("triage_inbox".to_string()));
        let _ = app.update(Message::SaveProjectAndMetricsConfig);
        assert_eq!(app.target_daily_hours(), 7.5);
        assert_eq!(app.stagnant_threshold_days(), 3);
        assert_eq!(app.default_scope(), "triage_inbox");

        // 4. Update Git Sync settings to Scheduled mode
        let _ = app.update(Message::SetGitSyncMode(GitSyncMode::Scheduled));
        let _ = app.update(Message::GitSyncIntervalChanged("15".to_string()));
        let _ = app.update(Message::GitRemoteChanged("upstream".to_string()));
        let _ = app.update(Message::GitBranchChanged("develop".to_string()));
        let _ = app.update(Message::SaveGitConfig);
        assert_eq!(app.index.project.git.sync_mode, GitSyncMode::Scheduled);
        assert_eq!(app.index.project.git.sync_interval_minutes, 15);
        assert_eq!(app.index.project.git.remote, "upstream");
        assert_eq!(app.index.project.git.branch, "develop");

        // 5. Save and clear PAT in OS keyring (uses fallback store on headless environments)
        let _ = app.update(Message::PatInputChanged(
            "ghp_test_secret_token_123".to_string(),
        ));
        let _ = app.update(Message::SavePatToKeyring);
        assert!(app.settings_form.has_stored_pat);
        assert_eq!(
            storage.get_pat().expect("retrieve stored PAT"),
            "ghp_test_secret_token_123"
        );

        let _ = app.update(Message::DeletePatFromKeyring);
        assert!(!app.settings_form.has_stored_pat);

        // 6. Verify subscription construction when scheduled sync is active
        let _sub = app.subscription();
    }

    #[test]
    fn test_edge_cases_editor_actions_watcher_preservation_and_validation_errors() {
        let (_temp, storage) = create_sample_tracker_workspace();
        let mut app =
            NebulousApp::new_from_storage(Core::default(), storage.clone()).expect("boot app");

        let _ = app.update(Message::SelectCard("BUG-0001".to_string()));
        assert!(!app.desc_editor_dirty);

        // 1. Non-edit text_editor action (`Action::Move`) must NOT mark `desc_editor_dirty = true`
        let _ = app.update(Message::DescEditorAction(
            widget::text_editor::Action::Move(widget::text_editor::Motion::DocumentStart),
        ));
        assert!(!app.desc_editor_dirty);

        // 2. Edit text_editor action (`Action::Edit(Edit::Insert)`) MUST mark `desc_editor_dirty = true`
        let _ = app.update(Message::DescEditorAction(
            widget::text_editor::Action::Edit(widget::text_editor::Edit::Insert('#')),
        ));
        assert!(app.desc_editor_dirty);

        // 3. External file watcher notification (`TrackerFilesChanged`) must NOT clobber unsaved editor buffer
        let unsaved_text = app.current_desc_editor_text();
        storage
            .write_item_desc(
                "01_core_engine",
                "backend_core",
                "BUG-0001",
                "External disk change while editor is dirty\n",
            )
            .unwrap();
        let _ = app.update(Message::TrackerFilesChanged(vec![storage
            .item_desc_path("01_core_engine", "backend_core", "BUG-0001")
            .unwrap()]));
        assert!(app.desc_editor_dirty);
        assert_eq!(app.current_desc_editor_text(), unsaved_text);

        // Once saved or clean, external disk changes ARE reflected on reload
        app.desc_editor_dirty = false;
        let _ = app.update(Message::TrackerFilesChanged(Vec::new()));
        assert!(app
            .current_desc_editor_text()
            .contains("External disk change while editor is dirty"));

        // 4. Closing an item with an active timer running on it automatically stops the timer
        let _ = app.update(Message::StartTimer("BUG-0001".to_string()));
        assert!(app.is_timer_running_for("BUG-0001"));
        let _ = app.update(Message::CloseSummaryInputChanged(
            "Closed while timer active".to_string(),
        ));
        let _ = app.update(Message::CloseSelectedWithSummary { merged: false });
        assert!(app.active_timer.is_none());
        assert_eq!(
            app.selected_item_detail
                .as_ref()
                .unwrap()
                .manifest
                .item
                .status,
            ItemStatus::Closed
        );

        // 5. Validation error paths: empty close summary, invalid manual minutes, invalid config values
        let _ = app.update(Message::ReopenSelectedItem);
        assert_eq!(
            app.selected_item_detail
                .as_ref()
                .unwrap()
                .manifest
                .item
                .status,
            ItemStatus::Open
        );

        // Reopening an already-open item fails gracefully with banner
        let prev_recurrences = app
            .selected_item_detail
            .as_ref()
            .unwrap()
            .manifest
            .history
            .recurrence_count;
        let _ = app.update(Message::ReopenSelectedItem);
        assert_eq!(
            app.selected_item_detail
                .as_ref()
                .unwrap()
                .manifest
                .history
                .recurrence_count,
            prev_recurrences
        );
        assert!(app
            .status_banner
            .as_deref()
            .unwrap_or_default()
            .contains("Cannot reopen"));

        // Empty close summary must not close item
        let _ = app.update(Message::CloseSummaryInputChanged("   ".to_string()));
        let _ = app.update(Message::CloseSelectedWithSummary { merged: false });
        assert_eq!(
            app.selected_item_detail
                .as_ref()
                .unwrap()
                .manifest
                .item
                .status,
            ItemStatus::Open
        );

        // Invalid manual minutes ("0" or "abc") must not alter logged_time_minutes
        let logged_before = app
            .selected_item_detail
            .as_ref()
            .unwrap()
            .manifest
            .effort
            .logged_time_minutes;
        let _ = app.update(Message::ManualMinutesInputChanged("0".to_string()));
        let _ = app.update(Message::SubmitCustomManualMinutes);
        let _ = app.update(Message::ManualMinutesInputChanged("abc".to_string()));
        let _ = app.update(Message::SubmitCustomManualMinutes);
        assert_eq!(
            app.selected_item_detail
                .as_ref()
                .unwrap()
                .manifest
                .effort
                .logged_time_minutes,
            logged_before
        );

        // Invalid target_daily_hours ("-3.0", "NaN", "inf") and git_sync_interval ("0") must not overwrite project.toml
        let prev_target = app.target_daily_hours();
        for bad_hours in ["-3.0", "NaN", "inf"] {
            let _ = app.update(Message::TargetDailyHoursChanged(bad_hours.to_string()));
            let _ = app.update(Message::SaveProjectAndMetricsConfig);
            assert_eq!(app.target_daily_hours(), prev_target);
        }

        let prev_interval = app.index.project.git.sync_interval_minutes;
        let _ = app.update(Message::GitSyncIntervalChanged("0".to_string()));
        let _ = app.update(Message::SaveGitConfig);
        assert_eq!(app.index.project.git.sync_interval_minutes, prev_interval);

        // 6. Dirty desc.md buffer is auto-flushed on CopyBoundedAgentPrompt and ReopenSelectedItem
        let _ = app.update(Message::SetDescMarkdown(
            "## Problem Description\nUnsaved editor specification before prompt export.\n"
                .to_string(),
        ));
        assert!(app.desc_editor_dirty);
        let _ = app.update(Message::CopyBoundedAgentPrompt);
        assert!(!app.desc_editor_dirty);
        assert!(app
            .last_copied_clipboard
            .as_deref()
            .unwrap_or_default()
            .contains("Unsaved editor specification before prompt export."));

        // Close BUG-0001, make desc.md dirty again, and ReopenSelectedItem with reason -> preserves dirty edit + appends reason
        let _ = app.update(Message::CloseSummaryInputChanged(
            "Temporary close".to_string(),
        ));
        let _ = app.update(Message::CloseSelectedWithSummary { merged: false });
        let _ = app.update(Message::AppendDescTemplate(
            "\n- [Fix Note] Unsaved note before reopen",
        ));
        assert!(app.desc_editor_dirty);
        let _ = app.update(Message::ReopenReasonInputChanged(
            "Reopened after unsaved note".to_string(),
        ));
        let _ = app.update(Message::ReopenSelectedItem);
        assert!(!app.desc_editor_dirty);
        let reopened_desc = app.current_desc_editor_text();
        assert!(reopened_desc.contains("Unsaved note before reopen"));
        assert!(reopened_desc.contains("Reopened after unsaved note"));

        // 7. Unsaved drawer_title_input is preserved when LogManualMinutes triggers reload_indexes()
        let _ = app.update(Message::DrawerTitleChanged(
            "Draft Unsaved Title".to_string(),
        ));
        let _ = app.update(Message::LogManualMinutes(5));
        assert_eq!(app.drawer_title_input, "Draft Unsaved Title");

        // 8. Clicking already-selected status or severity on stagnant BUG-0002 is a no-op and preserves stagnation
        let _ = app.update(Message::SelectCard("BUG-0002".to_string()));
        let orig_updated_at = app
            .selected_item_detail
            .as_ref()
            .unwrap()
            .manifest
            .item
            .updated_at
            .clone();
        let _ = app.update(Message::QuickStatusTransition {
            item_id: "BUG-0002".to_string(),
            new_status: ItemStatus::Open,
        });
        let _ = app.update(Message::UpdateSelectedSeverity(Severity::Medium));
        assert_eq!(
            app.selected_item_detail
                .as_ref()
                .unwrap()
                .manifest
                .item
                .updated_at,
            orig_updated_at
        );
        assert!(app
            .card_signals
            .get("BUG-0002")
            .unwrap()
            .stagnant_days
            .is_some());

        // 9. SelectPhaseScopeAndOpenBoard switches both active_phase_id and selected_scope and opens Board view
        let _ = app.update(Message::SwitchView(ViewMode::Metrics));
        let _ = app.update(Message::SelectPhaseScopeAndOpenBoard {
            phase_id: "02_git_and_polish".to_string(),
            scope_id: "inbox".to_string(),
        });
        assert_eq!(app.view_mode, ViewMode::Board);
        assert_eq!(app.active_phase_id, "02_git_and_polish");
        assert_eq!(app.selected_scope.as_deref(), Some("inbox"));

        // 10. External project.toml updates synchronize active_phase_id and settings_form on TrackerFilesChanged
        let mut proj = storage.read_project().unwrap();
        proj.project.active_phase = "01_core_engine".to_string();
        proj.metrics.target_daily_hours = 8.5;
        storage.write_project(&proj).unwrap();
        let _ = app.update(Message::TrackerFilesChanged(Vec::new()));
        assert_eq!(app.active_phase_id, "01_core_engine");
        assert_eq!(app.settings_form.target_daily_hours_input, "8.5");
    }
}
