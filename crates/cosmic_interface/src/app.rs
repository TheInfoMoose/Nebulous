use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use backend_core::{
    global_registry_path, load_global_registry_from, register_project_in, remove_project_in,
    set_active_project_in, validate_safe_project_root, ComponentDependency, ComponentManifest,
    CreateItemInput, DocumentedCommitInput, GitRepoStatus, GitSyncMode, GlobalProjectRegistry,
    IngestionOptions, ItemDetail, ItemLocation, ItemManifest, ItemStatus, ItemType, PhaseIndex,
    ProjectMetricsReport, Result, Severity, SshPublicKeyInfo, TimerStatus, TrackerError,
    TrackerIndex, TrackerStorage,
};
use chrono::Utc;
use cosmic::app::{Core, Task};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::{executor, widget, Application, Element};

use crate::subscriptions;
use crate::views::{
    detail_drawer, header_bar, kanban_board, metrics_view, nav_sidebar, phases_scopes_view,
    settings_view,
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

/// Form state for the Phases & Scopes Manager and Project/Git/Keyring Settings views.
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

    // Multi-Project Registry form fields
    pub new_project_path_input: String,
    pub new_project_name_input: String,
    pub new_project_ingest_on_add: bool,

    // Directory Ingestion & Component Pinning form fields
    pub ingest_subpath_input: String,
    pub ingest_write_manifests: bool,
    pub ingest_promote_all: bool,
    pub selected_component_id: String,
    pub component_tags_input: String,
    pub component_flags_input: String,
    pub component_build_script_input: String,
    pub dep_name_input: String,
    pub dep_kind_input: String,
    pub dep_constraint_input: String,
    pub dep_version_input: String,
    pub dep_source_input: String,

    // Documented Git Commit Builder & Commit Policy form fields
    pub commit_item_id_input: String,
    pub commit_scope_id_input: String,
    pub commit_summary_input: String,
    pub commit_body_input: String,
    pub commit_tracker_only: bool,
    pub commit_policy_require_item_id: bool,
    pub commit_policy_require_scope: bool,
    pub commit_policy_require_summary: bool,
    pub commit_policy_min_summary_len_input: String,
    pub commit_policy_auto_append_to_desc: bool,

    // Cross-Platform SecureStorage SSH Ed25519 Credentials form fields
    pub ssh_comment_input: String,
    pub ssh_private_key_import_input: String,
    pub ssh_passphrase_input: String,
    pub has_stored_ssh_key: bool,
    pub stored_ssh_public_key: Option<SshPublicKeyInfo>,
}

impl SettingsFormState {
    pub fn from_index_and_storage(index: &TrackerIndex, storage: &TrackerStorage) -> Self {
        let project = &index.project;
        let has_stored_pat = storage.get_pat().is_ok();
        let stored_ssh_public_key = storage.get_ssh_public_key_info().ok();
        let has_stored_ssh_key = stored_ssh_public_key.is_some();
        let first_comp = project.components.registry.first();

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
            new_project_path_input: String::new(),
            new_project_name_input: String::new(),
            new_project_ingest_on_add: true,
            ingest_subpath_input: String::new(),
            ingest_write_manifests: false,
            ingest_promote_all: false,
            selected_component_id: first_comp.map(|c| c.id.clone()).unwrap_or_default(),
            component_tags_input: first_comp.map(|c| c.tags.join(", ")).unwrap_or_default(),
            component_flags_input: first_comp.map(|c| c.flags.join(", ")).unwrap_or_default(),
            component_build_script_input: first_comp
                .and_then(|c| c.pinned_release.as_ref())
                .and_then(|p| p.build_script_path.clone())
                .unwrap_or_default(),
            dep_name_input: String::new(),
            dep_kind_input: "apt_sid".to_string(),
            dep_constraint_input: String::new(),
            dep_version_input: String::new(),
            dep_source_input: String::new(),
            commit_item_id_input: String::new(),
            commit_scope_id_input: String::new(),
            commit_summary_input: String::new(),
            commit_body_input: String::new(),
            commit_tracker_only: false,
            commit_policy_require_item_id: project.git.commit_policy.require_item_id,
            commit_policy_require_scope: project.git.commit_policy.require_scope,
            commit_policy_require_summary: project.git.commit_policy.require_summary,
            commit_policy_min_summary_len_input: project
                .git
                .commit_policy
                .min_summary_length
                .to_string(),
            commit_policy_auto_append_to_desc: project.git.commit_policy.auto_append_to_desc,
            ssh_comment_input: String::new(),
            ssh_private_key_import_input: String::new(),
            ssh_passphrase_input: String::new(),
            has_stored_ssh_key,
            stored_ssh_public_key,
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
        self.commit_policy_require_item_id = project.git.commit_policy.require_item_id;
        self.commit_policy_require_scope = project.git.commit_policy.require_scope;
        self.commit_policy_require_summary = project.git.commit_policy.require_summary;
        self.commit_policy_min_summary_len_input =
            project.git.commit_policy.min_summary_length.to_string();
        self.commit_policy_auto_append_to_desc = project.git.commit_policy.auto_append_to_desc;
        self.stored_ssh_public_key = storage.get_ssh_public_key_info().ok();
        self.has_stored_ssh_key = self.stored_ssh_public_key.is_some();

        let matched_comp = if self.selected_component_id.trim().is_empty() {
            project.components.registry.first()
        } else {
            project
                .components
                .find(&self.selected_component_id)
                .or_else(|| project.components.registry.first())
        };
        if let Some(comp) = matched_comp {
            self.selected_component_id = comp.id.clone();
            self.component_tags_input = comp.tags.join(", ");
            self.component_flags_input = comp.flags.join(", ");
            self.component_build_script_input = comp
                .pinned_release
                .as_ref()
                .and_then(|p| p.build_script_path.clone())
                .unwrap_or_default();
        }
    }
}

/// Application messages dispatched by the native `libcosmic` GUI.
#[derive(Debug, Clone)]
pub enum Message {
    // Top Header Bar & Multi-Project Switcher messages
    SelectProjectIndex(usize),
    SwitchProject(String),
    OpenAddProject,
    NewProjectPathChanged(String),
    NewProjectNameChanged(String),
    ToggleNewProjectIngest(bool),
    AddProjectToRegistry,
    RemoveProjectFromRegistry(String),

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

    // Directory Ingestion & Component Pinning messages (`Phases & Scopes` view)
    IngestSubpathChanged(String),
    ToggleIngestWriteManifests(bool),
    ToggleIngestPromoteAll(bool),
    IngestProjectDirectory,
    SelectComponentForPin(String),
    WriteComponentManifest(String),
    PromoteComponentToScope(String),
    ComponentTagsInputChanged(String),
    ComponentFlagsInputChanged(String),
    ComponentBuildScriptInputChanged(String),
    SaveComponentMetadata,
    ComponentDepNameChanged(String),
    ComponentDepKindChanged(String),
    ComponentDepConstraintChanged(String),
    ComponentDepVersionChanged(String),
    ComponentDepSourceChanged(String),
    UpsertComponentDependency,
    PinComponentRelease(String),
    PinSelectedComponentRelease,
    RollbackComponentRelease(String),
    RollbackSelectedComponentRelease,

    // Phase & Scope Manager messages
    NewPhaseIdChanged(String),
    NewPhaseTitleChanged(String),
    ToggleNewPhaseSetActive(bool),
    CreatePhase,
    NewScopeIdChanged(String),
    NewScopeTargetChanged(String),
    NewScopeTechStackChanged(String),
    NewScopeDescriptionChanged(String),
    CreateScope,

    // Git Operations, Documented Commit Builder, Commit Policy & SecureStorage messages (`Git & Settings` view)
    GitFetch,
    GitPull,
    GitPush,
    CommitItemIdChanged(String),
    CommitScopeIdChanged(String),
    CommitSummaryChanged(String),
    CommitBodyChanged(String),
    SetCommitTrackerOnly(bool),
    SubmitDocumentedCommit,
    ToggleCommitPolicyRequireItemId(bool),
    ToggleCommitPolicyRequireScope(bool),
    ToggleCommitPolicyAutoAppendDesc(bool),
    CommitPolicyMinSummaryLengthChanged(String),
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
    SshKeyCommentChanged(String),
    SshKeyImportInputChanged(String),
    SshKeyPassphraseChanged(String),
    GenerateSshKeypair,
    ImportSshKeypair,
    CopySshPublicKey,
    DeleteSshKeypair,

    // Reactive background subscription messages
    TrackerFilesChanged(Vec<PathBuf>),
    ScheduledGitSyncTick,
}

/// Native `libcosmic` application state for Nebulous (`nebulous` binary).
pub struct NebulousApp {
    pub core: Core,
    pub storage: TrackerStorage,
    /// Path to `projects.toml` used by this application instance.
    pub registry_path: PathBuf,
    /// Loaded multi-project registry (`projects.toml`).
    pub project_registry: GlobalProjectRegistry,
    /// Ordered project IDs for the top-bar Project Switcher dropdown.
    pub project_ids: Vec<String>,
    /// Formatted project labels (`◆ Nebulous`) for the top-bar Project Switcher dropdown.
    pub project_labels: Vec<String>,
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
    /// Resolves `TrackerStorage` from an optional workspace path, current working directory,
    /// or the global `projects.toml` registry without creating `.tracker/` in an untracked CWD.
    pub fn resolve_or_init_storage(workspace_path: Option<&Path>) -> Result<TrackerStorage> {
        let registry_path = global_registry_path();
        Self::resolve_or_init_storage_with_registry(workspace_path, &registry_path)
    }

    /// Resolves `TrackerStorage` using a 4-tier priority order:
    /// 1. Explicit `workspace_path` (`--workspace <PATH>`), validated via [`validate_safe_project_root`].
    /// 2. Existing `.tracker/` discovered from `std::env::current_dir()`, if its root is safe.
    /// 3. Active (or first valid) project registered in `registry_path` (`projects.toml`).
    /// 4. Returns `Err(TrackerError::TrackerRootNotFound)` without creating `.tracker/` in CWD.
    pub fn resolve_or_init_storage_with_registry(
        workspace_path: Option<&Path>,
        registry_path: &Path,
    ) -> Result<TrackerStorage> {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::resolve_or_init_storage_from_cwd_with_registry(workspace_path, &cwd, registry_path)
    }

    /// Resolves `TrackerStorage` using `cwd` when `workspace_path` is `None`.
    pub fn resolve_or_init_storage_from_cwd_with_registry(
        workspace_path: Option<&Path>,
        cwd: &Path,
        registry_path: &Path,
    ) -> Result<TrackerStorage> {
        if let Some(p) = workspace_path {
            validate_safe_project_root(p)?;
            let fallback_name = p
                .file_name()
                .and_then(|n| n.to_str())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("Nebulous");
            return Self::resolve_or_init_storage_named(p, Some(fallback_name));
        }

        if let Ok(discovered) = TrackerStorage::discover(cwd) {
            if validate_safe_project_root(&discovered.project_root()).is_ok() {
                return Ok(discovered);
            }
        }

        if let Ok(registry) = load_global_registry_from(registry_path) {
            if let Some(active) = registry.active_project() {
                let root = Path::new(&active.root_path);
                if root.is_dir() && validate_safe_project_root(root).is_ok() {
                    if let Ok(storage) = Self::open_or_init_project_root(root, Some(&active.name)) {
                        return Ok(storage);
                    }
                }
            }
            for proj in &registry.projects {
                let root = Path::new(&proj.root_path);
                if root.is_dir() && validate_safe_project_root(root).is_ok() {
                    if let Ok(storage) = Self::open_or_init_project_root(root, Some(&proj.name)) {
                        return Ok(storage);
                    }
                }
            }
        }

        Err(TrackerError::TrackerRootNotFound(cwd.to_path_buf()))
    }

    /// Resolves `TrackerStorage` at `base_dir` or initializes `.tracker/` with `project_name_hint` if missing.
    pub fn resolve_or_init_storage_named(
        base_dir: &Path,
        project_name_hint: Option<&str>,
    ) -> Result<TrackerStorage> {
        match TrackerStorage::discover(base_dir) {
            Ok(storage) => {
                validate_safe_project_root(&storage.project_root())?;
                Ok(storage)
            }
            Err(TrackerError::TrackerRootNotFound(_)) => {
                Self::open_or_init_project_root(base_dir, project_name_hint)
            }
            Err(err) => Err(err),
        }
    }

    /// Opens `.tracker/` directly inside `base_dir` (or if `base_dir` is `.tracker/`), initializing it if missing
    /// without walking up into parent directories.
    pub fn open_or_init_project_root(
        base_dir: &Path,
        project_name_hint: Option<&str>,
    ) -> Result<TrackerStorage> {
        let storage = TrackerStorage::new(base_dir);
        validate_safe_project_root(&storage.project_root())?;
        if storage.project_toml_path().is_file() {
            return Ok(storage);
        }
        let name = project_name_hint
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .or_else(|| {
                base_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .filter(|s| !s.trim().is_empty())
            })
            .unwrap_or("Nebulous");
        TrackerStorage::init(base_dir, name, "01_core_engine", "Core Engine")
    }

    /// Bootstraps `NebulousApp` reading ONLY top-level `.tracker/` indexes (`project.toml`,
    /// `phase.toml`, `scope.toml`) without eagerly loading any `item.toml` + `desc.md` detail pairs.
    pub fn new_from_storage(core: Core, storage: TrackerStorage) -> Result<Self> {
        let registry_path = resolve_app_registry_path(&storage);
        Self::new_from_storage_with_registry(core, storage, registry_path)
    }

    /// Bootstraps `NebulousApp` with an explicit `projects.toml` registry file path.
    pub fn new_from_storage_with_registry(
        core: Core,
        storage: TrackerStorage,
        registry_path: PathBuf,
    ) -> Result<Self> {
        // Startup strictly reads top-level indexes via `load_index()`.
        let index = storage.load_index()?;
        let active_phase_id = index.project.project.active_phase.clone();
        let (phase_ids, phase_labels) = build_phase_dropdown_lists(&index);
        let active_timer = storage.timer_status().ok().flatten();
        let git_status = storage.git_status().ok();
        let settings_form = SettingsFormState::from_index_and_storage(&index, &storage);

        if storage.project_toml_path().is_file()
            && validate_safe_project_root(&storage.project_root()).is_ok()
        {
            let _ = register_project_in(
                &registry_path,
                storage.project_root(),
                Some(&index.project.project.name),
                Vec::new(),
                true,
            );
        }
        let project_registry = load_global_registry_from(&registry_path).unwrap_or_default();
        let (project_ids, project_labels) =
            build_project_dropdown_lists(&project_registry, &index.project.project.name);

        Ok(Self {
            core,
            storage,
            registry_path,
            project_registry,
            project_ids,
            project_labels,
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

    /// Returns the selected index in `project_labels` for the top-bar Project Switcher dropdown.
    pub fn selected_project_dropdown_index(&self) -> Option<usize> {
        if let Some(ref active_id) = self.project_registry.active_project_id {
            if let Some(idx) = self.project_ids.iter().position(|id| id == active_id) {
                return Some(idx);
            }
        }
        let current_root = self
            .storage
            .project_root()
            .to_string_lossy()
            .replace('\\', "/");
        if let Some(proj) = self
            .project_registry
            .projects
            .iter()
            .find(|p| p.root_path == current_root)
        {
            if let Some(idx) = self.project_ids.iter().position(|id| id == &proj.id) {
                return Some(idx);
            }
        }
        (!self.project_labels.is_empty()).then_some(0)
    }

    /// Returns the currently selected [`ComponentManifest`] for the Pin Builder in `Phases & Scopes`.
    pub fn selected_component_manifest(&self) -> Option<&ComponentManifest> {
        let sel = self.settings_form.selected_component_id.trim();
        if !sel.is_empty() {
            if let Some(comp) = self.index.project.components.find(sel) {
                return Some(comp);
            }
        }
        self.index.project.components.registry.first()
    }

    fn effective_selected_component_id(&self) -> Option<String> {
        let sel = self.settings_form.selected_component_id.trim();
        if !sel.is_empty() {
            return Some(sel.to_string());
        }
        self.index
            .project
            .components
            .registry
            .first()
            .map(|c| c.id.clone())
    }

    fn refresh_project_registry(&mut self) {
        let mut current = load_global_registry_from(&self.registry_path).unwrap_or_default();
        let root_str = self
            .storage
            .project_root()
            .to_string_lossy()
            .replace('\\', "/");
        if self.storage.project_toml_path().is_file()
            && validate_safe_project_root(&self.storage.project_root()).is_ok()
            && current.find_project(&root_str).is_some()
        {
            let _ = register_project_in(
                &self.registry_path,
                self.storage.project_root(),
                Some(&self.index.project.project.name),
                Vec::new(),
                true,
            );
            current = load_global_registry_from(&self.registry_path).unwrap_or_default();
        }
        self.project_registry = current;
        let (ids, labels) =
            build_project_dropdown_lists(&self.project_registry, &self.index.project.project.name);
        self.project_ids = ids;
        self.project_labels = labels;
    }

    /// Returns the canonical section titles rendered by the currently active `view_mode`.
    pub fn active_view_section_titles(&self) -> &'static [&'static str] {
        match self.view_mode {
            ViewMode::Board => &["OPEN", "IN PROGRESS", "PENDING", "CLOSED / MERGED"],
            ViewMode::Metrics => &[
                "TOTAL LOGGED EFFORT",
                "PROJECT COMPLETION",
                "STAGNANT ITEMS",
                "RECURRING HOTSPOTS",
            ],
            ViewMode::PhasesAndScopes => &phases_scopes_view::PHASES_SCOPES_SECTION_TITLES,
            ViewMode::Settings => &settings_view::SETTINGS_SECTION_TITLES,
        }
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
        self.settings_form.stored_ssh_public_key = self.storage.get_ssh_public_key_info().ok();
        self.settings_form.has_stored_ssh_key = self.settings_form.stored_ssh_public_key.is_some();
        self.refresh_project_registry();

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

    fn switch_to_storage(&mut self, new_storage: TrackerStorage) -> Result<()> {
        let index = new_storage.load_index()?;
        let active_phase_id = index.project.project.active_phase.clone();
        let (phase_ids, phase_labels) = build_phase_dropdown_lists(&index);
        let active_timer = new_storage.timer_status().ok().flatten();
        let git_status = new_storage.git_status().ok();

        self.storage = new_storage;
        self.index = index;
        self.active_phase_id = active_phase_id;
        self.phase_ids = phase_ids;
        self.phase_labels = phase_labels;
        self.selected_scope = None;
        self.active_filter = FilterPill::All;
        self.selected_item_id = None;
        self.selected_item_location = None;
        self.selected_item_detail = None;
        self.desc_editor_dirty = false;
        self.card_signals.clear();
        self.active_timer = active_timer;
        self.git_status = git_status;
        self.settings_form
            .sync_from_index(&self.index, &self.storage);
        self.refresh_project_registry();
        Ok(())
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

        let resolved = Self::resolve_or_init_storage(flags.workspace_path.as_deref())
            .and_then(|storage| Self::new_from_storage(core.clone(), storage));

        let app = resolved.unwrap_or_else(|_err| {
            let fallback_storage = TrackerStorage::new(".");
            let registry_path = global_registry_path();
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
                components: Default::default(),
            };
            let empty_index = TrackerIndex {
                project: empty_project,
                phases: Vec::new(),
            };
            let settings_form =
                SettingsFormState::from_index_and_storage(&empty_index, &fallback_storage);
            let project_registry = load_global_registry_from(&registry_path).unwrap_or_default();
            let (project_ids, project_labels) =
                build_project_dropdown_lists(&project_registry, "Nebulous");
            Self {
                core,
                storage: fallback_storage,
                registry_path,
                project_registry,
                project_ids,
                project_labels,
                index: empty_index,
                active_phase_id: "01_core_engine".to_string(),
                phase_ids: Vec::new(),
                phase_labels: Vec::new(),
                selected_scope: None,
                active_filter: FilterPill::All,
                view_mode: ViewMode::Settings,
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
                status_banner: Some(
                    "No active project selected. Add or select a project directory below."
                        .to_string(),
                ),
            }
        });

        (app, Task::none())
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        subscriptions::subscription(self)
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::SelectProjectIndex(idx) => {
                if let Some(proj_id) = self.project_ids.get(idx).cloned() {
                    return self.update(Message::SwitchProject(proj_id));
                }
            }
            Message::SwitchProject(id_or_name) => {
                match set_active_project_in(&self.registry_path, &id_or_name) {
                    Ok(registered) => {
                        let root_path = PathBuf::from(&registered.root_path);
                        match Self::open_or_init_project_root(&root_path, Some(&registered.name)) {
                            Ok(new_storage) => {
                                if let Err(err) = self.switch_to_storage(new_storage) {
                                    self.status_banner = Some(format!(
                                        "Failed to load switched project '{}': {err}",
                                        registered.name
                                    ));
                                } else {
                                    self.status_banner = Some(format!(
                                        "Switched active project to '{}' ({})",
                                        registered.name, registered.root_path
                                    ));
                                }
                            }
                            Err(err) => {
                                self.status_banner = Some(format!(
                                    "Could not open project '{}': {err}",
                                    registered.name
                                ));
                            }
                        }
                    }
                    Err(err) => {
                        self.status_banner =
                            Some(format!("Could not switch project '{id_or_name}': {err}"));
                    }
                }
            }
            Message::OpenAddProject => {
                self.view_mode = ViewMode::Settings;
            }
            Message::NewProjectPathChanged(val) => {
                self.settings_form.new_project_path_input = val;
            }
            Message::NewProjectNameChanged(val) => {
                self.settings_form.new_project_name_input = val;
            }
            Message::ToggleNewProjectIngest(ingest) => {
                self.settings_form.new_project_ingest_on_add = ingest;
            }
            Message::AddProjectToRegistry => {
                let raw_path = self.settings_form.new_project_path_input.trim().to_string();
                if raw_path.is_empty() {
                    self.status_banner =
                        Some("Project directory path cannot be empty.".to_string());
                    return Task::none();
                }
                let target_dir = expand_tilde_path(&raw_path);
                if !target_dir.exists() {
                    self.status_banner = Some(format!(
                        "Project directory '{}' does not exist.",
                        target_dir.display()
                    ));
                    return Task::none();
                }
                let name_override = {
                    let trimmed = self.settings_form.new_project_name_input.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                };
                match Self::open_or_init_project_root(&target_dir, name_override.as_deref()) {
                    Ok(new_storage) => {
                        if let Some(ref explicit_name) = name_override {
                            if let Ok(mut proj_manifest) = new_storage.read_project() {
                                if proj_manifest.project.name != *explicit_name {
                                    proj_manifest.project.name = explicit_name.clone();
                                    let _ = new_storage.write_project(&proj_manifest);
                                }
                            }
                        }
                        if self.settings_form.new_project_ingest_on_add {
                            let _ = new_storage.ingest_project_directory(
                                IngestionOptions::new()
                                    .with_write_manifests(false)
                                    .with_promote_all(true),
                            );
                        }
                        match register_project_in(
                            &self.registry_path,
                            new_storage.project_root(),
                            name_override.as_deref(),
                            Vec::new(),
                            true,
                        ) {
                            Ok(registered) => {
                                self.settings_form.new_project_path_input.clear();
                                self.settings_form.new_project_name_input.clear();
                                if let Err(err) = self.switch_to_storage(new_storage) {
                                    self.status_banner = Some(format!(
                                        "Registered '{}' but failed to switch: {err}",
                                        registered.name
                                    ));
                                } else {
                                    self.status_banner = Some(format!(
                                        "Added and switched to project '{}' ({})",
                                        registered.name, registered.root_path
                                    ));
                                }
                            }
                            Err(err) => {
                                self.status_banner =
                                    Some(format!("Failed to register project: {err}"));
                            }
                        }
                    }
                    Err(err) => {
                        self.status_banner = Some(format!(
                            "Failed to initialize project at '{}': {err}",
                            target_dir.display()
                        ));
                    }
                }
            }
            Message::RemoveProjectFromRegistry(id_or_name) => {
                let was_active_id = self.project_registry.active_project_id.clone();
                let current_root_str = self
                    .storage
                    .project_root()
                    .to_string_lossy()
                    .replace('\\', "/");
                match remove_project_in(&self.registry_path, &id_or_name) {
                    Ok(removed) => {
                        self.project_registry =
                            load_global_registry_from(&self.registry_path).unwrap_or_default();
                        let removed_was_active = was_active_id.as_deref()
                            == Some(removed.id.as_str())
                            || removed.root_path == current_root_str;
                        if removed_was_active {
                            if let Some(next_proj) = self.project_registry.active_project().cloned()
                            {
                                let next_root = PathBuf::from(&next_proj.root_path);
                                if let Ok(next_storage) = Self::open_or_init_project_root(
                                    &next_root,
                                    Some(&next_proj.name),
                                ) {
                                    let _ = self.switch_to_storage(next_storage);
                                    self.status_banner = Some(format!(
                                        "Removed project '{}' ({}) and switched to '{}'",
                                        removed.name, removed.id, next_proj.name
                                    ));
                                    return Task::none();
                                }
                            }
                        }
                        let (ids, labels) = build_project_dropdown_lists(
                            &self.project_registry,
                            &self.index.project.project.name,
                        );
                        self.project_ids = ids;
                        self.project_labels = labels;
                        self.status_banner = Some(format!(
                            "Removed project '{}' ({}) from registry",
                            removed.name, removed.id
                        ));
                    }
                    Err(err) => {
                        self.status_banner =
                            Some(format!("Could not remove project '{id_or_name}': {err}"));
                    }
                }
            }

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

            // Directory Ingestion & Component Pinning handlers (`Phases & Scopes` view)
            Message::IngestSubpathChanged(val) => {
                self.settings_form.ingest_subpath_input = val;
            }
            Message::ToggleIngestWriteManifests(write) => {
                self.settings_form.ingest_write_manifests = write;
            }
            Message::ToggleIngestPromoteAll(promote) => {
                self.settings_form.ingest_promote_all = promote;
            }
            Message::IngestProjectDirectory => {
                if !self.storage.project_toml_path().is_file()
                    || validate_safe_project_root(&self.storage.project_root()).is_err()
                {
                    self.status_banner = Some(
                        "Cannot scan directory: no valid active project is loaded. Select or add a project in Settings first."
                            .to_string(),
                    );
                    return Task::none();
                }

                let project_root = self
                    .storage
                    .project_root()
                    .canonicalize()
                    .unwrap_or_else(|_| self.storage.project_root());
                let mut opts = IngestionOptions::new()
                    .with_write_manifests(self.settings_form.ingest_write_manifests)
                    .with_promote_all(self.settings_form.ingest_promote_all)
                    .with_promote_phase(self.active_phase_id.clone());

                let raw_subpath = self.settings_form.ingest_subpath_input.trim();
                if !raw_subpath.is_empty() {
                    let expanded = expand_tilde_path(raw_subpath);
                    if expanded.is_absolute() || raw_subpath.starts_with('~') {
                        let resolved = expanded.canonicalize().unwrap_or(expanded);
                        match resolved.strip_prefix(&project_root) {
                            Ok(rel) => {
                                let rel_str = rel
                                    .to_string_lossy()
                                    .replace('\\', "/")
                                    .trim_matches('/')
                                    .to_string();
                                if !rel_str.is_empty() && rel_str != "." {
                                    opts = opts.with_subpath(rel_str);
                                }
                            }
                            Err(_) => {
                                self.status_banner = Some(format!(
                                    "Scan path must be within the active project ({})",
                                    project_root.display()
                                ));
                                return Task::none();
                            }
                        }
                    } else {
                        if expanded
                            .components()
                            .any(|c| matches!(c, std::path::Component::ParentDir))
                        {
                            self.status_banner = Some(format!(
                                "Scan path must be within the active project ({})",
                                project_root.display()
                            ));
                            return Task::none();
                        }
                        let joined = project_root.join(&expanded);
                        if joined.exists() {
                            let resolved = joined.canonicalize().unwrap_or(joined);
                            if !resolved.starts_with(&project_root) {
                                self.status_banner = Some(format!(
                                    "Scan path must be within the active project ({})",
                                    project_root.display()
                                ));
                                return Task::none();
                            }
                        }
                        opts = opts.with_subpath(raw_subpath);
                    }
                }
                match self.storage.ingest_project_directory(opts) {
                    Ok(report) => {
                        let _ = self.reload_indexes();
                        self.settings_form
                            .sync_from_index(&self.index, &self.storage);
                        self.status_banner = Some(format!(
                            "Ingested {} components ({} files indexed, {} manifests written, {} scopes promoted)",
                            report.components.len(),
                            report.total_files_indexed,
                            report.manifests_written.len(),
                            report.promoted_scopes.len()
                        ));
                    }
                    Err(err) => {
                        self.status_banner = Some(format!("Directory ingestion failed: {err}"));
                    }
                }
            }
            Message::SelectComponentForPin(comp_id) => {
                self.settings_form.selected_component_id = comp_id.clone();
                if let Some(comp) = self.index.project.components.find(&comp_id) {
                    self.settings_form.component_tags_input = comp.tags.join(", ");
                    self.settings_form.component_flags_input = comp.flags.join(", ");
                    self.settings_form.component_build_script_input = comp
                        .pinned_release
                        .as_ref()
                        .and_then(|p| p.build_script_path.clone())
                        .unwrap_or_default();
                }
            }
            Message::WriteComponentManifest(comp_id) => {
                match self.storage.read_component_manifest(&comp_id) {
                    Ok(manifest) => match self.storage.write_component_manifest(&manifest) {
                        Ok(path) => {
                            let _ = self.reload_indexes();
                            self.settings_form
                                .sync_from_index(&self.index, &self.storage);
                            self.status_banner =
                                Some(format!("Wrote component manifest to {}", path.display()));
                        }
                        Err(err) => {
                            self.status_banner =
                                Some(format!("Failed to write nebulous.component.toml: {err}"));
                        }
                    },
                    Err(err) => {
                        self.status_banner =
                            Some(format!("Component '{comp_id}' not found: {err}"));
                    }
                }
            }
            Message::PromoteComponentToScope(comp_id) => {
                let phase_id = self.active_phase_id.clone();
                match self
                    .storage
                    .promote_component_to_scope(Some(&phase_id), &comp_id, None, None)
                {
                    Ok(scope) => {
                        let _ = self.reload_indexes();
                        self.status_banner = Some(format!(
                            "Promoted component '{}' to scope '{}' in phase '{}' ({} tracked files)",
                            comp_id,
                            scope.scope.id,
                            phase_id,
                            scope.scope.tracked_files.len()
                        ));
                    }
                    Err(err) => {
                        self.status_banner =
                            Some(format!("Failed to promote '{comp_id}' to scope: {err}"));
                    }
                }
            }
            Message::ComponentTagsInputChanged(val) => {
                self.settings_form.component_tags_input = val;
            }
            Message::ComponentFlagsInputChanged(val) => {
                self.settings_form.component_flags_input = val;
            }
            Message::ComponentBuildScriptInputChanged(val) => {
                self.settings_form.component_build_script_input = val;
            }
            Message::SaveComponentMetadata => {
                let Some(comp_id) = self.effective_selected_component_id() else {
                    self.status_banner = Some("Select or enter a component ID first.".to_string());
                    return Task::none();
                };
                match self.storage.read_component_manifest(&comp_id) {
                    Ok(mut manifest) => {
                        manifest.tags = parse_comma_list(&self.settings_form.component_tags_input);
                        manifest.flags =
                            parse_comma_list(&self.settings_form.component_flags_input);
                        if let Some(ref mut pin) = manifest.pinned_release {
                            let script = self.settings_form.component_build_script_input.trim();
                            pin.build_script_path = if script.is_empty() {
                                None
                            } else {
                                Some(script.to_string())
                            };
                        }
                        match self.storage.write_component_manifest(&manifest) {
                            Ok(path) => {
                                let _ = self.reload_indexes();
                                self.settings_form
                                    .sync_from_index(&self.index, &self.storage);
                                self.status_banner = Some(format!(
                                    "Saved component '{}' metadata to {}",
                                    manifest.id,
                                    path.display()
                                ));
                            }
                            Err(err) => {
                                self.status_banner =
                                    Some(format!("Failed to save component metadata: {err}"));
                            }
                        }
                    }
                    Err(err) => {
                        self.status_banner =
                            Some(format!("Could not read component '{comp_id}': {err}"));
                    }
                }
            }
            Message::ComponentDepNameChanged(val) => {
                self.settings_form.dep_name_input = val;
            }
            Message::ComponentDepKindChanged(val) => {
                self.settings_form.dep_kind_input = val;
            }
            Message::ComponentDepConstraintChanged(val) => {
                self.settings_form.dep_constraint_input = val;
            }
            Message::ComponentDepVersionChanged(val) => {
                self.settings_form.dep_version_input = val;
            }
            Message::ComponentDepSourceChanged(val) => {
                self.settings_form.dep_source_input = val;
            }
            Message::UpsertComponentDependency => {
                let Some(comp_id) = self.effective_selected_component_id() else {
                    self.status_banner = Some("Select or enter a component ID first.".to_string());
                    return Task::none();
                };
                let dep_name = self.settings_form.dep_name_input.trim().to_string();
                if dep_name.is_empty() {
                    self.status_banner =
                        Some("Dependency package name must not be empty.".to_string());
                    return Task::none();
                }
                let kind = {
                    let trimmed = self.settings_form.dep_kind_input.trim();
                    if trimmed.is_empty() {
                        "apt_sid".to_string()
                    } else {
                        trimmed.to_string()
                    }
                };
                let constraint = self.settings_form.dep_constraint_input.trim().to_string();
                let current_version = self.settings_form.dep_version_input.trim().to_string();
                let upstream_source = self.settings_form.dep_source_input.trim().to_string();

                match self.storage.read_component_manifest(&comp_id) {
                    Ok(mut manifest) => {
                        if let Some(existing) = manifest
                            .dependencies
                            .iter_mut()
                            .find(|d| d.name == dep_name)
                        {
                            existing.kind = kind.clone();
                            existing.version_constraint = constraint;
                            existing.current_version = current_version.clone();
                            existing.upstream_source = upstream_source;
                        } else {
                            manifest.dependencies.push(ComponentDependency::new(
                                dep_name.clone(),
                                kind.clone(),
                                constraint,
                                current_version.clone(),
                                upstream_source,
                            ));
                        }
                        match self.storage.write_component_manifest(&manifest) {
                            Ok(_) => {
                                self.settings_form.dep_name_input.clear();
                                self.settings_form.dep_constraint_input.clear();
                                self.settings_form.dep_version_input.clear();
                                self.settings_form.dep_source_input.clear();
                                let _ = self.reload_indexes();
                                self.settings_form
                                    .sync_from_index(&self.index, &self.storage);
                                self.status_banner = Some(format!(
                                    "Upserted dependency '{dep_name}' ({current_version}) on component '{comp_id}'"
                                ));
                            }
                            Err(err) => {
                                self.status_banner =
                                    Some(format!("Failed to save dependency: {err}"));
                            }
                        }
                    }
                    Err(err) => {
                        self.status_banner =
                            Some(format!("Could not read component '{comp_id}': {err}"));
                    }
                }
            }
            Message::PinSelectedComponentRelease => {
                if let Some(comp_id) = self.effective_selected_component_id() {
                    return self.update(Message::PinComponentRelease(comp_id));
                }
                self.status_banner =
                    Some("Select or enter a component ID to pin its release.".to_string());
            }
            Message::PinComponentRelease(comp_id) => {
                let flags = parse_comma_list(&self.settings_form.component_flags_input);
                let build_script = {
                    let trimmed = self.settings_form.component_build_script_input.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                };
                if self.effective_selected_component_id().as_deref() == Some(comp_id.as_str()) {
                    if let Ok(mut manifest) = self.storage.read_component_manifest(&comp_id) {
                        manifest.tags = parse_comma_list(&self.settings_form.component_tags_input);
                        manifest.flags = flags.clone();
                        let _ = self.storage.write_component_manifest(&manifest);
                    }
                }
                match self
                    .storage
                    .pin_component_release(&comp_id, None, build_script, flags, None)
                {
                    Ok(pinned_manifest) => {
                        let pin_sha = pinned_manifest
                            .pinned_release
                            .as_ref()
                            .map(|p| p.commit_sha.clone())
                            .unwrap_or_else(|| "uncommitted".to_string());
                        let _ = self.reload_indexes();
                        self.settings_form
                            .sync_from_index(&self.index, &self.storage);
                        self.status_banner = Some(format!(
                            "Pinned known-good release for '{}' at commit {}",
                            pinned_manifest.id, pin_sha
                        ));
                    }
                    Err(err) => {
                        self.status_banner =
                            Some(format!("Failed to pin component '{comp_id}': {err}"));
                    }
                }
            }
            Message::RollbackSelectedComponentRelease => {
                if let Some(comp_id) = self.effective_selected_component_id() {
                    return self.update(Message::RollbackComponentRelease(comp_id));
                }
                self.status_banner =
                    Some("Select or enter a component ID to roll back.".to_string());
            }
            Message::RollbackComponentRelease(comp_id) => {
                match self.storage.rollback_component_to_pinned(&comp_id) {
                    Ok(restored) => {
                        let pin_sha = restored
                            .pinned_release
                            .as_ref()
                            .map(|p| p.commit_sha.clone())
                            .unwrap_or_else(|| "uncommitted".to_string());
                        let _ = self.reload_indexes();
                        self.settings_form
                            .sync_from_index(&self.index, &self.storage);
                        self.status_banner = Some(format!(
                            "Rolled back component '{}' to pinned release ({})",
                            restored.id, pin_sha
                        ));
                    }
                    Err(err) => {
                        self.status_banner =
                            Some(format!("Cannot roll back component '{comp_id}': {err}"));
                    }
                }
            }

            // Phase & Scope Manager updates
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

            // Git Operations, Documented Commit Builder, Commit Policy & SecureStorage handlers
            Message::GitFetch => match self.storage.git_fetch(None) {
                Ok(status) => {
                    let ahead = status.ahead_count;
                    let behind = status.behind_count;
                    self.git_status = Some(status);
                    let _ = self.reload_indexes();
                    self.status_banner = Some(format!(
                        "Fetched remote tracking branch (↑{ahead} ↓{behind})"
                    ));
                }
                Err(err) => {
                    self.status_banner = Some(format!("Git fetch note: {err}"));
                }
            },
            Message::GitPull => match self.storage.git_pull(None) {
                Ok(updated) => {
                    let _ = self.reload_indexes();
                    if updated {
                        self.status_banner = Some(
                            "Pulled remote changes and reconciled .tracker/ indexes.".to_string(),
                        );
                    } else {
                        self.status_banner =
                            Some("Git pull complete (already up to date).".to_string());
                    }
                }
                Err(err) => {
                    self.status_banner = Some(format!("Git pull failed: {err}"));
                }
            },
            Message::GitPush => match self.storage.git_push(None) {
                Ok(()) => {
                    let _ = self.reload_indexes();
                    self.status_banner = Some(format!(
                        "Pushed '{}' to remote '{}'.",
                        self.index.project.git.branch, self.index.project.git.remote
                    ));
                }
                Err(err) => {
                    self.status_banner = Some(format!("Git push failed: {err}"));
                }
            },
            Message::CommitItemIdChanged(val) => {
                self.settings_form.commit_item_id_input = val;
            }
            Message::CommitScopeIdChanged(val) => {
                self.settings_form.commit_scope_id_input = val;
            }
            Message::CommitSummaryChanged(val) => {
                self.settings_form.commit_summary_input = val;
            }
            Message::CommitBodyChanged(val) => {
                self.settings_form.commit_body_input = val;
            }
            Message::SetCommitTrackerOnly(tracker_only) => {
                self.settings_form.commit_tracker_only = tracker_only;
            }
            Message::SubmitDocumentedCommit => {
                let item_id = {
                    let trimmed = self.settings_form.commit_item_id_input.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                };
                let scope_id = {
                    let trimmed = self.settings_form.commit_scope_id_input.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                };
                let body = {
                    let trimmed = self.settings_form.commit_body_input.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                };
                let input = DocumentedCommitInput {
                    item_id,
                    scope_id,
                    summary: self.settings_form.commit_summary_input.clone(),
                    body,
                    tracker_only: self.settings_form.commit_tracker_only,
                    paths: Vec::new(),
                };
                match self.storage.git_commit_documented(input) {
                    Ok(outcome) => {
                        self.settings_form.commit_summary_input.clear();
                        self.settings_form.commit_body_input.clear();
                        let _ = self.reload_indexes();
                        self.status_banner = Some(format!(
                            "Committed {} ({} files, doc_updated={})",
                            outcome.commit_sha,
                            outcome.changed_files.len(),
                            outcome.documentation_updated
                        ));
                    }
                    Err(err) => {
                        self.status_banner = Some(format!("Documented commit rejected: {err}"));
                    }
                }
            }
            Message::ToggleCommitPolicyRequireItemId(val) => {
                self.settings_form.commit_policy_require_item_id = val;
            }
            Message::ToggleCommitPolicyRequireScope(val) => {
                self.settings_form.commit_policy_require_scope = val;
            }
            Message::ToggleCommitPolicyAutoAppendDesc(val) => {
                self.settings_form.commit_policy_auto_append_to_desc = val;
            }
            Message::CommitPolicyMinSummaryLengthChanged(val) => {
                self.settings_form.commit_policy_min_summary_len_input = val;
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
                let Ok(min_summary_len) = self
                    .settings_form
                    .commit_policy_min_summary_len_input
                    .trim()
                    .parse::<usize>()
                else {
                    self.status_banner = Some(
                        "Minimum commit summary length must be a valid non-negative integer."
                            .to_string(),
                    );
                    return Task::none();
                };
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
                        project.git.commit_policy.require_item_id =
                            self.settings_form.commit_policy_require_item_id;
                        project.git.commit_policy.require_scope =
                            self.settings_form.commit_policy_require_scope;
                        project.git.commit_policy.require_summary =
                            self.settings_form.commit_policy_require_summary;
                        project.git.commit_policy.min_summary_length = min_summary_len;
                        project.git.commit_policy.auto_append_to_desc =
                            self.settings_form.commit_policy_auto_append_to_desc;
                        match self.storage.write_project(&project) {
                            Ok(()) => {
                                let _ = self.reload_indexes();
                                self.settings_form
                                    .sync_from_index(&self.index, &self.storage);
                                self.status_banner = Some(format!(
                                    "Saved Git & Commit Policy settings (mode={}, interval={}m, {remote}/{branch})",
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
            Message::SshKeyCommentChanged(val) => {
                self.settings_form.ssh_comment_input = val;
            }
            Message::SshKeyImportInputChanged(val) => {
                self.settings_form.ssh_private_key_import_input = val;
            }
            Message::SshKeyPassphraseChanged(val) => {
                self.settings_form.ssh_passphrase_input = val;
            }
            Message::GenerateSshKeypair => {
                let comment = {
                    let trimmed = self.settings_form.ssh_comment_input.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                };
                match self.storage.generate_ssh_keypair(comment.as_deref()) {
                    Ok(record) => {
                        let pub_info = record.public_info();
                        self.settings_form.stored_ssh_public_key = Some(pub_info.clone());
                        self.settings_form.has_stored_ssh_key = true;
                        self.status_banner = Some(format!(
                            "Generated Ed25519 SSH keypair ({}) in SecureStorage",
                            pub_info.fingerprint_sha256
                        ));
                    }
                    Err(err) => {
                        self.status_banner =
                            Some(format!("Failed to generate Ed25519 SSH keypair: {err}"));
                    }
                }
            }
            Message::ImportSshKeypair => {
                let raw_input = self
                    .settings_form
                    .ssh_private_key_import_input
                    .trim()
                    .to_string();
                if raw_input.is_empty() {
                    self.status_banner =
                        Some("Enter a private key file path or OpenSSH PEM to import.".to_string());
                    return Task::none();
                }
                let passphrase = {
                    let trimmed = self.settings_form.ssh_passphrase_input.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                };
                let expanded_path = expand_tilde_path(&raw_input);
                let import_res = if expanded_path.is_file() {
                    match std::fs::read_to_string(&expanded_path) {
                        Ok(pem) => self.storage.import_ssh_keypair(&pem, passphrase.as_deref()),
                        Err(source) => Err(TrackerError::Io {
                            path: expanded_path,
                            source,
                        }),
                    }
                } else {
                    let normalized_pem = normalize_openssh_pem_input(&raw_input);
                    self.storage
                        .import_ssh_keypair(&normalized_pem, passphrase.as_deref())
                };
                match import_res {
                    Ok(record) => {
                        let pub_info = record.public_info();
                        self.settings_form.ssh_private_key_import_input.clear();
                        self.settings_form.ssh_passphrase_input.clear();
                        self.settings_form.stored_ssh_public_key = Some(pub_info.clone());
                        self.settings_form.has_stored_ssh_key = true;
                        self.status_banner = Some(format!(
                            "Imported SSH keypair ({}) into SecureStorage",
                            pub_info.fingerprint_sha256
                        ));
                    }
                    Err(err) => {
                        self.status_banner = Some(format!("Failed to import SSH keypair: {err}"));
                    }
                }
            }
            Message::CopySshPublicKey => {
                if let Some(ref pub_info) = self.settings_form.stored_ssh_public_key {
                    let pub_key = pub_info.public_key_openssh.clone();
                    let fp = pub_info.fingerprint_sha256.clone();
                    return self.copy_text_to_clipboard(
                        pub_key,
                        format!("Copied SSH public key ({fp}) to clipboard"),
                    );
                }
                self.status_banner =
                    Some("No SSH public key stored in SecureStorage to copy.".to_string());
            }
            Message::DeleteSshKeypair => match self.storage.delete_ssh_keypair() {
                Ok(()) => {
                    self.settings_form.stored_ssh_public_key = None;
                    self.settings_form.has_stored_ssh_key = false;
                    self.status_banner =
                        Some("Deleted SSH keypair from SecureStorage.".to_string());
                }
                Err(err) => {
                    self.settings_form.stored_ssh_public_key = None;
                    self.settings_form.has_stored_ssh_key = false;
                    self.status_banner = Some(format!("SSH keypair note: {err}"));
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
            ViewMode::PhasesAndScopes => phases_scopes_view::view_phases_and_scopes(self),
            ViewMode::Settings => settings_view::view_settings(self),
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

fn resolve_app_registry_path(storage: &TrackerStorage) -> PathBuf {
    let temp_dir = std::env::temp_dir();
    let project_root = storage.project_root();
    if project_root.starts_with(&temp_dir) {
        let root_leaf = project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("temp");
        return temp_dir
            .join(format!("nebulous_test_registry_{}", std::process::id()))
            .join(root_leaf)
            .join("projects.toml");
    }
    global_registry_path()
}

fn build_project_dropdown_lists(
    registry: &GlobalProjectRegistry,
    fallback_project_name: &str,
) -> (Vec<String>, Vec<String>) {
    if registry.projects.is_empty() {
        let fallback_id = fallback_project_name.to_ascii_lowercase();
        return (
            vec![fallback_id],
            vec![format!("◆ {fallback_project_name}")],
        );
    }
    let mut ids = Vec::with_capacity(registry.projects.len());
    let mut labels = Vec::with_capacity(registry.projects.len());
    for proj in &registry.projects {
        ids.push(proj.id.clone());
        labels.push(format!("◆ {}", proj.name));
    }
    (ids, labels)
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

fn parse_comma_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn expand_tilde_path(raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if trimmed == "~" || trimmed.starts_with("~/") || trimmed.starts_with("~\\") {
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            let mut buf = PathBuf::from(home);
            let rest = trimmed
                .trim_start_matches('~')
                .trim_start_matches(['/', '\\']);
            if !rest.is_empty() {
                buf.push(rest);
            }
            return buf;
        }
    }
    PathBuf::from(trimmed)
}

/// Normalizes an inline OpenSSH private key PEM string pasted into a single-line `text_input`,
/// restoring RFC 7468 header/footer newlines and 70-character base64 line wrapping if flattened.
fn normalize_openssh_pem_input(raw: &str) -> String {
    let unescaped = raw.trim().replace("\\r\\n", "\n").replace("\\n", "\n");
    if unescaped.contains('\n') {
        return unescaped;
    }
    if let (Some(begin_end), Some(end_start)) = (
        unescaped.find("-----").and_then(|start| {
            let rest = &unescaped[start + 5..];
            rest.find("-----").map(|rel| start + 5 + rel + 5)
        }),
        unescaped.rfind("-----END "),
    ) {
        if begin_end < end_start {
            let header = unescaped[..begin_end].trim();
            let footer = unescaped[end_start..].trim();
            let body_compact: String = unescaped[begin_end..end_start]
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            if !body_compact.is_empty() {
                let mut wrapped = String::with_capacity(unescaped.len() + 32);
                wrapped.push_str(header);
                wrapped.push('\n');
                for chunk in body_compact.as_bytes().chunks(70) {
                    if let Ok(line) = std::str::from_utf8(chunk) {
                        wrapped.push_str(line);
                        wrapped.push('\n');
                    }
                }
                wrapped.push_str(footer);
                wrapped.push('\n');
                return wrapped;
            }
        }
    }
    unescaped
}

/// Computes a structural widget-tree fingerprint `(total_nodes, depth_child_counts)` for an `iced`
/// [`Element`] by traversing `as_widget().children()` recursively.
pub fn widget_tree_fingerprint(element: &Element<'_, Message>) -> (usize, Vec<usize>) {
    fn visit_tree(tree: &cosmic::iced::advanced::widget::Tree, counts: &mut Vec<usize>) -> usize {
        counts.push(tree.children.len());
        let mut total = 1usize;
        for child in &tree.children {
            total = total.saturating_add(visit_tree(child, counts));
        }
        total
    }

    let root_children = element.as_widget().children();
    let mut counts = vec![root_children.len()];
    let mut total = 1usize;
    for child in &root_children {
        total = total.saturating_add(visit_tree(child, &mut counts));
    }
    (total, counts)
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
/// 6. Distinct widget hierarchies for `ViewMode::PhasesAndScopes` vs `ViewMode::Settings`.
/// 7. End-to-end Directory Ingestion, Component Pinning & Rollback, Documented Git Commits, and
///    Multi-Project Registry switching in an isolated temporary workspace.
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

    // Create sample crate & script directories for directory ingestion and git commit testing
    let crate_dir = path.join("crates").join("backend_core").join("src");
    std::fs::create_dir_all(&crate_dir).map_err(|source| TrackerError::Io {
        path: crate_dir.clone(),
        source,
    })?;
    std::fs::write(
        path.join("crates").join("backend_core").join("Cargo.toml"),
        "[package]\nname = \"backend_core\"\nversion = \"0.1.0\"\n",
    )
    .map_err(|source| TrackerError::Io {
        path: path.clone(),
        source,
    })?;
    std::fs::write(crate_dir.join("lib.rs"), "pub fn init_engine() {}\n").map_err(|source| {
        TrackerError::Io {
            path: crate_dir,
            source,
        }
    })?;

    let scripts_dir = path.join("scripts");
    std::fs::create_dir_all(&scripts_dir).map_err(|source| TrackerError::Io {
        path: scripts_dir.clone(),
        source,
    })?;
    std::fs::write(
        scripts_dir.join("build-iso.sh"),
        "#!/usr/bin/env bash\necho 'building iso'\n",
    )
    .map_err(|source| TrackerError::Io {
        path: scripts_dir,
        source,
    })?;

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
    let self_test_reg_dir = std::env::temp_dir().join(format!(
        "nebulous_self_test_reg_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _reg_guard = SelfTestTempGuard {
        path: self_test_reg_dir.clone(),
    };
    let mut app = NebulousApp::new_from_storage_with_registry(
        Core::default(),
        storage,
        self_test_reg_dir.join("projects.toml"),
    )?;

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

    // 7. Exercise Metrics computation and verify ViewMode::PhasesAndScopes and ViewMode::Settings
    // render distinct widget hierarchies and section structures.
    let metrics_report = app.compute_current_metrics();
    if metrics_report.phases.is_empty() || metrics_report.total_items == 0 {
        return Err(TrackerError::ValidationError(
            "Metrics report should include phases and tracked items".to_string(),
        ));
    }

    let _ = app.update(Message::SwitchView(ViewMode::PhasesAndScopes));
    let phases_sections = app.active_view_section_titles();
    let phases_fingerprint = {
        let view_el = app.view();
        widget_tree_fingerprint(&view_el)
    };

    let _ = app.update(Message::SwitchView(ViewMode::Settings));
    let settings_sections = app.active_view_section_titles();
    let settings_fingerprint = {
        let view_el = app.view();
        widget_tree_fingerprint(&view_el)
    };

    if phases_sections == settings_sections || phases_fingerprint == settings_fingerprint {
        return Err(TrackerError::ValidationError(
            "ViewMode::PhasesAndScopes and ViewMode::Settings must render distinct widget hierarchies"
                .to_string(),
        ));
    }

    for mode in [ViewMode::Metrics, ViewMode::Board] {
        let _ = app.update(Message::SwitchView(mode));
        let _view = app.view();
    }

    // 8. Exercise Directory Ingestion, Component Pinning & Rollback, Documented Git Commits,
    // and Multi-Project Registry switching in an isolated temporary workspace.
    {
        let (iso_guard, iso_storage) = seed_self_test_temp_workspace()?;
        let mut iso_app = NebulousApp::new_from_storage(Core::default(), iso_storage)?;

        // Directory ingestion & component manifest / pin / rollback
        let _ = iso_app.update(Message::ToggleIngestWriteManifests(true));
        let _ = iso_app.update(Message::IngestProjectDirectory);
        if iso_app.index.project.components.is_empty() {
            return Err(TrackerError::ValidationError(
                "Expected directory ingestion in self-test to discover components".to_string(),
            ));
        }

        let first_comp_id = iso_app.index.project.components.registry[0].id.clone();
        let _ = iso_app.update(Message::PromoteComponentToScope(first_comp_id.clone()));
        let _ = iso_app.update(Message::SelectComponentForPin(first_comp_id.clone()));
        let _ = iso_app.update(Message::ComponentDepNameChanged(
            "linux-image-xanmod".to_string(),
        ));
        let _ = iso_app.update(Message::ComponentDepKindChanged(
            "kernel_xanmod".to_string(),
        ));
        let _ = iso_app.update(Message::ComponentDepVersionChanged(
            "6.11.5-xanmod1".to_string(),
        ));
        let _ = iso_app.update(Message::UpsertComponentDependency);
        let _ = iso_app.update(Message::PinSelectedComponentRelease);
        if iso_app
            .selected_component_manifest()
            .and_then(|c| c.pinned_release.as_ref())
            .is_none()
        {
            return Err(TrackerError::ValidationError(
                "Expected PinSelectedComponentRelease to record pinned_release".to_string(),
            ));
        }
        let _ = iso_app.update(Message::RollbackSelectedComponentRelease);

        // Initialize Git repository in isolated workspace and exercise Documented Commit workflow
        let _ = Command::new("git")
            .arg("init")
            .current_dir(&iso_guard.path)
            .output();
        let _ = iso_app.update(Message::CommitItemIdChanged("BUG-0001".to_string()));
        let _ = iso_app.update(Message::SetCommitTrackerOnly(false));
        let _ = iso_app.update(Message::CommitSummaryChanged(
            "Implement self-test documented commit verification".to_string(),
        ));
        let _ = iso_app.update(Message::SubmitDocumentedCommit);
        let (_, bug1_detail) = iso_app.storage.load_item_detail_by_id("BUG-0001")?;
        if bug1_detail.manifest.history.previous_fixes.is_empty()
            || !bug1_detail
                .description_markdown
                .contains("## Fix Log & Agent Notes")
        {
            return Err(TrackerError::ValidationError(
                "Expected SubmitDocumentedCommit to append fix ledger and desc.md notes"
                    .to_string(),
            ));
        }

        // Exercise Multi-Project Registry addition & project switching
        let second_proj_dir = iso_guard.path.join("second_project_novos");
        std::fs::create_dir_all(&second_proj_dir).map_err(|source| TrackerError::Io {
            path: second_proj_dir.clone(),
            source,
        })?;
        let _ = iso_app.update(Message::NewProjectPathChanged(
            second_proj_dir.to_string_lossy().to_string(),
        ));
        let _ = iso_app.update(Message::NewProjectNameChanged("NovOS".to_string()));
        let _ = iso_app.update(Message::AddProjectToRegistry);
        if iso_app.project_name() != "NovOS" {
            return Err(TrackerError::ValidationError(format!(
                "Expected active project to switch to 'NovOS', got '{}'",
                iso_app.project_name()
            )));
        }
        let _ = iso_app.update(Message::SelectProjectIndex(0));
        if iso_app.project_name() != "Nebulous" {
            return Err(TrackerError::ValidationError(format!(
                "Expected SelectProjectIndex(0) to switch back to 'Nebulous', got '{}'",
                iso_app.project_name()
            )));
        }
        let _ = std::fs::remove_dir_all(
            iso_app
                .registry_path
                .parent()
                .unwrap_or(&iso_app.registry_path),
        );
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

    #[test]
    fn test_phases_and_scopes_view_vs_settings_view_render_distinct_hierarchies() {
        let (_temp, storage) = create_sample_tracker_workspace();
        let mut app = NebulousApp::new_from_storage(Core::default(), storage).expect("boot app");

        let _ = app.update(Message::SwitchView(ViewMode::Board));
        let board_titles = app.active_view_section_titles();
        let board_fp = widget_tree_fingerprint(&app.view());

        let _ = app.update(Message::SwitchView(ViewMode::PhasesAndScopes));
        let phases_titles = app.active_view_section_titles();
        let phases_fp = widget_tree_fingerprint(&app.view());
        let direct_phases_fp =
            widget_tree_fingerprint(&phases_scopes_view::view_phases_and_scopes(&app));

        let _ = app.update(Message::SwitchView(ViewMode::Metrics));
        let metrics_titles = app.active_view_section_titles();
        let metrics_fp = widget_tree_fingerprint(&app.view());

        let _ = app.update(Message::SwitchView(ViewMode::Settings));
        let settings_titles = app.active_view_section_titles();
        let settings_fp = widget_tree_fingerprint(&app.view());
        let direct_settings_fp = widget_tree_fingerprint(&settings_view::view_settings(&app));

        // Verify PhasesAndScopes and Settings have completely distinct section titles and widget trees
        assert_ne!(phases_titles, settings_titles);
        assert_ne!(phases_fp, settings_fp);
        assert_ne!(direct_phases_fp, direct_settings_fp);
        assert!(phases_titles[0].contains("Project Directory Ingestion"));
        assert!(settings_titles[0].contains("Multi-Project Registry"));

        // Verify all 4 ViewModes have pairwise distinct section titles and widget tree fingerprints
        let all_titles = [board_titles, phases_titles, metrics_titles, settings_titles];
        let all_fps = [board_fp, phases_fp, metrics_fp, settings_fp];
        for i in 0..4 {
            for j in (i + 1)..4 {
                assert_ne!(all_titles[i], all_titles[j]);
                assert_ne!(all_fps[i], all_fps[j]);
            }
        }
    }

    #[test]
    fn test_directory_ingestion_component_manifest_pinning_and_rollback_workflows() {
        let (temp, storage) = create_sample_tracker_workspace();

        // Seed sample Cargo crate and shell script directory inside the workspace
        let crate_src = temp.path().join("crates").join("backend_core").join("src");
        fs::create_dir_all(&crate_src).unwrap();
        fs::write(
            temp.path()
                .join("crates")
                .join("backend_core")
                .join("Cargo.toml"),
            "[package]\nname = \"backend_core\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(crate_src.join("lib.rs"), "pub fn run() {}\n").unwrap();

        let scripts_dir = temp.path().join("scripts");
        fs::create_dir_all(&scripts_dir).unwrap();
        fs::write(
            scripts_dir.join("build-iso.sh"),
            "#!/usr/bin/env bash\nset -euo pipefail\n",
        )
        .unwrap();

        let mut app =
            NebulousApp::new_from_storage(Core::default(), storage.clone()).expect("boot app");
        let _ = app.update(Message::SwitchView(ViewMode::PhasesAndScopes));

        // 1. Scan directory and write nebulous.component.toml manifests
        let _ = app.update(Message::ToggleIngestWriteManifests(true));
        let _ = app.update(Message::ToggleIngestPromoteAll(false));
        let _ = app.update(Message::IngestProjectDirectory);
        assert!(!app.index.project.components.is_empty());
        assert!(temp
            .path()
            .join("crates")
            .join("backend_core")
            .join("nebulous.component.toml")
            .is_file());

        // Render Phases & Scopes view after ingestion
        let _ = app.view();

        // 2. Promote discovered component to a domain scope in the active phase
        let comp_id = "backend_core".to_string();
        let _ = app.update(Message::PromoteComponentToScope(comp_id.clone()));
        let scope_entry = app
            .active_phase()
            .unwrap()
            .scope("backend_core")
            .expect("promoted scope");
        assert_eq!(
            scope_entry.manifest.scope.bound_component.as_deref(),
            Some("backend_core")
        );
        assert!(!scope_entry.manifest.scope.tracked_files.is_empty());

        // 3. Select component for Pin Builder and update tags/flags
        let _ = app.update(Message::SelectComponentForPin(comp_id.clone()));
        let _ = app.update(Message::ComponentTagsInputChanged(
            "core, rolling-release, critical".to_string(),
        ));
        let _ = app.update(Message::ComponentFlagsInputChanged(
            "--release, --locked".to_string(),
        ));
        let _ = app.update(Message::SaveComponentMetadata);
        let updated_comp = app.selected_component_manifest().unwrap();
        assert_eq!(
            updated_comp.tags,
            vec!["core", "rolling-release", "critical"]
        );
        assert_eq!(updated_comp.flags, vec!["--release", "--locked"]);

        // 4. Attempting rollback before pinning a release fails gracefully with status banner
        let _ = app.update(Message::RollbackSelectedComponentRelease);
        assert!(app
            .status_banner
            .as_deref()
            .unwrap_or_default()
            .contains("Cannot roll back component"));

        // 5. Validation error on empty dependency name
        let _ = app.update(Message::ComponentDepNameChanged("   ".to_string()));
        let _ = app.update(Message::UpsertComponentDependency);
        assert!(app
            .status_banner
            .as_deref()
            .unwrap_or_default()
            .contains("must not be empty"));

        // 6. Upsert rolling-release dependency (`linux-image-xanmod` / `kernel_xanmod`)
        let _ = app.update(Message::ComponentDepNameChanged(
            "linux-image-xanmod".to_string(),
        ));
        let _ = app.update(Message::ComponentDepKindChanged(
            "kernel_xanmod".to_string(),
        ));
        let _ = app.update(Message::ComponentDepConstraintChanged(">=6.11".to_string()));
        let _ = app.update(Message::ComponentDepVersionChanged(
            "6.11.5-xanmod1".to_string(),
        ));
        let _ = app.update(Message::ComponentDepSourceChanged(
            "deb.xanmod.org".to_string(),
        ));
        let _ = app.update(Message::UpsertComponentDependency);
        assert_eq!(
            app.selected_component_manifest()
                .unwrap()
                .dependencies
                .len(),
            1
        );

        // 7. Pin known-good release
        let _ = app.update(Message::ComponentBuildScriptInputChanged(
            "scripts/build-iso.sh".to_string(),
        ));
        let _ = app.update(Message::PinSelectedComponentRelease);
        let pinned_comp = app.selected_component_manifest().unwrap().clone();
        assert!(pinned_comp.pinned_release.is_some());
        assert_eq!(
            pinned_comp
                .pinned_release
                .as_ref()
                .unwrap()
                .build_script_path
                .as_deref(),
            Some("scripts/build-iso.sh")
        );

        // 8. Simulate broken rolling-release upgrade (`6.12.0-rc1-broken`) and roll back to pinned release
        let _ = app.update(Message::ComponentDepNameChanged(
            "linux-image-xanmod".to_string(),
        ));
        let _ = app.update(Message::ComponentDepVersionChanged(
            "6.12.0-rc1-broken".to_string(),
        ));
        let _ = app.update(Message::UpsertComponentDependency);
        assert_eq!(
            app.selected_component_manifest().unwrap().dependencies[0].current_version,
            "6.12.0-rc1-broken"
        );

        let _ = app.update(Message::RollbackSelectedComponentRelease);
        let rolled_back_comp = app.selected_component_manifest().unwrap();
        assert_eq!(
            rolled_back_comp.dependencies[0].current_version,
            "6.11.5-xanmod1"
        );
        assert_eq!(
            rolled_back_comp
                .pinned_release
                .as_ref()
                .unwrap()
                .dependency_pins
                .get("linux-image-xanmod")
                .map(String::as_str),
            Some("6.11.5-xanmod1")
        );
    }

    #[test]
    fn test_multi_project_registry_switcher_add_switch_and_remove() {
        let (temp, storage) = create_sample_tracker_workspace();
        let reg_path = temp.path().join("isolated_registry").join("projects.toml");
        let mut app =
            NebulousApp::new_from_storage_with_registry(Core::default(), storage, reg_path)
                .expect("boot app with isolated registry");

        assert_eq!(app.project_name(), "NebulousTest");
        assert_eq!(app.project_registry.projects.len(), 1);
        assert_eq!(app.selected_project_dropdown_index(), Some(0));

        // Clicking `+ Project` in header bar switches to Settings view
        let _ = app.update(Message::OpenAddProject);
        assert_eq!(app.view_mode, ViewMode::Settings);

        // Validation errors on empty or non-existent directory path
        let _ = app.update(Message::NewProjectPathChanged("   ".to_string()));
        let _ = app.update(Message::AddProjectToRegistry);
        assert!(app
            .status_banner
            .as_deref()
            .unwrap_or_default()
            .contains("cannot be empty"));

        let _ = app.update(Message::NewProjectPathChanged(
            temp.path()
                .join("non_existent_dir_xyz")
                .to_string_lossy()
                .to_string(),
        ));
        let _ = app.update(Message::AddProjectToRegistry);
        assert!(app
            .status_banner
            .as_deref()
            .unwrap_or_default()
            .contains("does not exist"));

        // Create second project directory with a sample Rust crate and add with initial ingestion
        let second_proj_dir = TempDir::new().expect("second project temp dir");
        let crate_dir = second_proj_dir
            .path()
            .join("crates")
            .join("novos_kernel")
            .join("src");
        fs::create_dir_all(&crate_dir).unwrap();
        fs::write(
            second_proj_dir
                .path()
                .join("crates")
                .join("novos_kernel")
                .join("Cargo.toml"),
            "[package]\nname = \"novos_kernel\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(crate_dir.join("lib.rs"), "pub fn boot() {}\n").unwrap();

        let _ = app.update(Message::ToggleNewProjectIngest(true));
        let _ = app.update(Message::NewProjectPathChanged(
            second_proj_dir.path().to_string_lossy().to_string(),
        ));
        let _ = app.update(Message::NewProjectNameChanged("NovOS".to_string()));
        let _ = app.update(Message::AddProjectToRegistry);

        assert_eq!(app.project_name(), "NovOS");
        assert_eq!(app.project_registry.projects.len(), 2);
        assert_eq!(app.selected_project_dropdown_index(), Some(1));
        assert!(!app.index.project.components.is_empty());

        // Switch back to first project via top-bar dropdown index 0
        let _ = app.update(Message::SelectProjectIndex(0));
        assert_eq!(app.project_name(), "NebulousTest");
        assert_eq!(app.selected_project_dropdown_index(), Some(0));

        // Switch to NovOS (making it the currently active project) and remove NovOS while active:
        // must automatically switch back to the remaining project ('NebulousTest') and must NOT resurrect on reload_indexes()
        let _ = app.update(Message::SwitchProject("novos".to_string()));
        assert_eq!(app.project_name(), "NovOS");
        let _ = app.update(Message::RemoveProjectFromRegistry("novos".to_string()));
        assert_eq!(app.project_registry.projects.len(), 1);
        assert_eq!(app.project_name(), "NebulousTest");
        assert_eq!(app.selected_project_dropdown_index(), Some(0));
        app.reload_indexes()
            .expect("reload indexes after active project removal");
        assert_eq!(app.project_registry.projects.len(), 1);
        assert_eq!(app.project_name(), "NebulousTest");

        // Re-add NovOS, switch to NebulousTest so NovOS is inactive, and remove inactive NovOS
        let _ = app.update(Message::NewProjectPathChanged(
            second_proj_dir.path().to_string_lossy().to_string(),
        ));
        let _ = app.update(Message::NewProjectNameChanged("NovOS".to_string()));
        let _ = app.update(Message::AddProjectToRegistry);
        assert_eq!(app.project_registry.projects.len(), 2);
        let _ = app.update(Message::SelectProjectIndex(0));
        assert_eq!(app.project_name(), "NebulousTest");
        let _ = app.update(Message::RemoveProjectFromRegistry("novos".to_string()));
        assert_eq!(app.project_registry.projects.len(), 1);

        // Removing the last project must leave registry empty even after reload_indexes()
        let _ = app.update(Message::RemoveProjectFromRegistry(
            "nebuloustest".to_string(),
        ));
        assert!(app.project_registry.projects.is_empty());
        app.reload_indexes()
            .expect("reload indexes with empty registry");
        assert!(app.project_registry.projects.is_empty());
    }

    #[test]
    fn test_documented_git_commit_workflow_commit_policy_and_ssh_key_management() {
        let (temp, storage) = create_sample_tracker_workspace();
        // Initialize git repository in temp workspace and a local bare remote for Fetch/Pull/Push
        let _ = Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(temp.path())
            .output();
        let bare_remote = TempDir::new().expect("bare remote temp dir");
        let _ = Command::new("git")
            .args(["init", "--bare", "-b", "main"])
            .current_dir(bare_remote.path())
            .output();
        let _ = Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                &bare_remote.path().to_string_lossy(),
            ])
            .current_dir(temp.path())
            .output();

        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("src").join("lib.rs"), "pub fn v1() {}\n").unwrap();

        let mut app =
            NebulousApp::new_from_storage(Core::default(), storage.clone()).expect("boot app");

        // 1. Verify header bar Git status formatting with ahead/behind counts
        if let Some(ref mut gs) = app.git_status {
            gs.ahead_count = 3;
            gs.behind_count = 1;
        }
        let header_git_label = header_bar::format_git_status_label(&app);
        assert!(header_git_label.contains("↑3 ↓1"));

        // 2. Configure [git.commit_policy] via Settings messages
        let _ = app.update(Message::ToggleCommitPolicyRequireItemId(true));
        let _ = app.update(Message::ToggleCommitPolicyRequireScope(true));
        let _ = app.update(Message::ToggleCommitPolicyAutoAppendDesc(true));
        let _ = app.update(Message::CommitPolicyMinSummaryLengthChanged(
            "15".to_string(),
        ));
        let _ = app.update(Message::SaveGitConfig);
        assert!(app.index.project.git.commit_policy.require_item_id);
        assert!(app.index.project.git.commit_policy.require_scope);
        assert!(app.index.project.git.commit_policy.auto_append_to_desc);
        assert_eq!(app.index.project.git.commit_policy.min_summary_length, 15);

        // 3. Undocumented workspace code commit (missing item_id) is rejected by policy
        let _ = app.update(Message::SetCommitTrackerOnly(false));
        let _ = app.update(Message::CommitItemIdChanged(String::new()));
        let _ = app.update(Message::CommitSummaryChanged(
            "Valid length summary without item ID".to_string(),
        ));
        let _ = app.update(Message::SubmitDocumentedCommit);
        assert!(app
            .status_banner
            .as_deref()
            .unwrap_or_default()
            .contains("Documented commit rejected:"));

        // 4. Short summary (< 15 chars) is rejected by policy
        let _ = app.update(Message::CommitItemIdChanged("BUG-0001".to_string()));
        let _ = app.update(Message::CommitSummaryChanged("Too short".to_string()));
        let _ = app.update(Message::SubmitDocumentedCommit);
        assert!(app
            .status_banner
            .as_deref()
            .unwrap_or_default()
            .contains("Documented commit rejected:"));

        // 5. Valid documented commit stages workspace + .tracker/, records fix in BUG-0001 item.toml & desc.md
        let _ = app.update(Message::CommitItemIdChanged("BUG-0001".to_string()));
        let _ = app.update(Message::SetCommitTrackerOnly(false));
        let _ = app.update(Message::CommitSummaryChanged(
            "Resolve TOML empty array deserialization edge case".to_string(),
        ));
        let _ = app.update(Message::CommitBodyChanged(
            "Added regression unit test and verified git_commit_documented.".to_string(),
        ));
        let _ = app.update(Message::SubmitDocumentedCommit);
        assert!(app
            .status_banner
            .as_deref()
            .unwrap_or_default()
            .contains("Committed"));

        let (_, bug1) = storage.load_item_detail_by_id("BUG-0001").unwrap();
        assert!(bug1.manifest.history.previous_fixes.iter().any(|f| f
            .summary
            .contains("Resolve TOML empty array deserialization edge case")));
        assert!(bug1
            .description_markdown
            .contains("## Fix Log & Agent Notes"));
        assert!(bug1
            .description_markdown
            .contains("Resolve TOML empty array deserialization edge case"));

        // 6. Exercise GitPush, GitFetch, and GitPull against local bare remote
        let _ = app.update(Message::GitPush);
        assert!(app
            .status_banner
            .as_deref()
            .unwrap_or_default()
            .contains("Pushed"));
        let _ = app.update(Message::GitFetch);
        assert!(app
            .status_banner
            .as_deref()
            .unwrap_or_default()
            .contains("Fetched remote tracking branch"));
        let _ = app.update(Message::GitPull);
        assert!(app
            .status_banner
            .as_deref()
            .unwrap_or_default()
            .contains("Git pull complete"));

        // 7. Exercise SecureStorage SSH Ed25519 key generation, public key copy, import, and deletion
        let ssh_cfg_dir = temp.path().join("nebulous_ssh_vault");
        std::env::set_var("NEBULOUS_CONFIG_DIR", &ssh_cfg_dir);
        std::env::set_var("NEBULOUS_DISABLE_KEYRING", "1");

        let _ = app.update(Message::SshKeyCommentChanged(
            "test@nebulous.local".to_string(),
        ));
        let _ = app.update(Message::GenerateSshKeypair);
        assert!(app.settings_form.has_stored_ssh_key);
        let generated_fp = app
            .settings_form
            .stored_ssh_public_key
            .as_ref()
            .unwrap()
            .fingerprint_sha256
            .clone();
        assert!(app
            .settings_form
            .stored_ssh_public_key
            .as_ref()
            .unwrap()
            .public_key_openssh
            .starts_with("ssh-ed25519 "));

        let _ = app.update(Message::CopySshPublicKey);
        assert!(app
            .last_copied_clipboard
            .as_deref()
            .unwrap_or_default()
            .starts_with("ssh-ed25519 "));

        // Export generated keypair PEM so we can test single-line flattened PEM import
        let exported_record = storage.get_ssh_keypair().expect("read generated keypair");
        let flattened_single_line_pem = exported_record
            .private_key_openssh
            .lines()
            .collect::<Vec<_>>()
            .join(" ");

        // Empty SSH private key import fails validation gracefully
        let _ = app.update(Message::SshKeyImportInputChanged("   ".to_string()));
        let _ = app.update(Message::ImportSshKeypair);
        assert!(app
            .status_banner
            .as_deref()
            .unwrap_or_default()
            .contains("OpenSSH PEM"));

        // Delete keypair and re-import from single-line flattened PEM string
        let _ = app.update(Message::DeleteSshKeypair);
        assert!(!app.settings_form.has_stored_ssh_key);
        assert!(app.settings_form.stored_ssh_public_key.is_none());

        let _ = app.update(Message::SshKeyImportInputChanged(flattened_single_line_pem));
        let _ = app.update(Message::ImportSshKeypair);
        assert!(app.settings_form.has_stored_ssh_key);
        assert_eq!(
            app.settings_form
                .stored_ssh_public_key
                .as_ref()
                .unwrap()
                .fingerprint_sha256,
            generated_fp
        );

        // Test importing a passphrase-encrypted OpenSSH Ed25519 key file from disk
        let enc_key_path = temp.path().join("id_ed25519_encrypted_test");
        let keygen_ok = Command::new("ssh-keygen")
            .args([
                "-q",
                "-t",
                "ed25519",
                "-N",
                "nebulous-passphrase-42",
                "-C",
                "enc@nebulous.local",
                "-f",
                &enc_key_path.to_string_lossy(),
            ])
            .status()
            .is_ok_and(|s| s.success());
        if keygen_ok {
            // Importing encrypted key without passphrase must fail with clear message
            let _ = app.update(Message::SshKeyImportInputChanged(
                enc_key_path.to_string_lossy().to_string(),
            ));
            let _ = app.update(Message::SshKeyPassphraseChanged(String::new()));
            let _ = app.update(Message::ImportSshKeypair);
            assert!(app
                .status_banner
                .as_deref()
                .unwrap_or_default()
                .contains("passphrase is required"));

            // Importing encrypted key with valid passphrase succeeds
            let _ = app.update(Message::SshKeyPassphraseChanged(
                "nebulous-passphrase-42".to_string(),
            ));
            let _ = app.update(Message::ImportSshKeypair);
            assert!(app.settings_form.has_stored_ssh_key);
            assert_eq!(
                app.settings_form
                    .stored_ssh_public_key
                    .as_ref()
                    .and_then(|k| k.comment.as_deref()),
                Some("enc@nebulous.local")
            );
        }
    }

    #[test]
    fn test_active_project_isolation_startup_resolution_and_subpath_boundary_guards() {
        let (temp, storage) = create_sample_tracker_workspace();
        let reg_path = temp.path().join("isolated_registry").join("projects.toml");

        // 1. When CWD has no `.tracker/` and registry is empty, `resolve_or_init_storage_from_cwd_with_registry`
        // must return `Err(TrackerRootNotFound)` and must NOT create `.tracker/` in CWD.
        let untracked_cwd = TempDir::new().expect("untracked cwd");
        let err = NebulousApp::resolve_or_init_storage_from_cwd_with_registry(
            None,
            untracked_cwd.path(),
            &reg_path,
        )
        .expect_err("untracked cwd with empty registry must not auto-init .tracker");
        assert!(matches!(err, TrackerError::TrackerRootNotFound(_)));
        assert!(!untracked_cwd.path().join(".tracker").exists());

        // 2. Registering the sample workspace in `projects.toml` allows startup from an untracked CWD
        // (or `$HOME`) to load the active project from `projects.toml` without creating `.tracker/` in CWD.
        let mut app = NebulousApp::new_from_storage_with_registry(
            Core::default(),
            storage.clone(),
            reg_path.clone(),
        )
        .expect("boot app");

        let resolved_from_registry = NebulousApp::resolve_or_init_storage_from_cwd_with_registry(
            None,
            untracked_cwd.path(),
            &reg_path,
        )
        .expect("resolve active project from registry when cwd is untracked");
        assert_eq!(
            resolved_from_registry
                .project_root()
                .canonicalize()
                .unwrap(),
            storage.project_root().canonicalize().unwrap()
        );
        assert!(!untracked_cwd.path().join(".tracker").exists());

        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            let resolved_from_home =
                NebulousApp::resolve_or_init_storage_from_cwd_with_registry(None, &home, &reg_path)
                    .expect("resolve active project from registry when cwd is $HOME");
            assert_eq!(
                resolved_from_home.project_root().canonicalize().unwrap(),
                storage.project_root().canonicalize().unwrap()
            );
            assert!(NebulousApp::resolve_or_init_storage(Some(&home)).is_err());
        }
        assert!(NebulousApp::resolve_or_init_storage(Some(Path::new("/"))).is_err());

        // 3. Seed two crates inside the active project (`crates/alpha` and `crates/beta`) and a sibling
        // directory outside the active project (`sibling_outside/crates/gamma`).
        let alpha_src = temp.path().join("crates").join("alpha").join("src");
        let beta_src = temp.path().join("crates").join("beta").join("src");
        fs::create_dir_all(&alpha_src).unwrap();
        fs::create_dir_all(&beta_src).unwrap();
        fs::write(
            temp.path().join("crates").join("alpha").join("Cargo.toml"),
            "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(alpha_src.join("lib.rs"), "pub fn alpha() {}\n").unwrap();
        fs::write(
            temp.path().join("crates").join("beta").join("Cargo.toml"),
            "[package]\nname = \"beta\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(beta_src.join("lib.rs"), "pub fn beta() {}\n").unwrap();

        let sibling_outside = TempDir::new().expect("sibling outside dir");
        let gamma_src = sibling_outside
            .path()
            .join("crates")
            .join("gamma")
            .join("src");
        fs::create_dir_all(&gamma_src).unwrap();
        fs::write(
            sibling_outside
                .path()
                .join("crates")
                .join("gamma")
                .join("Cargo.toml"),
            "[package]\nname = \"gamma\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(gamma_src.join("lib.rs"), "pub fn gamma() {}\n").unwrap();

        // 4. Attempting to scan an absolute path outside the active project is rejected with banner.
        let _ = app.update(Message::IngestSubpathChanged(
            sibling_outside.path().to_string_lossy().to_string(),
        ));
        let _ = app.update(Message::IngestProjectDirectory);
        assert!(app
            .status_banner
            .as_deref()
            .unwrap_or_default()
            .contains("Scan path must be within the active project"));
        assert!(app.index.project.components.is_empty());

        // 5. Attempting to scan a `../` relative path escaping the active project is rejected.
        let _ = app.update(Message::IngestSubpathChanged("../outside".to_string()));
        let _ = app.update(Message::IngestProjectDirectory);
        assert!(app
            .status_banner
            .as_deref()
            .unwrap_or_default()
            .contains("Scan path must be within the active project"));
        assert!(app.index.project.components.is_empty());

        // 6. Scanning an absolute path inside the active project (`<project_root>/crates/alpha`)
        // strips the project root prefix and scans strictly `crates/alpha` without indexing `beta`.
        let alpha_abs = temp
            .path()
            .join("crates")
            .join("alpha")
            .to_string_lossy()
            .to_string();
        let _ = app.update(Message::IngestSubpathChanged(alpha_abs));
        let _ = app.update(Message::IngestProjectDirectory);
        assert!(app.index.project.components.find("alpha").is_some());
        assert!(app.index.project.components.find("beta").is_none());
        assert!(app.index.project.components.find("gamma").is_none());
    }
}
