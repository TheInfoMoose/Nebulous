use std::collections::{BTreeMap, HashMap};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{Result, TrackerError};
use crate::git_sync::{DocumentedCommitInput, GitAuthRequest};
use crate::ingestion::IngestionOptions;
use crate::models::{
    load_global_registry, load_global_registry_from, register_project, register_project_in,
    set_active_project, set_active_project_in, ComponentDependency, ComponentManifest,
    ComponentType, GlobalProjectRegistry, ItemStatus, ItemType, PinnedReleaseConfig,
    RegisteredProject, RollingUpdateEntry, Severity, VerificationRecord,
};
use crate::storage::{normalize_relative_path_str, ItemLocation, TrackerStorage};
use crate::workflow::CreateItemInput;

/// Default MCP protocol version advertised by the Nebulous stdio server.
pub const DEFAULT_MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Standard JSON-RPC 2.0 request envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    #[serde(default = "default_jsonrpc_version")]
    pub jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// Standard JSON-RPC 2.0 response envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Constructs a successful JSON-RPC 2.0 response.
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: default_jsonrpc_version(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Constructs an error JSON-RPC 2.0 response.
    pub fn error(id: Value, code: i64, message: impl Into<String>, data: Option<Value>) -> Self {
        Self {
            jsonrpc: default_jsonrpc_version(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data,
            }),
        }
    }
}

/// Standard JSON-RPC 2.0 error payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Tool schema definition advertised by `tools/list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Domain scope boundary entry returned by `list_crate_boundaries`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateBoundaryEntry {
    pub id: String,
    pub scope_id: String,
    pub scope_type: String,
    pub target: String,
    pub target_path: String,
    pub tech_stack: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bound_component: Option<String>,
    pub tracked_files: Vec<String>,
    pub bugs_count: usize,
    pub features_count: usize,
    pub total_items: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned_release: Option<PinnedReleaseConfig>,
}

/// Stateful Model Context Protocol (MCP) JSON-RPC 2.0 server for Nebulous.
#[derive(Debug, Clone)]
pub struct McpServer {
    storage: Option<TrackerStorage>,
    registry_path_override: Option<PathBuf>,
}

impl McpServer {
    /// Creates an [`McpServer`] with an optional pre-resolved [`TrackerStorage`].
    pub fn new(storage: Option<TrackerStorage>) -> Self {
        Self {
            storage,
            registry_path_override: None,
        }
    }

    /// Creates an [`McpServer`] bound to an explicit [`TrackerStorage`].
    pub fn from_storage(storage: TrackerStorage) -> Self {
        Self::new(Some(storage))
    }

    /// Discovers a `.tracker/` workspace from `root` (or the current directory, or the active
    /// project in `projects.toml`) and constructs an [`McpServer`].
    pub fn discover(root: Option<&Path>) -> Self {
        let storage = match root {
            Some(explicit) => TrackerStorage::discover(explicit).ok(),
            None => std::env::current_dir()
                .ok()
                .and_then(|cwd| TrackerStorage::discover(cwd).ok())
                .or_else(|| {
                    load_global_registry()
                        .ok()
                        .and_then(|reg| reg.active_project().cloned())
                        .and_then(|proj| TrackerStorage::discover(&proj.root_path).ok())
                }),
        };
        Self::new(storage)
    }

    /// Overrides the global `projects.toml` path for isolated testing.
    pub fn with_registry_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.registry_path_override = Some(path.into());
        self
    }

    /// Returns a reference to the active [`TrackerStorage`], attempting fallback discovery from
    /// the global project registry if none was bound at startup.
    pub fn resolve_active_storage(&mut self) -> Result<TrackerStorage> {
        if let Some(ref storage) = self.storage {
            return Ok(storage.clone());
        }
        let registry = self.load_registry()?;
        if let Some(active) = registry.active_project() {
            if let Ok(discovered) = TrackerStorage::discover(&active.root_path) {
                self.storage = Some(discovered.clone());
                return Ok(discovered);
            }
        }
        if let Ok(cwd) = std::env::current_dir() {
            if let Ok(discovered) = TrackerStorage::discover(&cwd) {
                self.storage = Some(discovered.clone());
                return Ok(discovered);
            }
        }
        Err(TrackerError::TrackerRootNotFound(PathBuf::from(".")))
    }

    fn load_registry(&self) -> Result<GlobalProjectRegistry> {
        match &self.registry_path_override {
            Some(path) => load_global_registry_from(path),
            None => load_global_registry(),
        }
    }

    fn register_in_registry(
        &self,
        root_or_tracker_path: &Path,
        name_override: Option<&str>,
        tags: Vec<String>,
        set_active: bool,
    ) -> Result<RegisteredProject> {
        match &self.registry_path_override {
            Some(path) => {
                register_project_in(path, root_or_tracker_path, name_override, tags, set_active)
            }
            None => register_project(root_or_tracker_path, name_override, tags, set_active),
        }
    }

    fn switch_in_registry(&self, id_or_name: &str) -> Result<RegisteredProject> {
        match &self.registry_path_override {
            Some(path) => set_active_project_in(path, id_or_name),
            None => set_active_project(id_or_name),
        }
    }

    /// Runs the blocking JSON-RPC 2.0 loop over standard input (`stdin`) and standard output (`stdout`).
    pub fn serve_stdio(&mut self) -> Result<()> {
        // Startup hygiene: remove plaintext key exports left by older releases.
        let _ = crate::crypto::purge_legacy_ssh_cache(crate::crypto::nebulous_config_dir());
        let stdin = io::stdin();
        let stdout = io::stdout();
        let reader = stdin.lock();
        let writer = stdout.lock();
        self.serve_io(reader, writer)
    }

    /// Runs the JSON-RPC 2.0 loop over arbitrary buffered reader and writer streams.
    pub fn serve_io<R: BufRead, W: Write>(&mut self, mut reader: R, mut writer: W) -> Result<()> {
        while let Some((raw_message, used_content_length)) = read_next_jsonrpc_message(&mut reader)
            .map_err(|source| TrackerError::Io {
                path: PathBuf::from("<stdio>"),
                source,
            })?
        {
            if let Some(response_str) = self.handle_raw_message(&raw_message) {
                if used_content_length {
                    write!(
                        writer,
                        "Content-Length: {}\r\n\r\n{}",
                        response_str.len(),
                        response_str
                    )
                    .map_err(|source| TrackerError::Io {
                        path: PathBuf::from("<stdio>"),
                        source,
                    })?;
                } else {
                    writeln!(writer, "{response_str}").map_err(|source| TrackerError::Io {
                        path: PathBuf::from("<stdio>"),
                        source,
                    })?;
                }
                writer.flush().map_err(|source| TrackerError::Io {
                    path: PathBuf::from("<stdio>"),
                    source,
                })?;
            }
        }
        Ok(())
    }

    /// Parses and handles a single raw JSON-RPC 2.0 message string (single request or batch),
    /// returning a serialized JSON response string if the message requires a reply.
    pub fn handle_raw_message(&mut self, raw_json: &str) -> Option<String> {
        let trimmed = raw_json.trim();
        if trimmed.is_empty() {
            return None;
        }

        let parsed: Value = match serde_json::from_str(trimmed) {
            Ok(val) => val,
            Err(err) => {
                let resp = JsonRpcResponse::error(
                    Value::Null,
                    -32700,
                    format!("Parse error: {err}"),
                    None,
                );
                return serde_json::to_string(&resp).ok();
            }
        };

        if let Value::Array(batch) = parsed {
            if batch.is_empty() {
                let resp = JsonRpcResponse::error(
                    Value::Null,
                    -32600,
                    "Invalid Request: empty batch",
                    None,
                );
                return serde_json::to_string(&resp).ok();
            }
            let mut responses = Vec::new();
            for item in batch {
                match serde_json::from_value::<JsonRpcRequest>(item) {
                    Ok(req) => {
                        if let Some(resp) = self.handle_request(req) {
                            responses.push(resp);
                        }
                    }
                    Err(err) => {
                        responses.push(JsonRpcResponse::error(
                            Value::Null,
                            -32600,
                            format!("Invalid Request: {err}"),
                            None,
                        ));
                    }
                }
            }
            if responses.is_empty() {
                return None;
            }
            return serde_json::to_string(&responses).ok();
        }

        let request: JsonRpcRequest = match serde_json::from_value(parsed) {
            Ok(req) => req,
            Err(err) => {
                let resp = JsonRpcResponse::error(
                    Value::Null,
                    -32600,
                    format!("Invalid Request: {err}"),
                    None,
                );
                return serde_json::to_string(&resp).ok();
            }
        };

        let response = self.handle_request(request)?;
        serde_json::to_string(&response).ok()
    }

    /// Dispatches a parsed [`JsonRpcRequest`] and returns a [`JsonRpcResponse`] (or `None` for notifications).
    pub fn handle_request(&mut self, request: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let method = request.method.trim();

        // Standard MCP notifications do not expect a response frame when `id` is omitted
        if request.id.is_none()
            && (method.starts_with("notifications/")
                || method == "initialized"
                || method == "cancelled")
        {
            return None;
        }

        let req_id = request.id.clone().unwrap_or(Value::Null);
        let params = request.params.unwrap_or_else(|| json!({}));

        match method {
            "initialize" => {
                let requested_version = params
                    .get("protocolVersion")
                    .and_then(Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or(DEFAULT_MCP_PROTOCOL_VERSION);

                Some(JsonRpcResponse::success(
                    req_id,
                    json!({
                        "protocolVersion": requested_version,
                        "capabilities": {
                            "tools": {
                                "listChanged": false
                            },
                            "resources": {
                                "subscribe": false,
                                "listChanged": false
                            }
                        },
                        "serverInfo": {
                            "name": "nebulous-mcp",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    }),
                ))
            }
            "notifications/initialized" | "initialized" => {
                if request.id.is_some() {
                    Some(JsonRpcResponse::success(req_id, json!({})))
                } else {
                    None
                }
            }
            "ping" => Some(JsonRpcResponse::success(req_id, json!({}))),
            "tools/list" => Some(JsonRpcResponse::success(
                req_id,
                json!({
                    "tools": Self::tools_list()
                }),
            )),
            "tools/call" => {
                let Some(tool_name) = params.get("name").and_then(Value::as_str) else {
                    return Some(JsonRpcResponse::error(
                        req_id,
                        -32602,
                        "Invalid params: missing required 'name' string in tools/call",
                        None,
                    ));
                };
                if !Self::is_known_tool(tool_name) {
                    return Some(JsonRpcResponse::error(
                        req_id,
                        -32602,
                        format!("Unknown tool: '{tool_name}'"),
                        None,
                    ));
                }

                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));

                match self.call_tool(tool_name, arguments) {
                    Ok(payload) => {
                        let text = serde_json::to_string_pretty(&payload)
                            .unwrap_or_else(|_| payload.to_string());
                        let mut result_obj = serde_json::Map::new();
                        if let Value::Object(ref map) = payload {
                            for (k, v) in map {
                                result_obj.insert(k.clone(), v.clone());
                            }
                        }
                        result_obj.insert(
                            "content".to_string(),
                            json!([
                                {
                                    "type": "text",
                                    "text": text
                                }
                            ]),
                        );
                        result_obj.insert("structuredContent".to_string(), payload);
                        result_obj.insert("isError".to_string(), Value::Bool(false));
                        Some(JsonRpcResponse::success(req_id, Value::Object(result_obj)))
                    }
                    Err(err) => {
                        let err_msg = err.to_string();
                        let err_payload = json!({
                            "status": "error",
                            "error": err_msg,
                        });
                        let text = serde_json::to_string_pretty(&err_payload)
                            .unwrap_or_else(|_| err_payload.to_string());
                        Some(JsonRpcResponse::success(
                            req_id,
                            json!({
                                "status": "error",
                                "error": err.to_string(),
                                "content": [
                                    {
                                        "type": "text",
                                        "text": text
                                    }
                                ],
                                "structuredContent": err_payload,
                                "isError": true
                            }),
                        ))
                    }
                }
            }
            "resources/list" => match self.handle_resources_list() {
                Ok(val) => Some(JsonRpcResponse::success(req_id, val)),
                Err(err) => Some(JsonRpcResponse::error(
                    req_id,
                    -32603,
                    err.to_string(),
                    None,
                )),
            },
            "resources/read" => {
                let Some(uri) = params.get("uri").and_then(Value::as_str) else {
                    return Some(JsonRpcResponse::error(
                        req_id,
                        -32602,
                        "Invalid params: missing required 'uri' string in resources/read",
                        None,
                    ));
                };
                match self.handle_resources_read(uri) {
                    Ok(val) => Some(JsonRpcResponse::success(req_id, val)),
                    Err(err) => Some(JsonRpcResponse::error(
                        req_id,
                        -32602,
                        err.to_string(),
                        None,
                    )),
                }
            }
            other => Some(JsonRpcResponse::error(
                req_id,
                -32601,
                format!("Method not found: '{other}'"),
                None,
            )),
        }
    }

    fn handle_resources_list(&mut self) -> Result<Value> {
        let mut resources = vec![json!({
            "uri": "nebulous://projects",
            "name": "Global Nebulous Project Registry",
            "description": "Cross-platform global registry of tracked Nebulous projects (projects.toml)",
            "mimeType": "application/json"
        })];

        if let Ok(storage) = self.resolve_active_storage() {
            if let Ok(index) = storage.load_index() {
                resources.push(json!({
                    "uri": "nebulous://project",
                    "name": format!("Project Manifest ({})", index.project.project.name),
                    "description": "Tier 1 .tracker/project.toml manifest and component registry",
                    "mimeType": "application/json"
                }));
                resources.push(json!({
                    "uri": "nebulous://components",
                    "name": "Ingested Project Components",
                    "description": "Registered components and rolling-release pins in .tracker/project.toml",
                    "mimeType": "application/json"
                }));
                resources.push(json!({
                    "uri": "nebulous://metrics",
                    "name": "Project Metrics & Valuation Report",
                    "description": "Effort valuation, stagnant items, and recurring bug hotspots",
                    "mimeType": "application/json"
                }));
                for item in index.all_items() {
                    resources.push(json!({
                        "uri": format!("nebulous://items/{}", item.stub.id),
                        "name": format!("{} — {}", item.stub.id, item.stub.title),
                        "description": format!("Bounded Agent Context for {} in {}/{}", item.stub.id, item.phase_id, item.scope_id),
                        "mimeType": "application/json"
                    }));
                }
            }
        }

        Ok(json!({ "resources": resources }))
    }

    fn handle_resources_read(&mut self, uri: &str) -> Result<Value> {
        let trimmed = uri.trim();
        let payload = if trimmed == "nebulous://projects" {
            let reg = self.load_registry()?;
            serde_json::to_value(reg).map_err(|e| TrackerError::ValidationError(e.to_string()))?
        } else if trimmed == "nebulous://project" {
            let storage = self.resolve_active_storage()?;
            let proj = storage.read_project()?;
            serde_json::to_value(proj).map_err(|e| TrackerError::ValidationError(e.to_string()))?
        } else if trimmed == "nebulous://components" {
            let storage = self.resolve_active_storage()?;
            let proj = storage.read_project()?;
            serde_json::to_value(proj.components.registry)
                .map_err(|e| TrackerError::ValidationError(e.to_string()))?
        } else if trimmed == "nebulous://metrics" {
            let storage = self.resolve_active_storage()?;
            let metrics = storage.compute_metrics()?;
            serde_json::to_value(metrics)
                .map_err(|e| TrackerError::ValidationError(e.to_string()))?
        } else if let Some(item_id) = trimmed.strip_prefix("nebulous://items/") {
            self.tool_get_item_context(json!({ "item_id": item_id }))?
        } else {
            return Err(TrackerError::NotFound {
                kind: "Resource",
                id: trimmed.to_string(),
            });
        };

        let text = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string());
        Ok(json!({
            "contents": [
                {
                    "uri": trimmed,
                    "mimeType": "application/json",
                    "text": text
                }
            ]
        }))
    }

    /// Returns `true` if `name` is a supported MCP tool.
    pub fn is_known_tool(name: &str) -> bool {
        matches!(
            name.trim(),
            "get_active_task"
                | "list_items"
                | "get_item_context"
                | "list_crate_boundaries"
                | "create_item"
                | "update_item_status"
                | "close_item_with_fix"
                | "record_regression"
                | "list_projects"
                | "switch_project"
                | "ingest_project_directory"
                | "upsert_component_manifest"
                | "promote_component_to_scope"
                | "record_rolling_update"
                | "record_verification_run"
                | "pin_stable_release"
                | "rollback_to_pinned_release"
                | "git_status"
                | "git_fetch"
                | "git_pull"
                | "git_push"
                | "git_commit_documented"
                | "git_sync"
        )
    }

    /// Returns the complete catalog of MCP tools across all 4 tool suites.
    pub fn tools_list() -> Vec<McpToolDefinition> {
        vec![
            // Suite 1: Core Agentic Task & Boundary Tools
            McpToolDefinition {
                name: "get_active_task".to_string(),
                description: "Returns the active task (from the running timer, or the highest-priority in_progress/open item in the active phase) along with its strict scope boundary and Bounded Agent Prompt.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "item_id": { "type": "string", "description": "Optional explicit item ID override (e.g. BUG-0001)." },
                        "phase_id": { "type": "string", "description": "Optional phase ID filter." },
                        "scope_id": { "type": "string", "description": "Optional domain scope ID filter." }
                    }
                }),
            },
            McpToolDefinition {
                name: "list_items".to_string(),
                description: "Lists tracked bugs and features with optional phase, scope, status, type, stagnant, and recurring filters.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "phase_id": { "type": "string", "description": "Optional phase ID filter." },
                        "scope_id": { "type": "string", "description": "Optional domain scope ID filter." },
                        "status": { "type": "string", "description": "Optional status filter (open, in_progress, pending, closed, merged)." },
                        "item_type": { "type": "string", "description": "Optional item type filter (bug, feature)." },
                        "stagnant": { "type": "boolean", "description": "When true, filters to stagnant non-completed items." },
                        "recurring": { "type": "boolean", "description": "When true, filters to items with recurrence_count > 0." }
                    }
                }),
            },
            McpToolDefinition {
                name: "get_item_context".to_string(),
                description: "Exports the Bounded Agent Prompt, @file references, fix history ledger, and scope boundary metadata for an item.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "item_id": { "type": "string", "description": "Item ID (e.g. BUG-0001 or FEAT-0001)." },
                        "refs_only": { "type": "boolean", "description": "When true, sets output to compact @file references only." }
                    },
                    "required": ["item_id"]
                }),
            },
            McpToolDefinition {
                name: "list_crate_boundaries".to_string(),
                description: "Lists all domain scopes in the target/active phase and all ingested components, including target paths, tracked files, tech stacks, and pinned release configurations.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "phase_id": { "type": "string", "description": "Optional phase ID (defaults to project.active_phase)." }
                    }
                }),
            },
            McpToolDefinition {
                name: "create_item".to_string(),
                description: "Creates a new bug (BUG-XXXX) or feature (FEAT-XXXX) item in the specified or default phase and scope.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "item_type": { "type": "string", "description": "Item type: 'bug' or 'feature' (default 'bug')." },
                        "title": { "type": "string", "description": "Concise item summary title." },
                        "phase_id": { "type": "string", "description": "Optional target phase ID." },
                        "scope_id": { "type": "string", "description": "Optional target domain scope ID." },
                        "severity": { "type": "string", "description": "Severity: low, medium, high, critical (default 'medium')." },
                        "description": { "type": "string", "description": "Markdown problem description or feature specification." }
                    },
                    "required": ["title"]
                }),
            },
            McpToolDefinition {
                name: "update_item_status".to_string(),
                description: "Updates an item's workflow status (open, in_progress, pending, closed, merged) and atomically reconciles parent indexes.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "item_id": { "type": "string", "description": "Item ID (e.g. BUG-0001)." },
                        "status": { "type": "string", "description": "Target status (open, in_progress, pending, closed, merged)." }
                    },
                    "required": ["item_id", "status"]
                }),
            },
            McpToolDefinition {
                name: "close_item_with_fix".to_string(),
                description: "Closes or merges an item and records a fix entry (date, commit SHA, summary) in history.previous_fixes.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "item_id": { "type": "string", "description": "Item ID (e.g. BUG-0001)." },
                        "summary": { "type": "string", "description": "Summary of the implemented fix." },
                        "commit": { "type": "string", "description": "Optional commit short SHA (defaults to Git HEAD or 'uncommitted')." },
                        "merged": { "type": "boolean", "description": "When true, marks status as 'merged' instead of 'closed'." }
                    },
                    "required": ["item_id", "summary"]
                }),
            },
            McpToolDefinition {
                name: "record_regression".to_string(),
                description: "Records a regression on an item, reopening it if closed/merged, incrementing recurrence_count and iteration_count, and appending the failure reason to desc.md.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "item_id": { "type": "string", "description": "Item ID (e.g. BUG-0001)." },
                        "reason": { "type": "string", "description": "Reason or test failure summary for the regression." }
                    },
                    "required": ["item_id"]
                }),
            },
            // Suite 2: Project & Ingestion Tools
            McpToolDefinition {
                name: "list_projects".to_string(),
                description: "Lists all projects registered in the global Nebulous project registry (projects.toml) and identifies the active project.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpToolDefinition {
                name: "switch_project".to_string(),
                description: "Switches the active project in the global Nebulous registry by ID, name, or root path and binds the MCP server to its .tracker/ workspace.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id_or_name": { "type": "string", "description": "Project ID, name, or workspace path." }
                    },
                    "required": ["id_or_name"]
                }),
            },
            McpToolDefinition {
                name: "ingest_project_directory".to_string(),
                description: "Stage 1 Directory Ingestion: scans the project directory respecting .gitignore, discovers ecosystem manifests and un-manifested functional directories, and indexes component file lists into .tracker/project.toml.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "subpath": { "type": "string", "description": "Optional relative subdirectory to restrict scanning." },
                        "write_manifests": { "type": "boolean", "description": "When true, writes nebulous.component.toml files to component directories." },
                        "promote_all": { "type": "boolean", "description": "When true, promotes all discovered components into domain scopes." },
                        "promote_phase_id": { "type": "string", "description": "Optional target phase ID when promote_all is true." }
                    }
                }),
            },
            McpToolDefinition {
                name: "upsert_component_manifest".to_string(),
                description: "Creates or updates a nebulous.component.toml manifest for a directory/component and synchronizes it into .tracker/project.toml [components].".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Component identifier (e.g. orbit_image or backend_core)." },
                        "name": { "type": "string", "description": "Human-readable component name." },
                        "relative_path": { "type": "string", "description": "Workspace-relative directory path (e.g. 'iso' or 'crates/backend_core')." },
                        "component_type": { "type": "string", "description": "Component type (rust_crate, cli_crate, ui_crate, python_pkg, node_pkg, c_cpp, os_image, kernel_module, build_scripts, custom)." },
                        "description": { "type": "string", "description": "Optional component responsibility description." },
                        "tech_stack": { "type": "array", "items": { "type": "string" } },
                        "tags": { "type": "array", "items": { "type": "string" } },
                        "flags": { "type": "array", "items": { "type": "string" } },
                        "files": { "type": "array", "items": { "type": "string" } },
                        "dependencies": { "type": "array", "items": { "type": "object" } }
                    },
                    "required": ["id"]
                }),
            },
            McpToolDefinition {
                name: "promote_component_to_scope".to_string(),
                description: "Stage 2 Ingestion: promotes an ingested component (and optional selected file subset) into a Tier 3 domain scope (scope.toml) inside a phase.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "component_id": { "type": "string", "description": "Component ID or relative path to promote." },
                        "phase_id": { "type": "string", "description": "Optional target phase ID (defaults to project.active_phase)." },
                        "scope_id": { "type": "string", "description": "Optional custom scope ID override." },
                        "selected_files": { "type": "array", "items": { "type": "string" }, "description": "Optional subset of component files to track." }
                    },
                    "required": ["component_id"]
                }),
            },
            // Suite 3: Release Engineering & Self-Healing CI Tools
            McpToolDefinition {
                name: "record_rolling_update".to_string(),
                description: "Logs an upstream rolling dependency bump (e.g. a rolling distro package or custom kernel delta) and target build script path into nebulous.component.toml and creates/updates a tracking item.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "component_id": { "type": "string", "description": "Target component ID or relative path." },
                        "dependency_name": { "type": "string", "description": "Upstream package or dependency name (e.g. linux-image-rt or systemd)." },
                        "kind": { "type": "string", "description": "Dependency channel/kind (e.g. apt_sid, kernel_custom, cargo, git)." },
                        "previous_version": { "type": "string", "description": "Optional previous version before update." },
                        "new_version": { "type": "string", "description": "New upstream version string." },
                        "upstream_source": { "type": "string", "description": "Optional upstream repository or APT source URL." },
                        "build_script_path": { "type": "string", "description": "Optional relative path to the target build script (e.g. scripts/build_iso.sh)." },
                        "summary": { "type": "string", "description": "Optional summary notes for the update." },
                        "tracked_item_id": { "type": "string", "description": "Optional existing item ID to link instead of creating a new item." },
                        "create_tracking_item": { "type": "boolean", "description": "Whether to auto-create a tracking item when tracked_item_id is omitted (default true)." }
                    },
                    "required": ["component_id", "dependency_name", "new_version"]
                }),
            },
            McpToolDefinition {
                name: "record_verification_run".to_string(),
                description: "Records VM, container, or CI test-suite verification results onto a component's latest rolling update and pinned release.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "component_id": { "type": "string", "description": "Target component ID or relative path." },
                        "passed": { "type": "boolean", "description": "Whether the verification test suite passed." },
                        "exit_code": { "type": "integer", "description": "Process exit code (0 on success)." },
                        "command": { "type": "string", "description": "Executed verification command line." },
                        "summary": { "type": "string", "description": "Human-readable verification summary." },
                        "logs_excerpt": { "type": "string", "description": "Optional excerpt of boot/test failure logs." },
                        "item_id": { "type": "string", "description": "Optional item ID to append verification notes onto." }
                    },
                    "required": ["component_id", "passed"]
                }),
            },
            McpToolDefinition {
                name: "pin_stable_release".to_string(),
                description: "Snapshots the current Git commit SHA, dependency versions, build script path, and build flags into nebulous.component.toml and .tracker/project.toml as last_known_good.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "component_id": { "type": "string", "description": "Target component ID or relative path." },
                        "commit_sha": { "type": "string", "description": "Optional explicit Git commit SHA (defaults to HEAD)." },
                        "build_script_path": { "type": "string", "description": "Optional build script path." },
                        "build_flags": { "type": "array", "items": { "type": "string" }, "description": "Optional build flags to pin." },
                        "dependency_pins": { "type": "object", "description": "Optional map of dependency_name -> version overrides to pin." }
                    },
                    "required": ["component_id"]
                }),
            },
            McpToolDefinition {
                name: "rollback_to_pinned_release".to_string(),
                description: "Returns the last_known_good pin configuration (and optionally restores nebulous.component.toml dependency pins and flags) while recording a regression on the active/linked item for self-healing CI.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "component_id": { "type": "string", "description": "Target component ID or relative path." },
                        "restore_manifest": { "type": "boolean", "description": "Whether to restore nebulous.component.toml dependencies and flags from last_known_good (default true)." },
                        "item_id": { "type": "string", "description": "Optional active item ID to record the regression on." },
                        "reason": { "type": "string", "description": "Optional rollback/regression reason appended to desc.md." }
                    },
                    "required": ["component_id"]
                }),
            },
            // Suite 4: Git & Documentation Tools
            McpToolDefinition {
                name: "git_status".to_string(),
                description: "Returns the Git repository status including dirty workspace files, dirty .tracker/ files, branch, remote URL, and ahead/behind counts.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpToolDefinition {
                name: "git_fetch".to_string(),
                description: "Fetches the configured remote branch without modifying the working tree and returns updated ahead/behind commit counts.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "auth_override": { "type": "string", "description": "Optional vault credential account name; must be bound to this project in the local registry. PATs and key file paths are not accepted." }
                    }
                }),
            },
            McpToolDefinition {
                name: "git_pull".to_string(),
                description: "Pulls the current HEAD branch (fast-forward or merge with automatic .tracker/ index conflict resolution) from the configured remote and reconciles .tracker/ indexes. Errors if HEAD is not on the configured [git] branch; aborts without writing when local uncommitted changes would be overwritten. The result's `duplicate_ids` lists item IDs the post-pull reconcile found in more than one item directory (likely a merge from two clones).".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "auth_override": { "type": "string", "description": "Optional vault credential account name; must be bound to this project in the local registry. PATs and key file paths are not accepted." }
                    }
                }),
            },
            McpToolDefinition {
                name: "git_push".to_string(),
                description: "Pushes the current HEAD branch to the remote repository using SecureStorage PAT or SSH credentials. Errors if HEAD is not on the configured [git] branch.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "auth_override": { "type": "string", "description": "Optional vault credential account name; must be bound to this project in the local registry. PATs and key file paths are not accepted." }
                    }
                }),
            },
            McpToolDefinition {
                name: "git_commit_documented".to_string(),
                description: "Commits .tracker/ changes (default) or .tracker/ plus explicitly listed workspace paths, enforces [git.commit_policy], and atomically records the commit SHA and changed file list in item.toml and desc.md. Never stages unlisted or untracked workspace files and never modifies the user's other staged changes.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "item_id": { "type": "string", "description": "Linked item ID (e.g. BUG-0001 or FEAT-0001)." },
                        "scope_id": { "type": "string", "description": "Optional domain scope ID override." },
                        "summary": { "type": "string", "description": "Commit summary message." },
                        "body": { "type": "string", "description": "Optional extended commit notes." },
                        "tracker_only": { "type": "boolean", "default": true, "description": "When true (default), stages only .tracker/ files. Set false together with `paths` to commit workspace files." },
                        "paths": { "type": "array", "items": { "type": "string" }, "description": "Explicit project-relative files or directories to stage (required when tracker_only is false). Glob metacharacters, absolute paths, and '..' are rejected; untracked files are staged only when listed by exact path." }
                    },
                    "required": ["summary"]
                }),
            },
            McpToolDefinition {
                name: "git_sync".to_string(),
                description: "Stages and commits dirty .tracker/ state, then pulls and pushes if a remote is configured. The result's `duplicate_ids` lists item IDs the post-pull reconcile found in more than one item directory (empty with commit_only or when nothing was pulled).".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "message": { "type": "string", "description": "Optional commit message for .tracker/ state." },
                        "auth_override": { "type": "string", "description": "Optional vault credential account name; must be bound to this project in the local registry. PATs and key file paths are not accepted." },
                        "commit_only": { "type": "boolean", "description": "When true, commits .tracker/ locally without pushing or pulling." }
                    }
                }),
            },
        ]
    }

    /// Invokes a named MCP tool with the given JSON arguments and returns the structured JSON result.
    pub fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value> {
        match name.trim() {
            // Suite 1: Core Agentic Task & Boundary Tools
            "get_active_task" => self.tool_get_active_task(arguments),
            "list_items" => self.tool_list_items(arguments),
            "get_item_context" => self.tool_get_item_context(arguments),
            "list_crate_boundaries" => self.tool_list_crate_boundaries(arguments),
            "create_item" => self.tool_create_item(arguments),
            "update_item_status" => self.tool_update_item_status(arguments),
            "close_item_with_fix" => self.tool_close_item_with_fix(arguments),
            "record_regression" => self.tool_record_regression(arguments),
            // Suite 2: Project & Ingestion Tools
            "list_projects" => self.tool_list_projects(arguments),
            "switch_project" => self.tool_switch_project(arguments),
            "ingest_project_directory" => self.tool_ingest_project_directory(arguments),
            "upsert_component_manifest" => self.tool_upsert_component_manifest(arguments),
            "promote_component_to_scope" => self.tool_promote_component_to_scope(arguments),
            // Suite 3: Release Engineering & Self-Healing CI Tools
            "record_rolling_update" => self.tool_record_rolling_update(arguments),
            "record_verification_run" => self.tool_record_verification_run(arguments),
            "pin_stable_release" => self.tool_pin_stable_release(arguments),
            "rollback_to_pinned_release" => self.tool_rollback_to_pinned_release(arguments),
            // Suite 4: Git & Documentation Tools
            "git_status" => self.tool_git_status(arguments),
            "git_fetch" => self.tool_git_fetch(arguments),
            "git_pull" => self.tool_git_pull(arguments),
            "git_push" => self.tool_git_push(arguments),
            "git_commit_documented" => self.tool_git_commit_documented(arguments),
            "git_sync" => self.tool_git_sync(arguments),
            other => Err(TrackerError::ValidationError(format!(
                "Unknown MCP tool '{other}'"
            ))),
        }
    }

    fn tool_get_active_task(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Default, Deserialize)]
        struct Args {
            #[serde(default, alias = "id")]
            item_id: Option<String>,
            #[serde(default, alias = "phase")]
            phase_id: Option<String>,
            #[serde(default, alias = "scope")]
            scope_id: Option<String>,
        }

        let parsed: Args = serde_json::from_value(args).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid get_active_task args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let active_timer = storage.read_active_timer()?;

        let resolved_id = if let Some(explicit_id) = non_empty(parsed.item_id.as_deref()) {
            Some(explicit_id.to_string())
        } else if let Some(ref timer) = active_timer {
            if storage.locate_item(&timer.item_id).is_ok() {
                Some(timer.item_id.clone())
            } else {
                None
            }
        } else {
            None
        };

        let resolved_id = match resolved_id {
            Some(id) => Some(id),
            None => select_best_active_item(
                &storage,
                non_empty(parsed.phase_id.as_deref()),
                non_empty(parsed.scope_id.as_deref()),
            )?,
        };

        let Some(item_id) = resolved_id else {
            return Ok(json!({
                "active": false,
                "item_id": Value::Null,
                "id": Value::Null,
                "timer_active": false,
                "message": "No active or open task found in tracker."
            }));
        };

        let (loc, detail) = storage.load_item_detail_by_id(&item_id)?;
        let scope_manifest = storage.read_scope(&loc.phase_id, &loc.scope_id)?;
        let bundle = storage.export_agent_context(&item_id)?;
        let timer_active = active_timer
            .as_ref()
            .is_some_and(|t| t.item_id == detail.manifest.item.id);

        Ok(json!({
            "active": true,
            "id": detail.manifest.item.id,
            "item_id": detail.manifest.item.id,
            "title": detail.manifest.item.title,
            "item_type": detail.manifest.item.item_type,
            "status": detail.manifest.item.status,
            "severity": detail.manifest.item.severity,
            "phase_id": loc.phase_id,
            "scope_id": loc.scope_id,
            "target": scope_manifest.scope.target,
            "target_path": scope_manifest.scope.target,
            "bound_component": scope_manifest.scope.bound_component,
            "tracked_files": scope_manifest.scope.tracked_files,
            "tech_stack": scope_manifest.scope.tech_stack,
            "recurrence_count": detail.manifest.history.recurrence_count,
            "iteration_count": detail.manifest.effort.iteration_count,
            "logged_time_minutes": detail.manifest.effort.logged_time_minutes,
            "previous_fixes": detail.manifest.history.previous_fixes,
            "description_markdown": detail.description_markdown,
            "file_refs": bundle.file_refs,
            "bounded_prompt": bundle.bounded_prompt,
            "timer_active": timer_active
        }))
    }

    fn tool_list_items(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Default, Deserialize)]
        struct Args {
            #[serde(default, alias = "phase")]
            phase_id: Option<String>,
            #[serde(default, alias = "scope")]
            scope_id: Option<String>,
            #[serde(default)]
            status: Option<String>,
            #[serde(default, alias = "type", alias = "kind")]
            item_type: Option<String>,
            #[serde(default, alias = "stagnant_only")]
            stagnant: bool,
            #[serde(default, alias = "recurring_only")]
            recurring: bool,
        }

        let parsed: Args = serde_json::from_value(args)
            .map_err(|e| TrackerError::ValidationError(format!("Invalid list_items args: {e}")))?;
        let storage = self.resolve_active_storage()?;
        let index = storage.load_index()?;

        let status_filter = match non_empty(parsed.status.as_deref()) {
            Some(s) => Some(ItemStatus::from_str(s)?),
            None => None,
        };
        let type_filter = match non_empty(parsed.item_type.as_deref()) {
            Some(t) => Some(ItemType::from_str(t)?),
            None => None,
        };

        let stagnant_map: HashMap<String, i64> = if parsed.stagnant {
            storage
                .find_stagnant_items()?
                .into_iter()
                .map(|entry| (entry.item_id, entry.days_since_update))
                .collect()
        } else {
            HashMap::new()
        };

        let recurring_map: HashMap<String, u32> = if parsed.recurring {
            storage
                .find_recurring_hotspots()?
                .0
                .into_iter()
                .filter(|entry| entry.recurrence_count > 0)
                .map(|entry| (entry.item_id, entry.recurrence_count))
                .collect()
        } else {
            HashMap::new()
        };

        let phase_filter = non_empty(parsed.phase_id.as_deref());
        let scope_filter = non_empty(parsed.scope_id.as_deref());

        let mut items = Vec::new();
        for loc in index.all_items() {
            if let Some(p) = phase_filter {
                if loc.phase_id != p {
                    continue;
                }
            }
            if let Some(s) = scope_filter {
                if loc.scope_id != s {
                    continue;
                }
            }
            if let Some(st) = status_filter {
                if loc.stub.status != st {
                    continue;
                }
            }
            if let Some(kind) = type_filter {
                if loc.item_type != kind {
                    continue;
                }
            }
            if parsed.stagnant && !stagnant_map.contains_key(&loc.stub.id) {
                continue;
            }
            if parsed.recurring && !recurring_map.contains_key(&loc.stub.id) {
                continue;
            }

            items.push(json!({
                "id": loc.stub.id,
                "item_id": loc.stub.id,
                "type": loc.item_type,
                "title": loc.stub.title,
                "status": loc.stub.status,
                "severity": loc.stub.severity,
                "phase_id": loc.phase_id,
                "scope_id": loc.scope_id,
                "days_since_update": stagnant_map.get(&loc.stub.id).copied(),
                "recurrence_count": recurring_map.get(&loc.stub.id).copied()
            }));
        }

        Ok(json!({
            "project_name": index.project.project.name,
            "active_phase": index.project.project.active_phase,
            "total": items.len(),
            "items": items
        }))
    }

    fn tool_get_item_context(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Args {
            #[serde(alias = "id")]
            item_id: String,
            #[serde(default)]
            refs_only: bool,
        }

        let parsed: Args = serde_json::from_value(args).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid get_item_context args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let (loc, detail) = storage.load_item_detail_by_id(parsed.item_id.trim())?;
        let scope_manifest = storage.read_scope(&loc.phase_id, &loc.scope_id)?;
        let bundle = storage.export_agent_context(parsed.item_id.trim())?;
        let output = if parsed.refs_only {
            bundle.file_refs.clone()
        } else {
            bundle.bounded_prompt.clone()
        };

        Ok(json!({
            "id": bundle.item_id,
            "item_id": bundle.item_id,
            "phase_id": bundle.phase_id,
            "scope_id": bundle.scope_id,
            "scope_rel_path": bundle.scope_rel_path,
            "item_rel_path": bundle.item_rel_path,
            "desc_rel_path": bundle.desc_rel_path,
            "file_refs": bundle.file_refs,
            "bounded_prompt": bundle.bounded_prompt,
            "refs_only": parsed.refs_only,
            "output": output,
            "target": scope_manifest.scope.target,
            "target_path": scope_manifest.scope.target,
            "bound_component": scope_manifest.scope.bound_component,
            "tracked_files": scope_manifest.scope.tracked_files,
            "tech_stack": scope_manifest.scope.tech_stack,
            "item": detail.manifest.item,
            "effort": detail.manifest.effort,
            "history": detail.manifest.history,
            "description_markdown": detail.description_markdown
        }))
    }

    fn tool_list_crate_boundaries(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Default, Deserialize)]
        struct Args {
            #[serde(default, alias = "phase")]
            phase_id: Option<String>,
        }

        let parsed: Args = serde_json::from_value(args).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid list_crate_boundaries args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let index = storage.load_index()?;

        let resolved_phase = non_empty(parsed.phase_id.as_deref())
            .unwrap_or(&index.project.project.active_phase)
            .to_string();

        let phase_idx = index
            .phase(&resolved_phase)
            .ok_or_else(|| TrackerError::NotFound {
                kind: "Phase",
                id: resolved_phase.clone(),
            })?;

        let mut boundaries = Vec::with_capacity(phase_idx.scopes.len());
        for scope_idx in &phase_idx.scopes {
            let scope_type = phase_idx
                .manifest
                .scopes
                .manifest
                .iter()
                .find(|s| s.id == scope_idx.scope_id)
                .map(|s| s.scope_type.clone())
                .unwrap_or_else(|| "custom".to_string());

            let bound_comp = scope_idx
                .manifest
                .scope
                .bound_component
                .as_deref()
                .and_then(|cid| index.project.components.find(cid))
                .or_else(|| index.project.components.find(&scope_idx.scope_id))
                .or_else(|| {
                    index
                        .project
                        .components
                        .find(&scope_idx.manifest.scope.target)
                });

            let tracked_files = if !scope_idx.manifest.scope.tracked_files.is_empty() {
                scope_idx.manifest.scope.tracked_files.clone()
            } else if let Some(comp) = bound_comp {
                comp.files.clone()
            } else {
                Vec::new()
            };

            boundaries.push(CrateBoundaryEntry {
                id: scope_idx.scope_id.clone(),
                scope_id: scope_idx.scope_id.clone(),
                scope_type,
                target: scope_idx.manifest.scope.target.clone(),
                target_path: scope_idx.manifest.scope.target.clone(),
                tech_stack: scope_idx.manifest.scope.tech_stack.clone(),
                description: scope_idx.manifest.scope.description.clone(),
                bound_component: scope_idx
                    .manifest
                    .scope
                    .bound_component
                    .clone()
                    .or_else(|| bound_comp.map(|c| c.id.clone())),
                tracked_files,
                bugs_count: scope_idx.manifest.items.count(ItemType::Bug),
                features_count: scope_idx.manifest.items.count(ItemType::Feature),
                total_items: scope_idx.manifest.total_items(),
                pinned_release: bound_comp.and_then(|c| c.pinned_release.clone()),
            });
        }

        Ok(json!({
            "project_name": index.project.project.name,
            "active_phase": index.project.project.active_phase,
            "phase_id": resolved_phase,
            "total_scopes": boundaries.len(),
            "total_components": index.project.components.registry.len(),
            "boundaries": boundaries,
            "scopes": boundaries,
            "components": index.project.components.registry
        }))
    }

    fn tool_create_item(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Args {
            #[serde(default = "default_item_type_str", alias = "type", alias = "kind")]
            item_type: String,
            title: String,
            #[serde(default, alias = "phase")]
            phase_id: Option<String>,
            #[serde(default, alias = "scope")]
            scope_id: Option<String>,
            #[serde(default = "default_severity_str")]
            severity: String,
            #[serde(default, alias = "desc")]
            description: Option<String>,
        }

        let parsed: Args = serde_json::from_value(args)
            .map_err(|e| TrackerError::ValidationError(format!("Invalid create_item args: {e}")))?;
        let storage = self.resolve_active_storage()?;
        let item_type = ItemType::from_str(&parsed.item_type)?;
        let severity = Severity::from_str(&parsed.severity)?;

        let (loc, detail) = storage.create_item(CreateItemInput {
            item_type,
            title: parsed.title,
            phase_id: non_empty(parsed.phase_id.as_deref()).map(str::to_string),
            scope_id: non_empty(parsed.scope_id.as_deref()).map(str::to_string),
            severity,
            description: non_empty(parsed.description.as_deref()).map(str::to_string),
        })?;

        let item_toml = storage
            .item_toml_path(&loc.phase_id, &loc.scope_id, &loc.stub.id)?
            .to_string_lossy()
            .to_string();
        let desc_md = storage
            .item_desc_path(&loc.phase_id, &loc.scope_id, &loc.stub.id)?
            .to_string_lossy()
            .to_string();

        Ok(json!({
            "id": detail.manifest.item.id,
            "item_id": detail.manifest.item.id,
            "type": detail.manifest.item.item_type,
            "title": detail.manifest.item.title,
            "status": detail.manifest.item.status,
            "severity": detail.manifest.item.severity,
            "phase_id": loc.phase_id,
            "scope_id": loc.scope_id,
            "item_toml": item_toml,
            "desc_md": desc_md
        }))
    }

    fn tool_update_item_status(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Args {
            #[serde(alias = "id")]
            item_id: String,
            status: String,
        }

        let parsed: Args = serde_json::from_value(args).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid update_item_status args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let status = ItemStatus::from_str(&parsed.status)?;
        let loc = storage.locate_item(parsed.item_id.trim())?;
        let detail = storage.update_item_status(parsed.item_id.trim(), status)?;

        Ok(json!({
            "id": detail.manifest.item.id,
            "item_id": detail.manifest.item.id,
            "status": detail.manifest.item.status,
            "phase_id": loc.phase_id,
            "scope_id": loc.scope_id,
            "recurrence_count": detail.manifest.history.recurrence_count,
            "iteration_count": detail.manifest.effort.iteration_count,
            "updated_at": detail.manifest.item.updated_at
        }))
    }

    fn tool_close_item_with_fix(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Args {
            #[serde(alias = "id")]
            item_id: String,
            #[serde(alias = "message")]
            summary: String,
            #[serde(default, alias = "commit_sha")]
            commit: Option<String>,
            #[serde(default)]
            merged: bool,
        }

        let parsed: Args = serde_json::from_value(args).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid close_item_with_fix args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let detail = storage.close_item(
            parsed.item_id.trim(),
            &parsed.summary,
            non_empty(parsed.commit.as_deref()),
            parsed.merged,
        )?;
        let latest_fix = detail.manifest.history.previous_fixes.last().cloned();
        let fixes_count = detail.manifest.history.previous_fixes.len();

        Ok(json!({
            "id": detail.manifest.item.id,
            "item_id": detail.manifest.item.id,
            "status": detail.manifest.item.status,
            "fix": latest_fix,
            "previous_fixes_count": fixes_count,
            "updated_at": detail.manifest.item.updated_at
        }))
    }

    fn tool_record_regression(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Args {
            #[serde(alias = "id")]
            item_id: String,
            #[serde(default, alias = "note", alias = "summary")]
            reason: Option<String>,
        }

        let parsed: Args = serde_json::from_value(args).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid record_regression args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let detail = storage
            .record_regression(parsed.item_id.trim(), non_empty(parsed.reason.as_deref()))?;

        Ok(json!({
            "id": detail.manifest.item.id,
            "item_id": detail.manifest.item.id,
            "status": detail.manifest.item.status,
            "recurrence_count": detail.manifest.history.recurrence_count,
            "iteration_count": detail.manifest.effort.iteration_count,
            "reason": non_empty(parsed.reason.as_deref()),
            "updated_at": detail.manifest.item.updated_at
        }))
    }

    fn tool_list_projects(&mut self, _args: Value) -> Result<Value> {
        let mut registry = self.load_registry()?;
        // If this MCP server was started with an explicit workspace that isn't registered yet, include it
        if registry.projects.is_empty() {
            if let Some(ref storage) = self.storage {
                if storage.project_toml_path().is_file() {
                    if let Ok(entry) =
                        self.register_in_registry(&storage.project_root(), None, Vec::new(), true)
                    {
                        registry.active_project_id = Some(entry.id.clone());
                        registry.projects.push(entry);
                    }
                }
            }
        }
        Ok(json!({
            "active_project_id": registry.active_project_id,
            "total": registry.projects.len(),
            "projects": registry.projects
        }))
    }

    fn tool_switch_project(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Args {
            #[serde(alias = "id", alias = "name", alias = "project")]
            id_or_name: String,
        }

        let parsed: Args = serde_json::from_value(args).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid switch_project args: {e}"))
        })?;
        let active = self.switch_in_registry(parsed.id_or_name.trim())?;
        if let Ok(discovered) = TrackerStorage::discover(&active.root_path) {
            self.storage = Some(discovered);
        } else {
            self.storage = Some(TrackerStorage::new(&active.root_path));
        }

        Ok(json!({
            "active_project_id": active.id,
            "project": active
        }))
    }

    fn tool_ingest_project_directory(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Default, Deserialize)]
        struct Args {
            #[serde(default, alias = "path")]
            subpath: Option<String>,
            #[serde(default)]
            write_manifests: bool,
            #[serde(default)]
            promote_all: bool,
            #[serde(default, alias = "phase_id", alias = "phase")]
            promote_phase_id: Option<String>,
        }

        let parsed: Args = serde_json::from_value(args).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid ingest_project_directory args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let mut opts = IngestionOptions::new()
            .with_write_manifests(parsed.write_manifests)
            .with_promote_all(parsed.promote_all);
        if let Some(sub) = non_empty(parsed.subpath.as_deref()) {
            let sub_path = Path::new(sub);
            if sub_path.is_absolute() {
                let root_canon =
                    crate::storage::validate_safe_project_root(&storage.project_root())?;
                let sub_canon = std::fs::canonicalize(sub_path).map_err(|_| {
                    TrackerError::ValidationError(format!(
                        "Scan subpath '{sub}' does not exist or is outside the active project root '{}'",
                        root_canon.display()
                    ))
                })?;
                if let Ok(rel) = sub_canon.strip_prefix(&root_canon) {
                    let norm = crate::storage::normalize_relative_path(rel);
                    if !norm.is_empty() && norm != "." {
                        opts = opts.with_subpath(norm);
                    }
                } else {
                    return Err(TrackerError::ValidationError(format!(
                        "Scan subpath '{sub}' is outside the active project root '{}'",
                        root_canon.display()
                    )));
                }
            } else {
                opts = opts.with_subpath(sub);
            }
        }
        if let Some(phase) = non_empty(parsed.promote_phase_id.as_deref()) {
            opts = opts.with_promote_phase(phase);
        }

        let report = storage.ingest_project_directory(opts)?;
        Ok(json!({
            "total_components": report.components.len(),
            "total_files_indexed": report.total_files_indexed,
            "components": report.components,
            "manifests_written": report.manifests_written,
            "promoted_scopes": report.promoted_scopes
        }))
    }

    fn tool_upsert_component_manifest(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Args {
            #[serde(alias = "component_id")]
            id: String,
            #[serde(default)]
            name: Option<String>,
            #[serde(default, alias = "path")]
            relative_path: Option<String>,
            #[serde(default, alias = "type")]
            component_type: Option<String>,
            #[serde(default, alias = "desc")]
            description: Option<String>,
            #[serde(default)]
            tech_stack: Option<Vec<String>>,
            #[serde(default)]
            tags: Option<Vec<String>>,
            #[serde(default)]
            flags: Option<Vec<String>>,
            #[serde(default)]
            files: Option<Vec<String>>,
            #[serde(default)]
            dependencies: Option<Vec<ComponentDependency>>,
        }

        let parsed: Args = serde_json::from_value(args).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid upsert_component_manifest args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let comp_id = parsed.id.trim();

        let existing = storage.read_component_manifest(comp_id).ok().or_else(|| {
            parsed
                .relative_path
                .as_deref()
                .and_then(|p| storage.read_component_manifest(p).ok())
        });

        let mut manifest = existing.unwrap_or_else(|| ComponentManifest {
            id: comp_id.to_string(),
            name: parsed.name.clone().unwrap_or_else(|| comp_id.to_string()),
            relative_path: parsed
                .relative_path
                .clone()
                .unwrap_or_else(|| ".".to_string()),
            component_type: parsed
                .component_type
                .clone()
                .unwrap_or_else(|| ComponentType::Custom.to_string()),
            ..ComponentManifest::default()
        });

        manifest.id = comp_id.to_string();
        if let Some(name) = non_empty(parsed.name.as_deref()) {
            manifest.name = name.to_string();
        }
        if let Some(rel) = non_empty(parsed.relative_path.as_deref()) {
            manifest.relative_path = normalize_relative_path_str(rel);
        }
        if let Some(ctype) = non_empty(parsed.component_type.as_deref()) {
            let validated = ComponentType::from_str(ctype)
                .map(|c| c.to_string())
                .unwrap_or_else(|_| ctype.to_string());
            manifest.component_type = validated;
        }
        if let Some(desc) = parsed.description {
            manifest.description = non_empty(Some(&desc)).map(str::to_string);
        }
        if let Some(stack) = parsed.tech_stack {
            manifest.tech_stack = stack;
        }
        if let Some(tags) = parsed.tags {
            manifest.tags = tags;
        }
        if let Some(flags) = parsed.flags {
            manifest.flags = flags;
        }
        if let Some(files) = parsed.files {
            manifest.files = files;
        }
        if let Some(deps) = parsed.dependencies {
            manifest.dependencies = deps;
        }

        let written_path = storage.write_component_manifest(&manifest)?;
        let reloaded = storage.read_component_manifest(&manifest.id)?;

        Ok(json!({
            "manifest_path": written_path.to_string_lossy().to_string(),
            "component": reloaded
        }))
    }

    fn tool_promote_component_to_scope(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Args {
            #[serde(alias = "id", alias = "component")]
            component_id: String,
            #[serde(default, alias = "phase")]
            phase_id: Option<String>,
            #[serde(default, alias = "scope_id_override", alias = "scope")]
            scope_id: Option<String>,
            #[serde(default, alias = "files")]
            selected_files: Option<Vec<String>>,
        }

        let parsed: Args = serde_json::from_value(args).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid promote_component_to_scope args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let project = storage.read_project()?;
        let resolved_phase = non_empty(parsed.phase_id.as_deref())
            .unwrap_or(&project.project.active_phase)
            .to_string();

        let scope = storage.promote_component_to_scope(
            Some(&resolved_phase),
            parsed.component_id.trim(),
            non_empty(parsed.scope_id.as_deref()),
            parsed.selected_files,
        )?;

        Ok(json!({
            "phase_id": resolved_phase,
            "scope_id": scope.scope.id,
            "target": scope.scope.target,
            "bound_component": scope.scope.bound_component,
            "tracked_files": scope.scope.tracked_files,
            "scope": scope
        }))
    }

    fn tool_record_rolling_update(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Args {
            #[serde(alias = "component", alias = "id")]
            component_id: String,
            #[serde(alias = "package", alias = "name", alias = "dependency")]
            dependency_name: String,
            #[serde(default = "default_rolling_dep_kind", alias = "channel")]
            kind: String,
            #[serde(default, alias = "from_version")]
            previous_version: Option<String>,
            #[serde(alias = "to_version", alias = "version")]
            new_version: String,
            #[serde(default, alias = "source")]
            upstream_source: Option<String>,
            #[serde(default, alias = "script_path", alias = "build_script")]
            build_script_path: Option<String>,
            #[serde(default, alias = "notes", alias = "description")]
            summary: Option<String>,
            #[serde(default, alias = "item_id")]
            tracked_item_id: Option<String>,
            #[serde(default = "default_true")]
            create_tracking_item: bool,
            #[serde(default = "default_feature_str")]
            item_type: String,
            #[serde(default = "default_severity_str")]
            severity: String,
            #[serde(default, alias = "phase")]
            phase_id: Option<String>,
            #[serde(default, alias = "scope")]
            scope_id: Option<String>,
        }

        let parsed: Args = serde_json::from_value(args).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid record_rolling_update args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let mut manifest = storage.read_component_manifest(parsed.component_id.trim())?;
        let comp_id = manifest.id.clone();

        let dep_name = parsed.dependency_name.trim().to_string();
        let new_ver = parsed.new_version.trim().to_string();
        if dep_name.is_empty() || new_ver.is_empty() {
            return Err(TrackerError::ValidationError(
                "dependency_name and new_version must not be empty".to_string(),
            ));
        }

        // Ensure dependency exists on the component manifest so `pin_stable_release` captures it
        let prev_ver = non_empty(parsed.previous_version.as_deref())
            .map(str::to_string)
            .or_else(|| {
                manifest
                    .dependencies
                    .iter()
                    .find(|d| d.name == dep_name)
                    .map(|d| d.current_version.clone())
                    .filter(|v| !v.trim().is_empty())
            });

        if !manifest.dependencies.iter().any(|d| d.name == dep_name) {
            manifest.dependencies.push(ComponentDependency {
                name: dep_name.clone(),
                kind: parsed.kind.trim().to_string(),
                version_constraint: "*".to_string(),
                current_version: prev_ver.clone().unwrap_or_default(),
                upstream_source: parsed.upstream_source.clone().unwrap_or_default(),
            });
            storage.write_component_manifest(&manifest)?;
        }

        let mut linked_item_id = non_empty(parsed.tracked_item_id.as_deref()).map(str::to_string);
        if let Some(ref existing_id) = linked_item_id {
            if let Ok((loc, mut detail)) = storage.load_item_detail_by_id(existing_id) {
                let today = Utc::now().format("%Y-%m-%d").to_string();
                let script_note = parsed
                    .build_script_path
                    .as_deref()
                    .map(|p| format!(" (build script: `{p}`)"))
                    .unwrap_or_default();
                if !detail.description_markdown.ends_with('\n') {
                    detail.description_markdown.push('\n');
                }
                detail.description_markdown.push_str(&format!(
                    "- Rolling update ({today}): `{dep_name}` -> `{new_ver}` on `{comp_id}`{script_note}\n"
                ));
                storage.write_item_desc(
                    &loc.phase_id,
                    &loc.scope_id,
                    &loc.stub.id,
                    &detail.description_markdown,
                )?;
                linked_item_id = Some(loc.stub.id);
            }
        } else if parsed.create_tracking_item {
            let index = storage.load_index()?;
            let target_phase = non_empty(parsed.phase_id.as_deref())
                .unwrap_or(&index.project.project.active_phase)
                .to_string();
            let target_scope = non_empty(parsed.scope_id.as_deref())
                .map(str::to_string)
                .or_else(|| {
                    index.phase(&target_phase).and_then(|p| {
                        p.scopes
                            .iter()
                            .find(|s| {
                                s.manifest.scope.bound_component.as_deref() == Some(&comp_id)
                                    || s.scope_id == comp_id
                                    || s.manifest.scope.target == manifest.relative_path
                            })
                            .map(|s| s.scope_id.clone())
                    })
                });

            let kind = ItemType::from_str(&parsed.item_type).unwrap_or(ItemType::Feature);
            let sev = Severity::from_str(&parsed.severity).unwrap_or(Severity::Medium);
            let from_str = prev_ver.as_deref().unwrap_or("unpinned");
            let title = format!("Rolling update: {dep_name} {from_str} -> {new_ver} ({comp_id})");
            let desc = format!(
                "Upstream rolling update for `{dep_name}` (`{from_str}` -> `{new_ver}`) in component `{comp_id}` (`{}`).{}",
                manifest.relative_path,
                parsed
                    .build_script_path
                    .as_deref()
                    .map(|s| format!("\n- Target build script: `{s}`"))
                    .unwrap_or_default()
            );

            let (loc, _) = storage.create_item(CreateItemInput {
                item_type: kind,
                title,
                phase_id: Some(target_phase),
                scope_id: target_scope,
                severity: sev,
                description: Some(desc),
            })?;
            linked_item_id = Some(loc.stub.id);
        }

        let entry = RollingUpdateEntry {
            dependency_name: dep_name,
            kind: parsed.kind.trim().to_string(),
            previous_version: prev_ver,
            new_version: new_ver,
            upstream_source: non_empty(parsed.upstream_source.as_deref()).map(str::to_string),
            build_script_path: non_empty(parsed.build_script_path.as_deref())
                .map(normalize_relative_path_str),
            tracked_item_id: linked_item_id.clone(),
            status: "pending".to_string(),
            recorded_at: Some(Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            summary: non_empty(parsed.summary.as_deref()).map(str::to_string),
            verification_run: None,
        };

        let updated_manifest = storage.record_component_rolling_update(&comp_id, entry)?;
        let recorded = updated_manifest.rolling_updates.last().cloned();

        Ok(json!({
            "component_id": updated_manifest.id,
            "tracked_item_id": linked_item_id,
            "item_id": linked_item_id,
            "rolling_update": recorded,
            "component": updated_manifest
        }))
    }

    fn tool_record_verification_run(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Args {
            #[serde(alias = "component", alias = "id")]
            component_id: String,
            passed: bool,
            #[serde(default)]
            exit_code: Option<i32>,
            #[serde(default)]
            command: Option<String>,
            #[serde(default)]
            summary: Option<String>,
            #[serde(default, alias = "logs")]
            logs_excerpt: Option<String>,
            #[serde(default, alias = "tracked_item_id")]
            item_id: Option<String>,
        }

        let parsed: Args = serde_json::from_value(args).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid record_verification_run args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let exit_code = parsed
            .exit_code
            .unwrap_or(if parsed.passed { 0 } else { 1 });

        let record = VerificationRecord {
            passed: parsed.passed,
            exit_code,
            command: parsed.command.unwrap_or_default(),
            summary: parsed.summary.unwrap_or_else(|| {
                if parsed.passed {
                    "Verification passed".to_string()
                } else {
                    "Verification failed".to_string()
                }
            }),
            logs_excerpt: non_empty(parsed.logs_excerpt.as_deref()).map(str::to_string),
            verified_at: Some(Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()),
        };

        let updated_manifest =
            storage.record_component_verification(parsed.component_id.trim(), record.clone())?;

        let target_item_id = non_empty(parsed.item_id.as_deref())
            .map(str::to_string)
            .or_else(|| {
                updated_manifest
                    .rolling_updates
                    .iter()
                    .rev()
                    .find_map(|u| u.tracked_item_id.clone())
            });

        if let Some(ref item_id) = target_item_id {
            if let Ok((loc, mut detail)) = storage.load_item_detail_by_id(item_id) {
                let badge = if record.passed { "PASSED" } else { "FAILED" };
                if !detail.description_markdown.ends_with('\n') {
                    detail.description_markdown.push('\n');
                }
                detail.description_markdown.push_str(&format!(
                    "- Verification [{badge}] (exit {}): `{}` — {}\n",
                    record.exit_code, record.command, record.summary
                ));
                if let Some(ref logs) = record.logs_excerpt {
                    detail
                        .description_markdown
                        .push_str(&format!("  - Logs excerpt: `{}`\n", logs.trim()));
                }
                storage.write_item_desc(
                    &loc.phase_id,
                    &loc.scope_id,
                    &loc.stub.id,
                    &detail.description_markdown,
                )?;
            }
        }

        Ok(json!({
            "component_id": updated_manifest.id,
            "passed": record.passed,
            "exit_code": record.exit_code,
            "tracked_item_id": target_item_id,
            "verification_run": record,
            "component": updated_manifest
        }))
    }

    fn tool_pin_stable_release(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Args {
            #[serde(alias = "component", alias = "id")]
            component_id: String,
            #[serde(default, alias = "commit")]
            commit_sha: Option<String>,
            #[serde(default, alias = "script_path", alias = "build_script")]
            build_script_path: Option<String>,
            #[serde(default, alias = "flags")]
            build_flags: Vec<String>,
            #[serde(default, alias = "dependency_versions")]
            dependency_pins: Option<BTreeMap<String, String>>,
        }

        let parsed: Args = serde_json::from_value(args).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid pin_stable_release args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let mut manifest = storage.read_component_manifest(parsed.component_id.trim())?;

        if let Some(explicit_pins) = parsed.dependency_pins {
            for (dep_name, dep_ver) in explicit_pins {
                if let Some(existing) = manifest
                    .dependencies
                    .iter_mut()
                    .find(|d| d.name == dep_name)
                {
                    existing.current_version = dep_ver;
                } else {
                    manifest.dependencies.push(ComponentDependency {
                        name: dep_name,
                        kind: "custom".to_string(),
                        version_constraint: "*".to_string(),
                        current_version: dep_ver,
                        upstream_source: String::new(),
                    });
                }
            }
            storage.write_component_manifest(&manifest)?;
        }

        let inferred_script = non_empty(parsed.build_script_path.as_deref())
            .map(str::to_string)
            .or_else(|| {
                manifest
                    .rolling_updates
                    .iter()
                    .rev()
                    .find_map(|u| u.build_script_path.clone())
            })
            .or_else(|| {
                manifest
                    .pinned_release
                    .as_ref()
                    .and_then(|p| p.build_script_path.clone())
            });

        let latest_verification = manifest
            .rolling_updates
            .iter()
            .rev()
            .find_map(|u| u.verification_run.clone())
            .or_else(|| {
                manifest
                    .pinned_release
                    .as_ref()
                    .and_then(|p| p.last_verification_run.clone())
            });

        let pinned_manifest = storage.pin_component_release(
            &manifest.id,
            non_empty(parsed.commit_sha.as_deref()),
            inferred_script,
            parsed.build_flags,
            latest_verification,
        )?;
        let pin = pinned_manifest.pinned_release.clone();

        Ok(json!({
            "component_id": pinned_manifest.id,
            "pinned_release": pin,
            "last_known_good": pinned_manifest.pinned_release,
            "component": pinned_manifest
        }))
    }

    fn tool_rollback_to_pinned_release(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Args {
            #[serde(alias = "component", alias = "id")]
            component_id: String,
            #[serde(default = "default_true", alias = "restore")]
            restore_manifest: bool,
            #[serde(default, alias = "active_item_id", alias = "tracked_item_id")]
            item_id: Option<String>,
            #[serde(default, alias = "summary", alias = "note")]
            reason: Option<String>,
        }

        let parsed: Args = serde_json::from_value(args).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid rollback_to_pinned_release args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let initial_manifest = storage.read_component_manifest(parsed.component_id.trim())?;
        let Some(pinned) = initial_manifest.pinned_release.clone() else {
            return Err(TrackerError::ValidationError(format!(
                "Component '{}' does not have a pinned_release configuration to roll back to",
                initial_manifest.id
            )));
        };

        let target_item_id = non_empty(parsed.item_id.as_deref())
            .map(str::to_string)
            .or_else(|| {
                initial_manifest
                    .rolling_updates
                    .iter()
                    .rev()
                    .find_map(|u| u.tracked_item_id.clone())
            })
            .or_else(|| {
                storage
                    .read_active_timer()
                    .ok()
                    .flatten()
                    .map(|t| t.item_id)
            })
            .or_else(|| select_best_active_item(&storage, None, None).ok().flatten());

        let final_manifest = if parsed.restore_manifest {
            storage.rollback_component_to_pinned(&initial_manifest.id)?
        } else {
            initial_manifest
        };

        let default_reason = format!(
            "Rolled back component '{}' to pinned release commit {}",
            final_manifest.id, pinned.commit_sha
        );
        let regression_reason = non_empty(parsed.reason.as_deref()).unwrap_or(&default_reason);

        let mut regression_recorded = false;
        let mut recurrence_count = None;
        let mut iteration_count = None;

        if let Some(ref item_id) = target_item_id {
            if let Ok(detail) = storage.record_regression(item_id, Some(regression_reason)) {
                regression_recorded = true;
                recurrence_count = Some(detail.manifest.history.recurrence_count);
                iteration_count = Some(detail.manifest.effort.iteration_count);
            }
        }

        Ok(json!({
            "component_id": final_manifest.id,
            "restored": parsed.restore_manifest,
            "last_known_good": pinned,
            "pinned_release": final_manifest.pinned_release,
            "regression_recorded": regression_recorded,
            "item_id": target_item_id,
            "recurrence_count": recurrence_count,
            "iteration_count": iteration_count,
            "component": final_manifest
        }))
    }

    fn tool_git_status(&mut self, _args: Value) -> Result<Value> {
        let storage = self.resolve_active_storage()?;
        let status = storage.git_status()?;
        serde_json::to_value(status).map_err(|e| TrackerError::ValidationError(e.to_string()))
    }

    fn tool_git_fetch(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Default, Deserialize)]
        struct Args {
            #[serde(default)]
            auth_override: Option<String>,
            #[serde(default)]
            pat: Option<Value>,
        }

        let parsed: Args = serde_json::from_value(args)
            .map_err(|e| TrackerError::ValidationError(format!("Invalid git_fetch args: {e}")))?;
        let auth = mcp_auth_request(parsed.auth_override.as_deref(), parsed.pat.as_ref())?;
        let storage = self.resolve_active_storage()?;
        let status = storage.git_fetch(&auth)?;
        serde_json::to_value(status).map_err(|e| TrackerError::ValidationError(e.to_string()))
    }

    fn tool_git_pull(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Default, Deserialize)]
        struct Args {
            #[serde(default)]
            auth_override: Option<String>,
            #[serde(default)]
            pat: Option<Value>,
        }

        let parsed: Args = serde_json::from_value(args)
            .map_err(|e| TrackerError::ValidationError(format!("Invalid git_pull args: {e}")))?;
        let auth = mcp_auth_request(parsed.auth_override.as_deref(), parsed.pat.as_ref())?;
        let storage = self.resolve_active_storage()?;
        let outcome = storage.git_pull_with_report(&auth)?;
        let status = storage.git_status()?;
        Ok(json!({
            "updated": outcome.updated,
            "status": status,
            "duplicate_ids": outcome.duplicate_ids
        }))
    }

    fn tool_git_push(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Default, Deserialize)]
        struct Args {
            #[serde(default)]
            auth_override: Option<String>,
            #[serde(default)]
            pat: Option<Value>,
        }

        let parsed: Args = serde_json::from_value(args)
            .map_err(|e| TrackerError::ValidationError(format!("Invalid git_push args: {e}")))?;
        let auth = mcp_auth_request(parsed.auth_override.as_deref(), parsed.pat.as_ref())?;
        let storage = self.resolve_active_storage()?;
        storage.git_push(&auth)?;
        let status = storage.git_status()?;
        Ok(json!({
            "pushed": true,
            "status": status
        }))
    }

    fn tool_git_commit_documented(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Default, Deserialize)]
        struct Args {
            #[serde(default, alias = "id")]
            item_id: Option<String>,
            #[serde(default, alias = "scope")]
            scope_id: Option<String>,
            #[serde(default, alias = "message")]
            summary: String,
            #[serde(default, alias = "notes")]
            body: Option<String>,
            #[serde(default = "default_tracker_only")]
            tracker_only: bool,
            #[serde(default)]
            paths: Vec<String>,
        }
        fn default_tracker_only() -> bool {
            true
        }

        let parsed: Args = serde_json::from_value(args).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid git_commit_documented args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let outcome = storage.git_commit_documented(DocumentedCommitInput {
            item_id: non_empty(parsed.item_id.as_deref()).map(str::to_string),
            scope_id: non_empty(parsed.scope_id.as_deref()).map(str::to_string),
            summary: parsed.summary,
            body: non_empty(parsed.body.as_deref()).map(str::to_string),
            tracker_only: parsed.tracker_only,
            paths: parsed.paths,
        })?;
        serde_json::to_value(outcome).map_err(|e| TrackerError::ValidationError(e.to_string()))
    }

    fn tool_git_sync(&mut self, args: Value) -> Result<Value> {
        #[derive(Debug, Default, Deserialize)]
        struct Args {
            #[serde(default, alias = "summary")]
            message: Option<String>,
            #[serde(default)]
            auth_override: Option<String>,
            #[serde(default)]
            pat: Option<Value>,
            #[serde(default)]
            commit_only: bool,
        }

        let parsed: Args = serde_json::from_value(args)
            .map_err(|e| TrackerError::ValidationError(format!("Invalid git_sync args: {e}")))?;
        let auth = mcp_auth_request(parsed.auth_override.as_deref(), parsed.pat.as_ref())?;
        let storage = self.resolve_active_storage()?;
        let (sha, duplicate_ids) = if parsed.commit_only {
            (
                storage.git_commit_tracker(non_empty(parsed.message.as_deref()))?,
                Vec::new(),
            )
        } else {
            let outcome =
                storage.git_sync_with_report(non_empty(parsed.message.as_deref()), &auth)?;
            (outcome.sha, outcome.duplicate_ids)
        };
        let tracker_dirty_after = storage
            .git_status()
            .map(|s| s.tracker_dirty)
            .unwrap_or(false);

        Ok(json!({
            "commit_only": parsed.commit_only,
            "committed_or_head_sha": sha,
            "tracker_dirty_after": tracker_dirty_after,
            "duplicate_ids": duplicate_ids
        }))
    }
}

fn select_best_active_item(
    storage: &TrackerStorage,
    phase_filter: Option<&str>,
    scope_filter: Option<&str>,
) -> Result<Option<String>> {
    let index = storage.load_index()?;
    let active_phase = index.project.project.active_phase.as_str();

    let all = index.all_items();
    let filter_candidates = |restrict_to_active_phase: bool| -> Vec<ItemLocation> {
        all.iter()
            .filter(|loc| {
                if loc.stub.status.is_completed() {
                    return false;
                }
                if let Some(p) = phase_filter {
                    if loc.phase_id != p {
                        return false;
                    }
                } else if restrict_to_active_phase && loc.phase_id != active_phase {
                    return false;
                }
                if let Some(s) = scope_filter {
                    if loc.scope_id != s {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    };

    let mut candidates = filter_candidates(phase_filter.is_none());
    if candidates.is_empty() && phase_filter.is_none() {
        candidates = filter_candidates(false);
    }
    if candidates.is_empty() {
        return Ok(None);
    }

    let status_rank = |st: ItemStatus| -> u8 {
        match st {
            ItemStatus::InProgress => 0,
            ItemStatus::Open => 1,
            ItemStatus::Pending => 2,
            ItemStatus::Closed | ItemStatus::Merged => 3,
        }
    };

    candidates.sort_by(|a, b| {
        status_rank(a.stub.status)
            .cmp(&status_rank(b.stub.status))
            .then_with(|| b.stub.severity.cmp(&a.stub.severity))
            .then_with(|| a.stub.id.cmp(&b.stub.id))
    });

    Ok(candidates.first().map(|loc| loc.stub.id.clone()))
}

fn read_next_jsonrpc_message<R: BufRead>(reader: &mut R) -> io::Result<Option<(String, bool)>> {
    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Ok(None);
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Support optional LSP/MCP Content-Length header framing
        let lower = trimmed.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            let content_len: usize = rest.trim().parse().map_err(|err| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid Content-Length header: {err}"),
                )
            })?;

            loop {
                let mut header_line = String::new();
                let n = reader.read_line(&mut header_line)?;
                if n == 0 || header_line.trim().is_empty() {
                    break;
                }
            }

            let mut buf = vec![0u8; content_len];
            reader.read_exact(&mut buf)?;
            let payload = String::from_utf8(buf).map_err(|err| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid UTF-8 in JSON-RPC body: {err}"),
                )
            })?;
            return Ok(Some((payload, true)));
        }

        // Standard newline-delimited JSON-RPC message
        return Ok(Some((trimmed.to_string(), false)));
    }
}

/// Builds the credential request for an MCP git tool. Agents may only name a vault account that
/// is bound to the project in the local registry; raw PATs are refused outright.
fn mcp_auth_request(auth_override: Option<&str>, pat: Option<&Value>) -> Result<GitAuthRequest> {
    if pat.is_some_and(|v| !v.is_null()) {
        return Err(TrackerError::ValidationError(
            "Personal Access Tokens are not accepted over MCP; use a vault account bound in the local registry (auth_override)".to_string(),
        ));
    }
    Ok(non_empty(auth_override)
        .map(GitAuthRequest::with_account)
        .unwrap_or_default())
}

fn non_empty(val: Option<&str>) -> Option<&str> {
    val.map(str::trim).filter(|s| !s.is_empty())
}

fn default_jsonrpc_version() -> String {
    "2.0".to_string()
}

fn default_item_type_str() -> String {
    "bug".to_string()
}

fn default_feature_str() -> String {
    "feature".to_string()
}

fn default_severity_str() -> String {
    "medium".to_string()
}

fn default_rolling_dep_kind() -> String {
    "apt_sid".to_string()
}

fn default_true() -> bool {
    true
}
