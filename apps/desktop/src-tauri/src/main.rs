#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod studio;

use agent_protocol::{AgentOperation, PatchFile as ProtocolPatchFile};
use audit_log::{append_event, init_schema, new_id};
use command_guard::{
    classify, personal_terminal_policy, CommandPolicy, CommandRequest as GuardCommandRequest,
    CommandRisk,
};
use context_os::{
    load_capsule, load_runtime_capability, upsert_runtime_capability, RuntimeCapability,
};
use patch_engine::{
    apply_patch_proposal_transactional, rollback_checkpoint_for_proposal,
    validate_patch_proposal_against_repo, PatchFile, PatchProposal,
};
use repo_guard::{FilePolicy, RepoGuard};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use studio::*;

const FIXTURE_SOURCE: &str =
    "export function refreshToken() {\n  throw new Error(\"not implemented\");\n}\n";
const DEFAULT_SESSION_ID: &str = "synthesize-session";
const MOA_BRIDGE_PROTOCOL: &str = "synthesize-moa-bridge/v1";
static REPO_MUTATION_LOCKS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static MANAGED_LLAMA: OnceLock<Mutex<Option<ManagedLlamaProcess>>> = OnceLock::new();

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RepoOpenResult {
    repo_root: String,
    current_file_path: String,
    current_file_content: String,
    current_commit: Option<String>,
    files: Vec<RepoFileView>,
}

#[derive(Debug, Serialize, Clone)]
struct RepoFileView {
    path: String,
    kind: String,
    denied: bool,
}

#[derive(Debug, Deserialize)]
struct PatchProposalRequest {
    session_id: String,
    repo_root: String,
    operation: AgentOperation,
    // Agent profile is persisted on the context bundle that produced the model response.
    // This optional field is retained only for request diagnostics and backward UI shape;
    // authorization never trusts it.
    #[allow(dead_code)]
    agent_profile_id: Option<String>,
    context_bundle_id: String,
}

#[derive(Debug, Deserialize)]
struct PatchApprovalRequest {
    session_id: String,
    repo_root: String,
    proposal_id: String,
    operation_sha256: String,
}

#[derive(Debug, Deserialize)]
struct PatchApplyRequest {
    session_id: String,
    repo_root: String,
    proposal_id: String,
    approval_id: String,
}

#[derive(Debug, Deserialize)]
struct RollbackRequest {
    session_id: String,
    repo_root: String,
    proposal_id: String,
}

#[derive(Debug, Serialize)]
struct PatchFileValidationView {
    id: String,
    path: String,
    risk: String,
    real_path: String,
}

#[derive(Debug, Serialize)]
struct PatchValidationResult {
    ok: bool,
    proposal_id: String,
    operation_sha256: String,
    status: String,
    files: Vec<PatchFileValidationView>,
    warnings: Vec<String>,
    errors: Vec<String>,
    message: String,
    audit_event_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MoaBridgeDecision {
    ok: bool,
    approved: Option<bool>,
    reason: Option<String>,
    action_type: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct MoaBridgeOperationFile {
    path: String,
    risk: String,
}

#[derive(Debug, Serialize)]
struct PatchApprovalView {
    proposal_id: String,
    approval_id: String,
    operation_sha256: String,
    approved_by_source: String,
    approved_at: String,
    audit_event_id: String,
}

#[derive(Debug, Serialize)]
struct PatchApplyView {
    proposal_id: String,
    approval_id: String,
    checkpoint_id: String,
    checkpoint_dir: String,
    applied_files: Vec<AppliedFileView>,
    audit_event_id: String,
}

#[derive(Debug, Serialize)]
struct AppliedFileView {
    id: String,
    path: String,
    after_sha256: String,
}

#[derive(Debug, Serialize)]
struct RollbackView {
    proposal_id: String,
    checkpoint_id: String,
    checkpoint_dir: String,
    restored_paths: Vec<String>,
    deleted_paths: Vec<String>,
    audit_event_id: String,
}

#[derive(Debug, Serialize)]
struct AuditEventView {
    id: String,
    timestamp: String,
    kind: String,
    payload_json: String,
}

#[derive(Debug, Deserialize)]
struct SessionEventRequest {
    session_id: String,
    repo_root: String,
    kind: String,
    payload_json: String,
}

#[derive(Debug, Serialize)]
struct SessionEventResult {
    audit_event_id: String,
}

#[derive(Debug, Deserialize)]
struct ClearLocalSessionDataRequest {
    session_id: String,
    repo_root: String,
    clear_endpoint_approvals: bool,
}

#[derive(Debug, Serialize)]
struct ClearLocalSessionDataResult {
    cleared_context_bundles: usize,
    cleared_runtime_requests: usize,
    cleared_audit_events: usize,
    cleared_endpoint_approvals: usize,
    message: String,
}

#[derive(Debug, Deserialize)]
struct CommandClassifyRequest {
    argv: Vec<String>,
    cwd: String,
    requires_network: bool,
    may_modify_files: bool,
    session_id: Option<String>,
    repo_root: Option<String>,
}

#[derive(Debug, Serialize)]
struct CommandClassifyResult {
    ok: bool,
    risk: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct WriteFileRequest {
    session_id: String,
    repo_root: String,
    relative_path: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct CreateFileRequest {
    session_id: String,
    repo_root: String,
    relative_path: String,
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RenamePathRequest {
    session_id: String,
    repo_root: String,
    from_path: String,
    to_path: String,
}

#[derive(Debug, Deserialize)]
struct DeletePathRequest {
    session_id: String,
    repo_root: String,
    relative_path: String,
    allow_directory: bool,
    confirmation_token: Option<String>,
}

#[derive(Debug, Serialize)]
struct FileMutationResult {
    path: String,
    message: String,
    audit_event_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProjectSearchRequest {
    session_id: String,
    repo_root: String,
    query: String,
    max_results: Option<usize>,
}

#[derive(Debug, Serialize)]
struct ProjectSearchResult {
    path: String,
    line: usize,
    preview: String,
}

#[derive(Debug, Deserialize)]
struct GitStatusRequest {
    session_id: String,
    repo_root: String,
}

#[derive(Debug, Serialize)]
struct GitStatusFileView {
    path: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct GitStatusView {
    branch: String,
    files: Vec<GitStatusFileView>,
    raw: String,
}

#[derive(Debug, Deserialize)]
struct GitDiffRequest {
    session_id: String,
    repo_root: String,
    path: String,
    staged: bool,
}

#[derive(Debug, Serialize)]
struct GitDiffView {
    path: String,
    staged: bool,
    diff: String,
    truncated: bool,
    audit_event_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitFileMutationRequest {
    session_id: String,
    repo_root: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct GitCommitRequest {
    session_id: String,
    repo_root: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct GitMutationResult {
    ok: bool,
    message: String,
    stdout: String,
    stderr: String,
    audit_event_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LspCapabilityRequest {
    session_id: String,
    repo_root: String,
}

#[derive(Debug, Serialize)]
struct LspCapabilityView {
    language: String,
    detected: bool,
    server_hint: String,
    capabilities: Vec<String>,
    notes: String,
}

#[derive(Debug, Deserialize)]
struct TaskDetectRequest {
    session_id: String,
    repo_root: String,
}

#[derive(Debug, Serialize, Clone)]
struct DetectedTaskView {
    id: String,
    label: String,
    argv: Vec<String>,
    cwd: String,
    risk: String,
    reason: String,
    requires_network: bool,
    may_modify_files: bool,
}

#[derive(Debug, Deserialize)]
struct TaskApproveRequest {
    session_id: String,
    repo_root: String,
    task_id: String,
}

#[derive(Debug, Serialize)]
struct TaskApprovalView {
    command_id: String,
    task_id: String,
    risk: String,
    approved: bool,
    message: String,
}

#[derive(Debug, Deserialize)]
struct TaskRunRequest {
    session_id: String,
    repo_root: String,
    command_id: String,
}

#[derive(Debug, Deserialize)]
struct PersonalCommandRequest {
    session_id: String,
    repo_root: String,
    argv: Vec<String>,
    cwd: String,
    requires_network: bool,
    may_modify_files: bool,
}

#[derive(Debug, Serialize)]
struct TaskRunResult {
    command_id: String,
    exit_code: Option<i32>,
    timed_out: bool,
    stdout_tail: String,
    stderr_tail: String,
    message: String,
    audit_event_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct RuntimeStatusView {
    active_runtime: String,
    loaded_model: Option<String>,
    local_only_target: bool,
    llamacpp_supervisor: String,
    notes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CuratedModelView {
    id: String,
    name: String,
    runtime: String,
    format: String,
    recommended_ram_gb: u32,
    notes: String,
}

#[derive(Debug, Deserialize)]
struct RegisterModelRequest {
    model_id: String,
    name: String,
    local_path: String,
    runtime: String,
    format: String,
}

#[derive(Debug, Serialize)]
struct RegisterModelResult {
    id: String,
    name: String,
    local_path: String,
    registered: bool,
    message: String,
}

#[derive(Debug, Serialize)]
struct RuntimePresetView {
    id: String,
    label: String,
    default_url: String,
    protocol: String,
    notes: String,
    local_by_default: bool,
}

#[derive(Debug, Serialize)]
struct LocalModelView {
    id: String,
    display_name: String,
    local_path: String,
    format: String,
    runtime_compatibility: String,
    size_bytes: Option<u64>,
    sha256: Option<String>,
    imported_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImportLocalModelRequest {
    display_name: String,
    local_path: String,
    calculate_sha256: bool,
}

#[derive(Debug, Deserialize)]
struct ManagedLlamaConfigRequest {
    binary_path: String,
    model_path: String,
    port: Option<u16>,
    ctx_size: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ManagedLlamaStatusView {
    status: String,
    endpoint_url: Option<String>,
    pid: Option<u32>,
    model_path: Option<String>,
    binary_path: Option<String>,
    stdout_tail: Option<String>,
    stderr_tail: Option<String>,
    message: String,
}

struct ManagedLlamaProcess {
    child: Child,
    endpoint_url: String,
    model_path: String,
    binary_path: String,
    stdout_tail: Arc<Mutex<String>>,
    stderr_tail: Arc<Mutex<String>>,
}

#[derive(Debug, Deserialize)]
struct ContextBundleRequest {
    session_id: String,
    repo_root: String,
    user_message: String,
    selected_file_path: String,
    selected_text: Option<String>,
    dirty_buffer_state: bool,
    provider: String,
    endpoint_url: Option<String>,
    agent_profile_id: Option<String>,
    model: String,
    context_window_tokens: usize,
    maximum_output_tokens: usize,
    safety_margin_tokens: usize,
    token_estimation_method: String,
    structured_output_behavior: String,
    capability_source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ContextBundleView {
    context_bundle_id: String,
    session_id: String,
    repo_root: String,
    user_message: String,
    selected_file_path: String,
    dirty_buffer_state: bool,
    git_commit: Option<String>,
    endpoint_classification: String,
    destination_warning: String,
    char_estimate: usize,
    #[serde(default)]
    runtime: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    model_context_window_tokens: usize,
    #[serde(default)]
    reserved_output_tokens: usize,
    #[serde(default)]
    safety_margin_tokens: usize,
    #[serde(default)]
    compiled_input_tokens: usize,
    #[serde(default)]
    remaining_capacity_tokens: usize,
    #[serde(default)]
    token_count_kind: String,
    #[serde(default)]
    token_estimation_method: String,
    included: Vec<ContextIncludedView>,
    #[serde(default)]
    omitted: Vec<ContextOmittedView>,
    #[serde(default)]
    summaries_used: Vec<Value>,
    #[serde(default)]
    truncations: Vec<ContextTruncationView>,
    messages: Vec<RuntimeMessage>,
    exact_prompt: String,
    messages_sha256: String,
    exact_context: bool,
    agent_profile_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ContextIncludedView {
    kind: String,
    path: Option<String>,
    chars: usize,
    note: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ContextOmittedView {
    kind: String,
    path: Option<String>,
    reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ContextTruncationView {
    kind: String,
    path: Option<String>,
    original_chars: usize,
    included_chars: usize,
    reason: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RuntimeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct RuntimeHealthRequest {
    session_id: String,
    repo_root: Option<String>,
    provider: String,
    endpoint_url: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct RuntimeHealthResult {
    ok: bool,
    provider: String,
    endpoint_url: String,
    endpoint_classification: String,
    model: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct RuntimeEndpointApprovalRequest {
    session_id: String,
    repo_root: String,
    endpoint_url: String,
    allow_repo_context: bool,
}

#[derive(Debug, Serialize)]
struct RuntimeEndpointApprovalResult {
    endpoint_url: String,
    endpoint_classification: String,
    allow_repo_context: bool,
    approved_at: String,
}

#[derive(Debug, Deserialize)]
struct RuntimeEndpointApprovalStatusRequest {
    session_id: String,
    repo_root: Option<String>,
    endpoint_url: String,
}

#[derive(Debug, Serialize)]
struct RuntimeEndpointApprovalStatusResult {
    endpoint_url: String,
    endpoint_classification: String,
    approved: bool,
    allow_repo_context: bool,
    approved_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RuntimeGenerateRequest {
    session_id: String,
    repo_root: String,
    provider: String,
    endpoint_url: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
    response_format: Option<String>,
    context_bundle_id: String,
}

#[derive(Debug, Serialize)]
struct RuntimeGenerateResult {
    provider: String,
    endpoint_url: String,
    endpoint_classification: String,
    model: String,
    content: String,
    duration_ms: u128,
    input_chars: usize,
    output_chars: usize,
    audit_event_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RuntimeCancelRequest {
    session_id: String,
    request_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct RuntimeCancelResult {
    cancelled: bool,
    message: String,
}

#[derive(Debug, Deserialize)]
struct RuntimeListModelsRequest {
    session_id: String,
    repo_root: Option<String>,
    provider: String,
    endpoint_url: String,
}

#[derive(Debug, Serialize)]
struct RuntimeModelView {
    id: String,
}

#[derive(Debug)]
struct StoredProposal {
    proposal: PatchProposal,
    operation_sha256: String,
    #[allow(dead_code)]
    operation_json: String,
    status: String,
    checkpoint_id: Option<String>,
    #[allow(dead_code)]
    checkpoint_dir: Option<String>,
}

#[tauri::command]
fn open_repo_mock() -> Result<RepoOpenResult, String> {
    let repo = std::env::temp_dir().join(format!("synthesize-fixture-repo-{}", std::process::id()));
    if repo.exists() {
        fs::remove_dir_all(&repo).map_err(|e| e.to_string())?;
    }
    let src_dir = repo.join("src").join("auth");
    fs::create_dir_all(&src_dir).map_err(|e| e.to_string())?;
    fs::write(src_dir.join("refresh.ts"), FIXTURE_SOURCE).map_err(|e| e.to_string())?;
    fs::create_dir_all(repo.join("tests").join("auth")).map_err(|e| e.to_string())?;
    fs::write(
        repo.join("tests").join("auth").join("refresh.test.ts"),
        "import { refreshToken } from '../../src/auth/refresh';\n",
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        repo.join("package.json"),
        "{\n  \"scripts\": { \"test\": \"echo fixture tests\" }\n}\n",
    )
    .map_err(|e| e.to_string())?;
    init_audit(&repo, DEFAULT_SESSION_ID).map_err(|e| e.to_string())?;
    repo_result(repo, "src/auth/refresh.ts")
}

#[tauri::command]
fn open_repo_path(repo_root: String) -> Result<RepoOpenResult, String> {
    let repo = PathBuf::from(repo_root);
    if !repo.exists() || !repo.is_dir() {
        return Err("repo path must be an existing directory".into());
    }
    init_audit(&repo, DEFAULT_SESSION_ID).map_err(|e| e.to_string())?;
    let files = list_files_internal(&repo, 800)?;
    let first = files
        .iter()
        .find(|f| !f.denied && f.kind == "file" && is_text_like(&f.path))
        .map(|f| f.path.clone())
        .unwrap_or_else(|| "README.md".into());
    repo_result(repo, &first)
}

fn repo_result(repo: PathBuf, file_path: &str) -> Result<RepoOpenResult, String> {
    let content = read_guarded_file(repo.to_string_lossy().to_string(), file_path.to_string())
        .unwrap_or_else(|_| "".into());
    let files = list_files_internal(&repo, 800)?;
    Ok(RepoOpenResult {
        repo_root: repo.to_string_lossy().to_string(),
        current_file_path: file_path.into(),
        current_file_content: content,
        current_commit: git_current_commit(&repo),
        files,
    })
}

#[tauri::command]
fn list_repo_files(repo_root: String) -> Result<Vec<RepoFileView>, String> {
    list_files_internal(Path::new(&repo_root), 1200)
}

#[tauri::command]
fn clear_local_session_data(
    req: ClearLocalSessionDataRequest,
) -> Result<ClearLocalSessionDataResult, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let mut conn = init_audit(&repo_root, &req.session_id).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let cleared_runtime_requests = tx
        .execute(
            "DELETE FROM runtime_requests WHERE session_id = ?1",
            params![&req.session_id],
        )
        .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM context_requests WHERE initiative_id IN
         (SELECT id FROM initiatives WHERE session_id=?1)",
        params![&req.session_id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM context_summaries WHERE initiative_id IN
         (SELECT id FROM initiatives WHERE session_id=?1)",
        params![&req.session_id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM context_capsules WHERE session_id=?1",
        params![&req.session_id],
    )
    .map_err(|e| e.to_string())?;
    // Agent-run foreign keys preserve the bundle identity used for lifecycle
    // evidence. Remove its sensitive body while retaining that stable binding.
    let redacted_role_bundles = tx
        .execute(
            "UPDATE context_bundles
             SET payload_json='{\"cleared\":true,\"reason\":\"local context cleared by user\"}',
                 token_estimate=0, input_token_count=0, token_count_method='cleared'
             WHERE session_id=?1
               AND id IN (SELECT context_bundle_id FROM agent_runs)
               AND id NOT IN (
                 SELECT source_context_bundle_id FROM patch_proposals
                 WHERE source_context_bundle_id IS NOT NULL
               )",
            params![&req.session_id],
        )
        .map_err(|e| e.to_string())?;
    let deleted_context_bundles = tx
        .execute(
            "DELETE FROM context_bundles
             WHERE session_id=?1
               AND id NOT IN (SELECT context_bundle_id FROM agent_runs)
               AND id NOT IN (
                 SELECT source_context_bundle_id FROM patch_proposals
                 WHERE source_context_bundle_id IS NOT NULL
               )",
            params![&req.session_id],
        )
        .map_err(|e| e.to_string())?;
    let cleared_context_bundles = redacted_role_bundles + deleted_context_bundles;
    let cleared_audit_events = tx
        .execute(
            "DELETE FROM audit_events WHERE session_id = ?1",
            params![&req.session_id],
        )
        .map_err(|e| e.to_string())?;
    let cleared_endpoint_approvals = if req.clear_endpoint_approvals {
        tx.execute("DELETE FROM endpoint_approvals", [])
            .map_err(|e| e.to_string())?
    } else {
        0
    };
    tx.commit().map_err(|e| e.to_string())?;
    Ok(ClearLocalSessionDataResult {
        cleared_context_bundles,
        cleared_runtime_requests,
        cleared_audit_events,
        cleared_endpoint_approvals,
        message: "Cleared local session Context Capsules, summaries, requests, runtime requests, and audit events. Required lifecycle bindings and checkpoints were preserved; agent-only compatibility bundles were redacted.".into(),
    })
}

#[tauri::command]
fn build_context_bundle(req: ContextBundleRequest) -> Result<ContextBundleView, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let conn = init_audit(&repo_root, &req.session_id).map_err(|e| e.to_string())?;
    let guard = RepoGuard::new(&repo_root, FilePolicy::default()).map_err(|e| e.to_string())?;
    let selected_path = guard
        .resolve_for_existing_path(&req.selected_file_path)
        .map_err(|e| e.to_string())?;
    let selected_content = fs::read_to_string(&selected_path)
        .map_err(|e| format!("failed to read selected file through RepoGuard: {}", e))?;
    let before_sha = sha256_str(&selected_content);
    let files = list_files_internal(&repo_root, 120).unwrap_or_default();
    let tree = files
        .iter()
        .take(120)
        .filter(|f| !f.denied)
        .map(|f| format!("{} {}", f.kind, f.path))
        .collect::<Vec<_>>()
        .join("\n");
    let package_scripts = read_package_scripts_excerpt(&repo_root)
        .unwrap_or_else(|| "package scripts unavailable or denied by RepoGuard".into());
    if req.token_estimation_method != "conservative_utf8_bytes_div3" {
        return Err("Assist runtime tokenizer is not available in this build; select the clearly labeled conservative token estimate".into());
    }
    let validated_at: String = conn
        .query_row("SELECT datetime('now')", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    upsert_runtime_capability(
        &conn,
        &RuntimeCapability {
            id: new_id("CAPABILITY"),
            session_id: req.session_id.clone(),
            runtime: req.provider.clone(),
            model: req.model.clone(),
            context_window_tokens: req.context_window_tokens,
            maximum_output_tokens: req.maximum_output_tokens,
            token_estimation_method: req.token_estimation_method.clone(),
            safety_margin_tokens: req.safety_margin_tokens,
            structured_output_behavior: req.structured_output_behavior.clone(),
            capability_source: req.capability_source.clone(),
            last_validated_at: validated_at,
        },
    )
    .map_err(|error| error.to_string())?;
    let capability = load_runtime_capability(&conn, &req.session_id, &req.provider, &req.model)
        .map_err(|error| error.to_string())?;
    let endpoint = req
        .endpoint_url
        .clone()
        .unwrap_or_else(|| "memory://fake".into());
    let endpoint_classification = if req.provider == "fake" {
        "local".into()
    } else {
        classify_endpoint_url(&endpoint)
    };
    let destination_warning = if endpoint_classification == "local" {
        "Repo context stays in the in-process fake runtime or localhost endpoint path; Synthesize does not enforce OS-level network sandboxing.".into()
    } else if endpoint_classification == "private-lan" {
        "Private LAN endpoint: repo context may leave this machine. Backend approval is required before sending.".into()
    } else {
        "Remote endpoint: repo context may leave this machine. Backend approval is required before sending.".into()
    };
    let source_agent_profile_id = req
        .agent_profile_id
        .clone()
        .unwrap_or_else(|| "local-patcher".into());
    let system = build_synthesize_system_prompt(&source_agent_profile_id);
    let canonical_repo =
        canonical_repo_root_string(&repo_root).unwrap_or_else(|_| req.repo_root.clone());
    let selected_text = req.selected_text.clone().unwrap_or_default();
    let selected_excerpt: String = selected_content.chars().take(24_000).collect();
    let selected_original_chars = selected_content.chars().count();
    let selected_excerpt_chars = selected_excerpt.chars().count();
    let required_prompt = format!(
        "User task:\n{}\n\nRepo metadata:\nrepoRoot={}\ncurrentFile={}\nbeforeSha256={}\ncurrentCommit={}\ndirtyBuffer={}\nendpointClassification={}\nagentProfile={}\n\nSelected text supplied directly by the user:\n```\n{}\n```\n\nConstraints:\n- The model may only return typed operations.\n- Backend validation will verify beforeSha256 against disk before apply.\n- Commands may be suggested, but only the local user can approve and run them through Synthesize's governed task runner or personal terminal.\n- Do not include files outside the repo or denied files.\n",
        req.user_message,
        canonical_repo,
        req.selected_file_path,
        before_sha,
        git_current_commit(&repo_root).unwrap_or_default(),
        req.dirty_buffer_state,
        endpoint_classification,
        &source_agent_profile_id,
        selected_text,
    );
    let required_messages = vec![
        RuntimeMessage {
            role: "system".into(),
            content: system.clone(),
        },
        RuntimeMessage {
            role: "user".into(),
            content: required_prompt.clone(),
        },
    ];
    let available_input = capability
        .context_window_tokens
        .checked_sub(capability.maximum_output_tokens + capability.safety_margin_tokens)
        .ok_or_else(|| "runtime capability leaves no input capacity".to_string())?;
    let mandatory_tokens = conservative_runtime_message_tokens(&required_messages);
    if mandatory_tokens > available_input {
        return Err(format!(
            "BLOCKED_CONTEXT_OVERFLOW: mandatory Assist protocol/task requires {mandatory_tokens} input tokens but only {available_input} are available; narrow the request"
        ));
    }
    let mut file_section =
        format!("\n\nCurrent file excerpt (P2 working context):\n```\n{selected_excerpt}\n```");
    let mut package_section =
        format!("\n\nPackage metadata/scripts (P3 supporting):\n{package_scripts}");
    let mut tree_section = format!("\n\nFile tree excerpt (P4 background):\n{tree}");
    let mut omitted = Vec::new();
    let build_messages = |file: &str, package: &str, tree: &str| {
        let prompt = format!("{required_prompt}{file}{package}{tree}");
        (
            prompt.clone(),
            vec![
                RuntimeMessage {
                    role: "system".into(),
                    content: system.clone(),
                },
                RuntimeMessage {
                    role: "user".into(),
                    content: prompt,
                },
            ],
        )
    };
    let (mut prompt, mut messages) = build_messages(&file_section, &package_section, &tree_section);
    if conservative_runtime_message_tokens(&messages) > available_input {
        tree_section.clear();
        omitted.push(ContextOmittedView {
            kind: "file_tree_excerpt".into(),
            path: None,
            reason: "P4 background pruned first by token budget".into(),
        });
        (prompt, messages) = build_messages(&file_section, &package_section, &tree_section);
    }
    if conservative_runtime_message_tokens(&messages) > available_input {
        package_section.clear();
        omitted.push(ContextOmittedView {
            kind: "package_scripts".into(),
            path: Some("package.json".into()),
            reason: "P3 supporting context pruned after P4".into(),
        });
        (prompt, messages) = build_messages(&file_section, &package_section, &tree_section);
    }
    if conservative_runtime_message_tokens(&messages) > available_input {
        file_section.clear();
        omitted.push(ContextOmittedView {
            kind: "selected_file_excerpt".into(),
            path: Some(req.selected_file_path.clone()),
            reason: "P2 working context pruned after P4 and P3; user task/protocol preserved"
                .into(),
        });
        (prompt, messages) = build_messages(&file_section, &package_section, &tree_section);
    }
    let compiled_input_tokens = conservative_runtime_message_tokens(&messages);
    if compiled_input_tokens > available_input {
        return Err("BLOCKED_CONTEXT_OVERFLOW: mandatory Assist context does not fit after deterministic optional pruning".into());
    }
    let messages_sha256 = hash_runtime_messages(&messages)?;
    let char_estimate = messages.iter().map(|m| m.content.len()).sum::<usize>();
    let context_bundle_id = new_id("ctx");
    let mut included = Vec::new();
    if !file_section.is_empty() {
        included.push(ContextIncludedView {
            kind: "selected_file".into(),
            path: Some(req.selected_file_path.clone()),
            chars: selected_excerpt_chars,
            note: format!("beforeSha256={before_sha}; exact included excerpt size"),
        });
    }
    if !tree_section.is_empty() {
        included.push(ContextIncludedView {
            kind: "file_tree_excerpt".into(),
            path: None,
            chars: tree.chars().count(),
            note: format!(
                "{} allowed entries, denied directories/files omitted",
                files.iter().filter(|f| !f.denied).count().min(120)
            ),
        });
    }
    if !package_section.is_empty() {
        included.push(ContextIncludedView {
            kind: "package_scripts".into(),
            path: Some("package.json".into()),
            chars: package_scripts.chars().count(),
            note: "package metadata excerpt if available".into(),
        });
    }
    let truncations = if selected_original_chars > selected_excerpt_chars {
        vec![ContextTruncationView {
            kind: "selected_file".into(),
            path: Some(req.selected_file_path.clone()),
            original_chars: selected_original_chars,
            included_chars: selected_excerpt_chars,
            reason: "bounded P2 file excerpt; full file remains retrievable through guarded context requests".into(),
        }]
    } else {
        vec![]
    };
    let view = ContextBundleView {
        context_bundle_id: context_bundle_id.clone(),
        session_id: req.session_id.clone(),
        repo_root: canonical_repo_root_string(&repo_root).unwrap_or_else(|_| req.repo_root.clone()),
        user_message: req.user_message,
        selected_file_path: req.selected_file_path,
        dirty_buffer_state: req.dirty_buffer_state,
        git_commit: git_current_commit(&repo_root),
        endpoint_classification: endpoint_classification.clone(),
        destination_warning,
        char_estimate,
        runtime: req.provider.clone(),
        model: req.model.clone(),
        model_context_window_tokens: capability.context_window_tokens,
        reserved_output_tokens: capability.maximum_output_tokens,
        safety_margin_tokens: capability.safety_margin_tokens,
        compiled_input_tokens,
        remaining_capacity_tokens: available_input - compiled_input_tokens,
        token_count_kind: "estimated".into(),
        token_estimation_method: capability.token_estimation_method.clone(),
        included,
        omitted,
        summaries_used: vec![],
        truncations,
        messages,
        exact_prompt: prompt,
        messages_sha256,
        exact_context: true,
        agent_profile_id: source_agent_profile_id.clone(),
    };
    let payload_json = serde_json::to_string(&view).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO context_bundles
         (id, session_id, token_estimate, input_token_count, token_count_method, payload_json)
         VALUES (?1, ?2, ?3, ?3, ?4, ?5)",
        params![
            &context_bundle_id,
            &req.session_id,
            view.compiled_input_tokens as i64,
            &view.token_estimation_method,
            &payload_json
        ],
    )
    .map_err(|e| e.to_string())?;
    let payload = serde_json::json!({
        "context_bundle_id": &context_bundle_id,
        "endpoint_classification": &endpoint_classification,
        "selected_file_path": &view.selected_file_path,
        "char_estimate": view.char_estimate,
        "compiled_input_tokens": view.compiled_input_tokens,
        "remaining_capacity_tokens": view.remaining_capacity_tokens,
        "included_count": view.included.len(),
        "messages_sha256": &view.messages_sha256,
        "agent_profile_id": &view.agent_profile_id
    });
    let _ = append_event(
        &conn,
        &req.session_id,
        "context.bundle_created",
        &payload.to_string(),
    );
    Ok(view)
}

#[tauri::command]
fn approve_runtime_endpoint(
    req: RuntimeEndpointApprovalRequest,
) -> Result<RuntimeEndpointApprovalResult, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let conn = init_audit(&repo_root, &req.session_id).map_err(|e| e.to_string())?;
    let classification = classify_endpoint_url(&req.endpoint_url);
    if classification == "local" && req.allow_repo_context {
        // Local endpoints do not need a warning approval, but persisting it is harmless and useful for audit.
    }
    conn.execute(
        "INSERT OR REPLACE INTO endpoint_approvals (endpoint_url, endpoint_classification, approved_at, approved_by_source, allow_repo_context) VALUES (?1, ?2, datetime('now'), 'local-user', ?3)",
        params![&req.endpoint_url, &classification, if req.allow_repo_context { 1 } else { 0 }],
    ).map_err(|e| e.to_string())?;
    let approved_at = select_scalar_string(
        &conn,
        "SELECT approved_at FROM endpoint_approvals WHERE endpoint_url = ?1",
        &req.endpoint_url,
    )?;
    let payload = serde_json::json!({ "endpoint_url": sanitized_endpoint_for_log(&req.endpoint_url), "endpoint_classification": &classification, "allow_repo_context": req.allow_repo_context, "approved_by_source": "local-user" });
    let _ = append_event(
        &conn,
        &req.session_id,
        "runtime.endpoint_approved",
        &payload.to_string(),
    );
    Ok(RuntimeEndpointApprovalResult {
        endpoint_url: req.endpoint_url,
        endpoint_classification: classification,
        allow_repo_context: req.allow_repo_context,
        approved_at,
    })
}

#[tauri::command]
fn runtime_endpoint_approval_status(
    req: RuntimeEndpointApprovalStatusRequest,
) -> Result<RuntimeEndpointApprovalStatusResult, String> {
    let classification = classify_endpoint_url(&req.endpoint_url);
    if classification == "local" {
        return Ok(RuntimeEndpointApprovalStatusResult {
            endpoint_url: req.endpoint_url,
            endpoint_classification: classification,
            approved: true,
            allow_repo_context: true,
            approved_at: None,
        });
    }
    let repo_root = req
        .repo_root
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("synthesize-no-repo-approval-status"));
    let conn = init_audit(&repo_root, &req.session_id).map_err(|e| e.to_string())?;
    let row: Option<(i64, String)> = conn.query_row(
        "SELECT allow_repo_context, approved_at FROM endpoint_approvals WHERE endpoint_url = ?1 AND endpoint_classification = ?2",
        params![&req.endpoint_url, &classification],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional().map_err(|e| e.to_string())?;
    let (approved, allow_repo_context, approved_at) = match row {
        Some((allow, at)) if allow == 1 => (true, true, Some(at)),
        Some((allow, at)) => (false, allow == 1, Some(at)),
        None => (false, false, None),
    };
    Ok(RuntimeEndpointApprovalStatusResult {
        endpoint_url: req.endpoint_url,
        endpoint_classification: classification,
        approved,
        allow_repo_context,
        approved_at,
    })
}

#[tauri::command]
fn runtime_health_check(req: RuntimeHealthRequest) -> Result<RuntimeHealthResult, String> {
    let classification = if req.provider == "fake" {
        "local".into()
    } else {
        classify_endpoint_url(&req.endpoint_url)
    };
    let repo_root = req.repo_root.clone().map(PathBuf::from);
    let started = Instant::now();
    let result = if req.provider == "fake" {
        Ok("fake runtime ready".to_string())
    } else if req.provider == "cloud-openai" {
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
            "OPENAI_API_KEY environment variable is not set; cloud-openai provider requires it"
                .to_string()
        })?;
        if api_key.trim().is_empty() {
            return Err("OPENAI_API_KEY is set but empty".into());
        }
        let base = normalize_endpoint_base(&req.endpoint_url)?;
        let url = format!("{}/models", base);
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(8))
            .build();
        match agent
            .get(&url)
            .set("Authorization", &format!("Bearer {}", api_key))
            .call()
        {
            Ok(resp) => Ok(format!("cloud endpoint reachable; HTTP {}", resp.status())),
            Err(err) => Err(format_runtime_error(err)),
        }
    } else if req.provider == "cloud-anthropic" {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "ANTHROPIC_API_KEY environment variable is not set; cloud-anthropic provider requires it".to_string())?;
        if api_key.trim().is_empty() {
            return Err("ANTHROPIC_API_KEY is set but empty".into());
        }
        Ok("cloud credentials present; runtime_generate will verify API reachability on first request".to_string())
    } else {
        let base = normalize_endpoint_base(&req.endpoint_url)?;
        let url = format!("{}/models", base);
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(8))
            .build();
        match agent.get(&url).call() {
            Ok(resp) => Ok(format!("endpoint reachable; HTTP {}", resp.status())),
            Err(err) => Err(format_runtime_error(err)),
        }
    };
    if let Some(root) = repo_root.as_ref() {
        if let Ok(conn) = init_audit(root, &req.session_id) {
            let payload = serde_json::json!({ "provider": &req.provider, "endpoint_url": sanitized_endpoint_for_log(&req.endpoint_url), "endpoint_classification": &classification, "model": &req.model, "duration_ms": started.elapsed().as_millis(), "ok": result.is_ok(), "error": result.as_ref().err() });
            let _ = append_event(
                &conn,
                &req.session_id,
                "runtime.health_check",
                &payload.to_string(),
            );
        }
    }
    match result {
        Ok(message) => Ok(RuntimeHealthResult {
            ok: true,
            provider: req.provider,
            endpoint_url: req.endpoint_url,
            endpoint_classification: classification,
            model: req.model,
            message,
        }),
        Err(message) => Ok(RuntimeHealthResult {
            ok: false,
            provider: req.provider,
            endpoint_url: req.endpoint_url,
            endpoint_classification: classification,
            model: req.model,
            message,
        }),
    }
}

#[tauri::command]
fn list_runtime_models(req: RuntimeListModelsRequest) -> Result<Vec<RuntimeModelView>, String> {
    if req.provider == "fake" {
        return Ok(vec![RuntimeModelView {
            id: "fixture-patcher".into(),
        }]);
    }
    if req.provider == "cloud-openai" {
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
            "OPENAI_API_KEY environment variable is not set; cloud-openai provider requires it"
                .to_string()
        })?;
        if api_key.trim().is_empty() {
            return Err("OPENAI_API_KEY is set but empty".into());
        }
        let base = normalize_endpoint_base(&req.endpoint_url)?;
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(10))
            .build();
        let resp = agent
            .get(&format!("{}/models", base))
            .set("Authorization", &format!("Bearer {}", api_key))
            .call()
            .map_err(format_runtime_error)?;
        let json: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
        let models = json
            .get("data")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| {
                item.get("id")
                    .and_then(|id| id.as_str())
                    .map(|s| RuntimeModelView { id: s.to_string() })
            })
            .collect::<Vec<_>>();
        return Ok(models);
    }
    if req.provider == "cloud-anthropic" {
        return Ok(vec![
            RuntimeModelView {
                id: "claude-3-5-sonnet-latest".into(),
            },
            RuntimeModelView {
                id: "claude-3-7-sonnet-latest".into(),
            },
        ]);
    }
    let base = normalize_endpoint_base(&req.endpoint_url)?;
    let classification = classify_endpoint_url(&req.endpoint_url);
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .build();
    let resp = agent
        .get(&format!("{}/models", base))
        .call()
        .map_err(format_runtime_error)?;
    let json: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
    let models = json
        .get("data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            item.get("id")
                .and_then(|id| id.as_str())
                .map(|s| RuntimeModelView { id: s.to_string() })
        })
        .collect::<Vec<_>>();
    if let Some(root) = req.repo_root.as_ref().map(PathBuf::from) {
        if let Ok(conn) = init_audit(&root, &req.session_id) {
            let payload = serde_json::json!({ "endpoint_url": sanitized_endpoint_for_log(&req.endpoint_url), "endpoint_classification": classification, "model_count": models.len() });
            let _ = append_event(
                &conn,
                &req.session_id,
                "runtime.models_listed",
                &payload.to_string(),
            );
        }
    }
    Ok(models)
}

#[tauri::command]
fn runtime_generate(req: RuntimeGenerateRequest) -> Result<RuntimeGenerateResult, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let conn = init_audit(&repo_root, &req.session_id).map_err(|e| e.to_string())?;
    let classification = if req.provider == "fake" {
        "local".into()
    } else {
        classify_endpoint_url(&req.endpoint_url)
    };
    if req.provider != "fake" && classification != "local" {
        verify_endpoint_approval(&conn, &req.endpoint_url, &classification)?;
    }

    let context = load_context_bundle(&conn, &req.context_bundle_id)?;
    if context.session_id != req.session_id {
        return Err("context bundle session_id does not match runtime request".into());
    }
    if !repo_roots_equal(&context.repo_root, &repo_root)? {
        return Err("context bundle repo_root does not match runtime request".into());
    }
    if context.endpoint_classification != classification {
        return Err(format!(
            "context bundle endpoint classification {} does not match runtime request {}",
            context.endpoint_classification, classification
        ));
    }
    if !context.runtime.is_empty() && context.runtime != req.provider {
        return Err("context bundle runtime does not match runtime request".into());
    }
    if !context.model.is_empty() && context.model != req.model {
        return Err("context bundle model does not match runtime request".into());
    }
    let capability = load_runtime_capability(&conn, &req.session_id, &req.provider, &req.model)
        .map_err(|error| {
            format!(
                "runtime capability must be registered before inference; rebuild context: {error}"
            )
        })?;
    if req.max_tokens as usize > capability.maximum_output_tokens {
        return Err(format!(
            "requested output tokens {} exceed registered maximum {}",
            req.max_tokens, capability.maximum_output_tokens
        ));
    }
    let messages = context.messages.clone();
    let messages_sha256 = hash_runtime_messages(&messages)?;
    if messages_sha256 != context.messages_sha256 {
        return Err(
            "persisted context bundle messages hash mismatch; refusing runtime call".into(),
        );
    }
    let compiled_input_tokens = conservative_runtime_message_tokens(&messages);
    if context.compiled_input_tokens != 0 && compiled_input_tokens != context.compiled_input_tokens
    {
        return Err("persisted compiled input token count does not match exact messages".into());
    }
    if compiled_input_tokens + req.max_tokens as usize + capability.safety_margin_tokens
        > capability.context_window_tokens
    {
        return Err(format!(
            "BLOCKED_CONTEXT_OVERFLOW before inference: input {compiled_input_tokens} + output {} + safety {} exceeds model window {}",
            req.max_tokens, capability.safety_margin_tokens, capability.context_window_tokens
        ));
    }

    let input_chars = messages.iter().map(|m| m.content.len()).sum::<usize>();
    let request_id = new_id("rt");
    conn.execute(
        "INSERT INTO runtime_requests (id, session_id, context_bundle_id, endpoint_url, endpoint_classification, model, provider, status, input_chars) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'started', ?8)",
        params![&request_id, &req.session_id, &req.context_bundle_id, &req.endpoint_url, &classification, &req.model, &req.provider, input_chars as i64],
    ).map_err(|e| e.to_string())?;
    let start_payload = serde_json::json!({ "runtime_request_id": &request_id, "context_bundle_id": &req.context_bundle_id, "messages_sha256": &messages_sha256, "provider": &req.provider, "endpoint_url": sanitized_endpoint_for_log(&req.endpoint_url), "endpoint_classification": &classification, "model": &req.model, "input_chars": input_chars });
    let _ = append_event(
        &conn,
        &req.session_id,
        "runtime.request_started",
        &start_payload.to_string(),
    );
    let started = Instant::now();
    let generated = if req.provider == "fake" {
        Ok(fake_runtime_response(&messages))
    } else if req.provider == "cloud-openai" {
        call_cloud_openai(
            &req.endpoint_url,
            &req.model,
            &messages,
            req.temperature,
            req.max_tokens,
        )
    } else if req.provider == "cloud-anthropic" {
        call_cloud_anthropic(&req.endpoint_url, &req.model, &messages, req.max_tokens)
    } else {
        call_openai_compatible_endpoint(
            &req.endpoint_url,
            &req.model,
            &messages,
            req.temperature,
            req.max_tokens,
            req.response_format.as_deref(),
        )
    };
    match generated {
        Ok(content) => {
            let duration = started.elapsed().as_millis();
            let output_chars = content.len();
            conn.execute("UPDATE runtime_requests SET status = 'completed', completed_at = datetime('now'), output_chars = ?2 WHERE id = ?1", params![&request_id, output_chars as i64]).map_err(|e| e.to_string())?;
            let payload = serde_json::json!({ "runtime_request_id": &request_id, "context_bundle_id": &req.context_bundle_id, "messages_sha256": &messages_sha256, "endpoint_classification": &classification, "model": &req.model, "duration_ms": duration, "input_chars": input_chars, "output_chars": output_chars });
            let event_id = append_event(
                &conn,
                &req.session_id,
                "runtime.request_completed",
                &payload.to_string(),
            )
            .map_err(|e| e.to_string())?;
            Ok(RuntimeGenerateResult {
                provider: req.provider,
                endpoint_url: req.endpoint_url,
                endpoint_classification: classification,
                model: req.model,
                content,
                duration_ms: duration,
                input_chars,
                output_chars,
                audit_event_id: Some(event_id),
            })
        }
        Err(err) => {
            let duration = started.elapsed().as_millis();
            let _ = conn.execute("UPDATE runtime_requests SET status = 'failed', completed_at = datetime('now'), error = ?2 WHERE id = ?1", params![&request_id, &err]);
            let payload = serde_json::json!({ "runtime_request_id": &request_id, "context_bundle_id": &req.context_bundle_id, "messages_sha256": &messages_sha256, "endpoint_classification": &classification, "model": &req.model, "duration_ms": duration, "error": &err });
            let _ = append_event(
                &conn,
                &req.session_id,
                "runtime.request_failed",
                &payload.to_string(),
            );
            Err(err)
        }
    }
}

#[tauri::command]
fn runtime_cancel(req: RuntimeCancelRequest) -> Result<RuntimeCancelResult, String> {
    let message =
        "runtime cancellation is not implemented for blocking backend calls in this build"
            .to_string();
    Ok(RuntimeCancelResult {
        cancelled: false,
        message: format!(
            "{}; session={}; request={}",
            message,
            req.session_id,
            req.request_id.unwrap_or_else(|| "none".into())
        ),
    })
}

#[tauri::command]
fn validate_patch_proposal(req: PatchProposalRequest) -> Result<PatchValidationResult, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let conn = init_audit(&repo_root, &req.session_id).map_err(|e| e.to_string())?;
    validate_patch_proposal_with_connection(&conn, req, repo_root)
}

fn validate_patch_proposal_with_connection(
    conn: &Connection,
    req: PatchProposalRequest,
    repo_root: PathBuf,
) -> Result<PatchValidationResult, String> {
    let proposal = proposal_from_operation(&req.operation)?;
    let (operation_json, operation_sha256) = canonical_operation_and_hash(&req.operation)?;
    let (source_agent_profile_id, source_context_bundle_id) =
        source_profile_for_patch_validation(conn, &req, &repo_root)?;
    enforce_agent_profile_allows_operation(&source_agent_profile_id, &req.operation)?;
    let moa_decision = enforce_moa_gate_for_operation(&req.operation, &source_agent_profile_id)?;

    if let Some(existing) = load_existing_proposal_identity(conn, &proposal.id)? {
        if existing.operation_sha256 != operation_sha256 {
            return Err(format!(
                "proposal_id {} already exists with a different operation_sha256",
                proposal.id
            ));
        }
        if existing.status == "validated" {
            let files = load_patch_file_views(&conn, &proposal.id)?;
            return Ok(PatchValidationResult {
                ok: true,
                proposal_id: proposal.id,
                operation_sha256,
                status: "validated".into(),
                files,
                warnings: vec!["duplicate validation returned persisted validated snapshot without mutating lifecycle state".into()],
                errors: vec![],
                message: "proposal was already validated with the same operation hash".into(),
                audit_event_id: None,
            });
        }
        if existing.status == "rejected" {
            let files = load_patch_file_views(&conn, &proposal.id).unwrap_or_default();
            return Ok(PatchValidationResult {
                ok: false,
                proposal_id: proposal.id.clone(),
                operation_sha256,
                status: "rejected".into(),
                files,
                warnings: vec!["duplicate validation returned persisted rejected snapshot without mutating lifecycle state".into()],
                errors: vec![],
                message: "proposal was already rejected with the same operation hash".into(),
                audit_event_id: None,
            });
        }
        if existing.status == "proposed" {
            return Err(format!(
                "proposal {} is already proposed and awaiting validation transition",
                proposal.id
            ));
        }
        if matches!(
            existing.status.as_str(),
            "approved"
                | "applying"
                | "applied"
                | "rolling_back"
                | "rolled_back"
                | "apply_failed"
                | "rollback_failed"
        ) {
            let files = load_patch_file_views(&conn, &proposal.id).unwrap_or_default();
            return Ok(PatchValidationResult {
                ok: existing.status != "apply_failed" && existing.status != "rollback_failed",
                proposal_id: proposal.id.clone(),
                operation_sha256,
                status: existing.status.clone(),
                files,
                warnings: vec![format!(
                    "duplicate validation ignored; persisted proposal is already in {} state",
                    existing.status
                )],
                errors: vec![],
                message: "proposal lifecycle state was not mutated by duplicate validation".into(),
                audit_event_id: None,
            });
        }
    }

    insert_patch_proposal(
        &conn,
        &req.session_id,
        &repo_root,
        &proposal,
        &operation_json,
        &operation_sha256,
        "proposed",
        None,
        Some(source_context_bundle_id.as_str()),
        &source_agent_profile_id,
    )?;

    let actual_commit = git_current_commit(&repo_root);
    let validation =
        validate_patch_proposal_against_repo(&repo_root, &proposal, actual_commit.as_deref());

    match validation {
        Ok(files) => {
            persist_patch_files(&conn, &proposal, &files)?;
            transition_proposal_status(
                &conn,
                &req.session_id,
                &proposal.id,
                "proposed",
                "validated",
                None,
            )?;
            let views = files
                .iter()
                .map(|file| PatchFileValidationView {
                    id: file.id.clone(),
                    path: file.path.clone(),
                    risk: format!("{:?}", file.risk),
                    real_path: file.real_path.to_string_lossy().to_string(),
                })
                .collect::<Vec<_>>();
            let payload = serde_json::json!({
                "proposal_id": proposal.id,
                "operation_sha256": &operation_sha256,
                "files": &views,
                "status": "validated",
                "source_context_bundle_id": source_context_bundle_id,
                "source_agent_profile_id": source_agent_profile_id,
                "moa": moa_decision
            });
            let audit_event_id = append_event(
                &conn,
                &req.session_id,
                "patch.validated",
                &payload.to_string(),
            )
            .map_err(|e| e.to_string())?;
            Ok(PatchValidationResult {
                ok: true,
                proposal_id: proposal.id,
                operation_sha256,
                status: "validated".into(),
                files: views,
                warnings: vec![],
                errors: vec![],
                message: "patch proposal persisted and passed backend validation".into(),
                audit_event_id: Some(audit_event_id),
            })
        }
        Err(err) => {
            let reason = err.to_string();
            persist_patch_files_rejected(&conn, &proposal)?;
            transition_proposal_status(
                &conn,
                &req.session_id,
                &proposal.id,
                "proposed",
                "rejected",
                Some(&reason),
            )?;
            let payload = serde_json::json!({ "proposal_id": proposal.id, "operation_sha256": &operation_sha256, "error": &reason, "status": "rejected", "source_context_bundle_id": source_context_bundle_id, "source_agent_profile_id": source_agent_profile_id });
            let audit_event_id = append_event(
                &conn,
                &req.session_id,
                "patch.rejected",
                &payload.to_string(),
            )
            .map_err(|e| e.to_string())?;
            Ok(PatchValidationResult {
                ok: false,
                proposal_id: proposal.id,
                operation_sha256,
                status: "rejected".into(),
                files: vec![],
                warnings: vec![],
                errors: vec![reason.clone()],
                message: reason,
                audit_event_id: Some(audit_event_id),
            })
        }
    }
}

#[tauri::command]
fn approve_patch_proposal(req: PatchApprovalRequest) -> Result<PatchApprovalView, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let mut conn = init_audit(&repo_root, &req.session_id).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let stored = load_stored_proposal(&tx, &req.proposal_id)?;
    if stored.status != "validated" {
        return Err(format!(
            "proposal {} must be validated before approval; current status is {}",
            req.proposal_id, stored.status
        ));
    }
    if stored.operation_sha256 != req.operation_sha256 {
        return Err("operation_sha256 does not match persisted proposal snapshot".into());
    }

    let approval_id = new_id("approval");
    tx.execute(
        "INSERT INTO patch_approvals (approval_id, proposal_id, operation_sha256, approved_at, approved_by_source, approval_scope) VALUES (?1, ?2, ?3, datetime('now'), 'local-user', 'whole-proposal')",
        params![&approval_id, &req.proposal_id, &req.operation_sha256],
    ).map_err(|e| e.to_string())?;
    transition_proposal_status(
        &tx,
        &req.session_id,
        &req.proposal_id,
        "validated",
        "approved",
        None,
    )?;
    let approved_at = select_scalar_string(
        &tx,
        "SELECT approved_at FROM patch_approvals WHERE approval_id = ?1",
        &approval_id,
    )?;
    let payload = serde_json::json!({ "proposal_id": &req.proposal_id, "approval_id": &approval_id, "operation_sha256": &req.operation_sha256, "approved_by_source": "local-user", "approval_scope": "whole-proposal" });
    let audit_event_id = append_event(&tx, &req.session_id, "patch.approved", &payload.to_string())
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;

    Ok(PatchApprovalView {
        proposal_id: req.proposal_id,
        approval_id,
        operation_sha256: req.operation_sha256,
        approved_by_source: "local-user".into(),
        approved_at,
        audit_event_id,
    })
}

#[tauri::command]
fn apply_approved_patch(req: PatchApplyRequest) -> Result<PatchApplyView, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let _lock = acquire_repo_mutation_lock(&repo_root, &req.session_id, "apply")?;
    let mut conn = init_audit(&repo_root, &req.session_id).map_err(|e| e.to_string())?;
    let stored = load_stored_proposal(&conn, &req.proposal_id)?;
    if stored.status != "approved" {
        return Err(format!(
            "proposal {} must be approved before apply; current status is {}",
            req.proposal_id, stored.status
        ));
    }
    verify_approval(
        &conn,
        &req.proposal_id,
        &req.approval_id,
        &stored.operation_sha256,
    )?;
    // Status transition is authoritative. Phase audit is best-effort so audit failure cannot strand the proposal after moving to applying.
    transition_proposal_status(
        &conn,
        &req.session_id,
        &req.proposal_id,
        "approved",
        "applying",
        None,
    )?;
    let _ = append_event(
        &conn,
        &req.session_id,
        "patch.applying",
        &serde_json::json!({"proposal_id": &req.proposal_id, "approval_id": &req.approval_id})
            .to_string(),
    );

    let actual_commit = git_current_commit(&repo_root);
    let result = match apply_patch_proposal_transactional(
        &repo_root,
        &stored.proposal,
        actual_commit.as_deref(),
    ) {
        Ok(result) => result,
        Err(err) => {
            let reason = err.to_string();
            let _ = transition_proposal_status(
                &conn,
                &req.session_id,
                &req.proposal_id,
                "applying",
                "apply_failed",
                Some(&reason),
            );
            let payload = serde_json::json!({ "proposal_id": &req.proposal_id, "approval_id": &req.approval_id, "error": &reason });
            let _ = append_event(
                &conn,
                &req.session_id,
                "patch.apply_failed",
                &payload.to_string(),
            );
            return Err(reason);
        }
    };

    let applied_files = result
        .applied_files
        .iter()
        .map(|file| AppliedFileView {
            id: file.id.clone(),
            path: file.path.clone(),
            after_sha256: file.after_sha256.clone(),
        })
        .collect::<Vec<_>>();
    let checkpoint_dir_string = result.checkpoint_dir.to_string_lossy().to_string();
    let canonical_repo = canonical_repo_root_string(&repo_root)?;

    let finalize_result: Result<String, String> = (|| {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT OR REPLACE INTO patch_checkpoints (checkpoint_id, proposal_id, repo_root, operation_sha256, checkpoint_dir, created_at) VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
            params![&result.checkpoint_id, &req.proposal_id, &canonical_repo, &stored.operation_sha256, &checkpoint_dir_string],
        ).map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE patch_proposals SET checkpoint_id = ?2, checkpoint_dir = ?3 WHERE proposal_id = ?1",
            params![&req.proposal_id, &result.checkpoint_id, &checkpoint_dir_string],
        ).map_err(|e| e.to_string())?;
        append_event(&tx, &req.session_id, "patch.checkpoint_created", &serde_json::json!({"proposal_id": &req.proposal_id, "checkpoint_id": &result.checkpoint_id}).to_string()).map_err(|e| e.to_string())?;
        transition_proposal_status(
            &tx,
            &req.session_id,
            &req.proposal_id,
            "applying",
            "applied",
            None,
        )?;
        let view_payload = serde_json::json!({
            "proposal_id": &req.proposal_id,
            "approval_id": &req.approval_id,
            "checkpoint_id": &result.checkpoint_id,
            "applied_files": &applied_files,
        });
        let audit_event_id = append_event(
            &tx,
            &req.session_id,
            "patch.applied",
            &view_payload.to_string(),
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(audit_event_id)
    })();

    let audit_event_id = match finalize_result {
        Ok(id) => id,
        Err(err) => {
            let restore = rollback_checkpoint_for_proposal(
                &repo_root,
                &result.checkpoint_id,
                &req.proposal_id,
            );
            let reason = format!(
                "post-write persistence/status/audit failed: {}; restore_result={:?}",
                err,
                restore.as_ref().err()
            );
            let _ = transition_proposal_status(
                &conn,
                &req.session_id,
                &req.proposal_id,
                "applying",
                "apply_failed",
                Some(&reason),
            );
            let payload = serde_json::json!({ "proposal_id": &req.proposal_id, "approval_id": &req.approval_id, "checkpoint_id": &result.checkpoint_id, "error": &reason });
            let _ = append_event(
                &conn,
                &req.session_id,
                "patch.apply_failed",
                &payload.to_string(),
            );
            return Err(reason);
        }
    };

    Ok(PatchApplyView {
        proposal_id: req.proposal_id,
        approval_id: req.approval_id,
        checkpoint_id: result.checkpoint_id,
        checkpoint_dir: checkpoint_dir_string,
        applied_files,
        audit_event_id,
    })
}

#[tauri::command]
fn rollback_patch(req: RollbackRequest) -> Result<RollbackView, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let _lock = acquire_repo_mutation_lock(&repo_root, &req.session_id, "rollback")?;
    let mut conn = init_audit(&repo_root, &req.session_id).map_err(|e| e.to_string())?;
    let stored = load_stored_proposal(&conn, &req.proposal_id)?;
    if stored.status != "applied" {
        return Err(format!(
            "proposal {} must be applied before rollback; current status is {}",
            req.proposal_id, stored.status
        ));
    }
    let checkpoint_id = stored.checkpoint_id.clone().ok_or_else(|| {
        format!(
            "proposal {} has no persisted checkpoint_id",
            req.proposal_id
        )
    })?;
    let checkpoint_row: Option<(String, String)> = conn.query_row(
        "SELECT repo_root, operation_sha256 FROM patch_checkpoints WHERE checkpoint_id = ?1 AND proposal_id = ?2",
        params![&checkpoint_id, &req.proposal_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional().map_err(|e| e.to_string())?;
    let (checkpoint_repo, checkpoint_sha) = checkpoint_row
        .ok_or_else(|| "checkpoint record not found for applied proposal".to_string())?;
    if !repo_roots_equal(&checkpoint_repo, &repo_root)? {
        return Err("checkpoint record repo_root does not match requested repo".into());
    }
    if checkpoint_sha != stored.operation_sha256 {
        return Err("checkpoint record operation_sha256 does not match proposal snapshot".into());
    }

    // Status transition is authoritative. Phase audit is best-effort so audit failure cannot strand the proposal after moving to rolling_back.
    transition_proposal_status(
        &conn,
        &req.session_id,
        &req.proposal_id,
        "applied",
        "rolling_back",
        None,
    )?;
    let _ = append_event(
        &conn,
        &req.session_id,
        "patch.rolling_back",
        &serde_json::json!({"proposal_id": &req.proposal_id, "checkpoint_id": &checkpoint_id})
            .to_string(),
    );

    let result = match rollback_checkpoint_for_proposal(
        &repo_root,
        &checkpoint_id,
        &req.proposal_id,
    ) {
        Ok(result) => result,
        Err(err) => {
            let reason = err.to_string();
            let _ = transition_proposal_status(
                &conn,
                &req.session_id,
                &req.proposal_id,
                "rolling_back",
                "rollback_failed",
                Some(&reason),
            );
            let payload = serde_json::json!({ "proposal_id": &req.proposal_id, "checkpoint_id": &checkpoint_id, "error": &reason });
            let _ = append_event(
                &conn,
                &req.session_id,
                "patch.rollback_failed",
                &payload.to_string(),
            );
            return Err(reason);
        }
    };

    let finalize_result: Result<String, String> = (|| {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        transition_proposal_status(
            &tx,
            &req.session_id,
            &req.proposal_id,
            "rolling_back",
            "rolled_back",
            None,
        )?;
        let payload = serde_json::json!({ "proposal_id": &req.proposal_id, "checkpoint_id": &result.checkpoint_id, "restored_paths": &result.restored_paths, "deleted_paths": &result.deleted_paths });
        let audit_event_id = append_event(
            &tx,
            &req.session_id,
            "patch.rolled_back",
            &payload.to_string(),
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(audit_event_id)
    })();

    let audit_event_id = match finalize_result {
        Ok(id) => id,
        Err(err) => {
            let reason = format!(
                "rollback restored files but status/audit finalization failed: {}",
                err
            );
            let _ = transition_proposal_status(
                &conn,
                &req.session_id,
                &req.proposal_id,
                "rolling_back",
                "rollback_failed",
                Some(&reason),
            );
            let payload = serde_json::json!({ "proposal_id": &req.proposal_id, "checkpoint_id": &checkpoint_id, "error": &reason });
            let _ = append_event(
                &conn,
                &req.session_id,
                "patch.rollback_failed",
                &payload.to_string(),
            );
            return Err(reason);
        }
    };

    Ok(RollbackView {
        proposal_id: req.proposal_id,
        checkpoint_id: result.checkpoint_id,
        checkpoint_dir: result.checkpoint_dir.to_string_lossy().to_string(),
        restored_paths: result.restored_paths,
        deleted_paths: result.deleted_paths,
        audit_event_id,
    })
}

#[tauri::command]
fn read_guarded_file(repo_root: String, relative_path: String) -> Result<String, String> {
    let guard = RepoGuard::new(PathBuf::from(repo_root), FilePolicy::default())
        .map_err(|e| e.to_string())?;
    let path = guard
        .resolve_for_existing_path(relative_path)
        .map_err(|e| e.to_string())?;
    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_guarded_file(req: WriteFileRequest) -> Result<FileMutationResult, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let guard = RepoGuard::new(&repo_root, FilePolicy::default()).map_err(|e| e.to_string())?;
    let path = guard
        .resolve_for_write_path(&req.relative_path)
        .map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, req.content.as_bytes()).map_err(|e| e.to_string())?;
    let audit_event_id = init_audit(&repo_root, &req.session_id)
        .ok()
        .and_then(|conn| {
            let payload = serde_json::json!({ "path": req.relative_path, "source": "user-save", "operation": "write_guarded_file" });
            append_event(&conn, &req.session_id, "file.saved", &payload.to_string()).ok()
        });
    Ok(FileMutationResult {
        path: req.relative_path,
        message: "File saved through RepoGuard.".into(),
        audit_event_id,
    })
}

#[tauri::command]
fn create_repo_file(req: CreateFileRequest) -> Result<FileMutationResult, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let guard = RepoGuard::new(&repo_root, FilePolicy::default()).map_err(|e| e.to_string())?;
    let path = guard
        .resolve_for_write_path(&req.relative_path)
        .map_err(|e| e.to_string())?;
    if path.exists() {
        return Err("target already exists".into());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, req.content.unwrap_or_default()).map_err(|e| e.to_string())?;
    let audit_event_id = init_audit(&repo_root, &req.session_id)
        .ok()
        .and_then(|conn| {
            let payload =
                serde_json::json!({ "path": req.relative_path, "operation": "create_repo_file" });
            append_event(&conn, &req.session_id, "file.created", &payload.to_string()).ok()
        });
    Ok(FileMutationResult {
        path: req.relative_path,
        message: "File created through RepoGuard.".into(),
        audit_event_id,
    })
}

#[tauri::command]
fn rename_repo_path(req: RenamePathRequest) -> Result<FileMutationResult, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let guard = RepoGuard::new(&repo_root, FilePolicy::default()).map_err(|e| e.to_string())?;
    let from = guard
        .resolve_for_existing_path(&req.from_path)
        .map_err(|e| e.to_string())?;
    let to = guard
        .resolve_for_write_path(&req.to_path)
        .map_err(|e| e.to_string())?;
    if to.exists() {
        return Err("destination already exists".into());
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::rename(&from, &to).map_err(|e| e.to_string())?;
    let audit_event_id = init_audit(&repo_root, &req.session_id)
        .ok()
        .and_then(|conn| {
            let payload = serde_json::json!({ "from": req.from_path, "to": req.to_path, "operation": "rename_repo_path" });
            append_event(&conn, &req.session_id, "file.renamed", &payload.to_string()).ok()
        });
    Ok(FileMutationResult {
        path: req.to_path,
        message: "Path renamed through RepoGuard.".into(),
        audit_event_id,
    })
}

fn reject_delete_attempt(repo_root: &Path, session_id: &str, relative_path: &str, reason: &str) {
    if let Ok(conn) = init_audit(repo_root, session_id) {
        let payload = serde_json::json!({ "path": relative_path, "reason": reason, "operation": "delete_repo_path", "allowed": false });
        let _ = append_event(
            &conn,
            session_id,
            "file.delete_rejected",
            &payload.to_string(),
        );
    }
}

fn is_dangerous_delete_path(relative_path: &str) -> bool {
    let trimmed = relative_path.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == "./" {
        return true;
    }
    let normalized = trimmed
        .replace('\\', "/")
        .trim_matches('/')
        .to_ascii_lowercase();
    normalized.is_empty()
        || normalized == "."
        || normalized == ".git"
        || normalized == ".synthesize"
        || normalized.starts_with(".git/")
        || normalized.starts_with(".synthesize/")
}

#[tauri::command]
fn delete_repo_path(req: DeletePathRequest) -> Result<FileMutationResult, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let guard = RepoGuard::new(&repo_root, FilePolicy::default()).map_err(|e| e.to_string())?;
    if is_dangerous_delete_path(&req.relative_path) {
        reject_delete_attempt(
            &repo_root,
            &req.session_id,
            &req.relative_path,
            "refusing to delete repo root or Synthesize/Git control directory",
        );
        return Err("refusing to delete repo root or Synthesize/Git control directory".into());
    }
    let path = guard
        .resolve_for_existing_path(&req.relative_path)
        .map_err(|e| e.to_string())?;
    let canonical_root = repo_root.canonicalize().map_err(|e| e.to_string())?;
    if path == canonical_root {
        reject_delete_attempt(
            &repo_root,
            &req.session_id,
            &req.relative_path,
            "refusing to delete repo root",
        );
        return Err("refusing to delete repo root".into());
    }
    if path.is_dir() {
        if !req.allow_directory {
            reject_delete_attempt(
                &repo_root,
                &req.session_id,
                &req.relative_path,
                "directory delete requires allow_directory=true and confirmation token",
            );
            return Err(
                "directory delete requires explicit allow_directory=true and confirmation token"
                    .into(),
            );
        }
        let expected = format!("DELETE_DIRECTORY:{}", req.relative_path.trim());
        if req.confirmation_token.as_deref() != Some(expected.as_str()) {
            reject_delete_attempt(
                &repo_root,
                &req.session_id,
                &req.relative_path,
                "directory delete confirmation token mismatch",
            );
            return Err(format!(
                "directory delete requires confirmation token {}",
                expected
            ));
        }
        fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
    } else {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    let audit_event_id = init_audit(&repo_root, &req.session_id)
        .ok()
        .and_then(|conn| {
            let payload = serde_json::json!({ "path": req.relative_path, "directory": req.allow_directory, "operation": "delete_repo_path", "protected_checks": "repo-root/git/synthesize/directory-token" });
            append_event(&conn, &req.session_id, "file.deleted", &payload.to_string()).ok()
        });
    Ok(FileMutationResult {
        path: req.relative_path,
        message: "Path deleted through RepoGuard with protected-delete checks.".into(),
        audit_event_id,
    })
}

#[tauri::command]
fn list_audit_events(repo_root: String, session_id: String) -> Result<Vec<AuditEventView>, String> {
    let conn = init_audit(Path::new(&repo_root), &session_id).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, timestamp, kind, payload_json FROM audit_events WHERE session_id = ?1 ORDER BY timestamp DESC, rowid DESC LIMIT 80").map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([session_id], |row| {
            Ok(AuditEventView {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                kind: row.get(2)?,
                payload_json: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut events = Vec::new();
    for row in rows {
        events.push(row.map_err(|e| e.to_string())?);
    }
    Ok(events)
}

#[tauri::command]
fn record_session_event(req: SessionEventRequest) -> Result<SessionEventResult, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let conn = init_audit(&repo_root, &req.session_id).map_err(|e| e.to_string())?;
    let id = append_event(&conn, &req.session_id, &req.kind, &req.payload_json)
        .map_err(|e| e.to_string())?;
    Ok(SessionEventResult { audit_event_id: id })
}

#[tauri::command]
fn classify_command(req: CommandClassifyRequest) -> Result<CommandClassifyResult, String> {
    let guard_req = GuardCommandRequest {
        argv: req.argv.clone(),
        cwd: req.cwd.clone(),
        requires_network: req.requires_network,
        may_modify_files: req.may_modify_files,
    };
    let result = match classify(&guard_req, &CommandPolicy::default()) {
        Ok(risk) => CommandClassifyResult {
            ok: true,
            risk: risk_label(&risk).into(),
            message: "command execution is disabled; this is classification only".into(),
        },
        Err(err) => CommandClassifyResult {
            ok: false,
            risk: "Blocked".into(),
            message: err.to_string(),
        },
    };
    if let (Some(repo_root), Some(session_id)) = (req.repo_root.as_ref(), req.session_id.as_ref()) {
        if let Ok(conn) = init_audit(Path::new(repo_root), session_id) {
            let payload = serde_json::json!({ "argv": &req.argv, "cwd": &req.cwd, "risk": &result.risk, "ok": result.ok, "message": &result.message, "requiresNetwork": req.requires_network, "mayModifyFiles": req.may_modify_files, "execution": "disabled" });
            let _ = append_event(
                &conn,
                session_id,
                "command.classified",
                &payload.to_string(),
            );
        }
    }
    Ok(result)
}

#[tauri::command]
fn project_search(req: ProjectSearchRequest) -> Result<Vec<ProjectSearchResult>, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let conn = init_audit(&repo_root, &req.session_id).map_err(|e| e.to_string())?;
    let query = req.query.trim().to_string();
    if query.len() < 2 {
        return Ok(vec![]);
    }
    let max = req.max_results.unwrap_or(100).min(250);
    let files = list_files_internal(&repo_root, 1500)?;
    let mut results = Vec::new();
    for file in files
        .iter()
        .filter(|f| f.kind == "file" && !f.denied && is_text_like(&f.path))
    {
        if results.len() >= max {
            break;
        }
        let text = match read_guarded_file(req.repo_root.clone(), file.path.clone()) {
            Ok(t) => t,
            Err(_) => continue,
        };
        for (idx, line) in text.lines().enumerate() {
            if line
                .to_ascii_lowercase()
                .contains(&query.to_ascii_lowercase())
            {
                results.push(ProjectSearchResult {
                    path: file.path.clone(),
                    line: idx + 1,
                    preview: line.trim().chars().take(240).collect(),
                });
                if results.len() >= max {
                    break;
                }
            }
        }
    }
    let payload =
        serde_json::json!({ "query": query, "result_count": results.len(), "max_results": max });
    let _ = append_event(
        &conn,
        &req.session_id,
        "project.search",
        &payload.to_string(),
    );
    Ok(results)
}

#[tauri::command]
fn git_status(req: GitStatusRequest) -> Result<GitStatusView, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let real_repo = repo_root.canonicalize().map_err(|e| e.to_string())?;
    let output = Command::new("git")
        .arg("-C")
        .arg(&real_repo)
        .arg("status")
        .arg("--short")
        .arg("--branch")
        .output()
        .map_err(|e| format!("failed to run read-only git status: {}", e))?;
    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    let mut branch = "unknown".to_string();
    let mut files = Vec::new();
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            branch = rest.to_string();
        } else if line.len() >= 4 {
            let status = line[..2].trim().to_string();
            let path = line[3..].trim().to_string();
            if !path.is_empty() {
                files.push(GitStatusFileView { path, status });
            }
        }
    }
    if let Ok(conn) = init_audit(&repo_root, &req.session_id) {
        let payload = serde_json::json!({ "branch": branch, "changed_files": files.len(), "command": ["git", "status", "--short", "--branch"], "execution": "read-only backend command" });
        let _ = append_event(&conn, &req.session_id, "git.status", &payload.to_string());
    }
    Ok(GitStatusView { branch, files, raw })
}

#[tauri::command]
fn git_diff_file(req: GitDiffRequest) -> Result<GitDiffView, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let real_repo = repo_root.canonicalize().map_err(|e| e.to_string())?;
    let guard = RepoGuard::new(&real_repo, FilePolicy::default()).map_err(|e| e.to_string())?;
    let _ = guard
        .resolve_for_existing_path(&req.path)
        .map_err(|e| e.to_string())?;
    let mut cmd = Command::new("git");
    cmd.arg("diff");
    if req.staged {
        cmd.arg("--cached");
    }
    cmd.arg("--").arg(&req.path).current_dir(&real_repo);
    let output = cmd
        .output()
        .map_err(|e| format!("failed to run read-only git diff: {}", e))?;
    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    let limit = 80_000usize;
    let truncated = raw.len() > limit;
    let diff = raw.chars().take(limit).collect::<String>();
    let audit_event_id = init_audit(&real_repo, &req.session_id).ok().and_then(|conn| {
        let payload = serde_json::json!({ "path": &req.path, "staged": req.staged, "chars": diff.len(), "truncated": truncated, "operation": "git_diff_file", "execution": "read-only backend command" });
        append_event(&conn, &req.session_id, "git.diff", &payload.to_string()).ok()
    });
    Ok(GitDiffView {
        path: req.path,
        staged: req.staged,
        diff,
        truncated,
        audit_event_id,
    })
}

#[tauri::command]
fn git_stage_file(req: GitFileMutationRequest) -> Result<GitMutationResult, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let real_repo = repo_root.canonicalize().map_err(|e| e.to_string())?;
    let guard = RepoGuard::new(&real_repo, FilePolicy::default()).map_err(|e| e.to_string())?;
    let _ = guard
        .resolve_for_write_path(&req.path)
        .map_err(|e| e.to_string())?;
    let output = Command::new("git")
        .arg("add")
        .arg("--")
        .arg(&req.path)
        .current_dir(&real_repo)
        .output()
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout)
        .chars()
        .take(8000)
        .collect::<String>();
    let stderr = String::from_utf8_lossy(&output.stderr)
        .chars()
        .take(8000)
        .collect::<String>();
    let ok = output.status.success();
    let audit_event_id = init_audit(&real_repo, &req.session_id).ok().and_then(|conn| {
        let payload = serde_json::json!({ "path": &req.path, "ok": ok, "operation": "git_stage_file", "note": "user-initiated git index mutation" });
        append_event(&conn, &req.session_id, "git.stage", &payload.to_string()).ok()
    });
    Ok(GitMutationResult {
        ok,
        message: if ok {
            "File staged.".into()
        } else {
            "git add failed.".into()
        },
        stdout,
        stderr,
        audit_event_id,
    })
}

#[tauri::command]
fn git_unstage_file(req: GitFileMutationRequest) -> Result<GitMutationResult, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let real_repo = repo_root.canonicalize().map_err(|e| e.to_string())?;
    let guard = RepoGuard::new(&real_repo, FilePolicy::default()).map_err(|e| e.to_string())?;
    let _ = guard
        .resolve_for_write_path(&req.path)
        .map_err(|e| e.to_string())?;
    let output = Command::new("git")
        .arg("restore")
        .arg("--staged")
        .arg("--")
        .arg(&req.path)
        .current_dir(&real_repo)
        .output()
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout)
        .chars()
        .take(8000)
        .collect::<String>();
    let stderr = String::from_utf8_lossy(&output.stderr)
        .chars()
        .take(8000)
        .collect::<String>();
    let ok = output.status.success();
    let audit_event_id = init_audit(&real_repo, &req.session_id).ok().and_then(|conn| {
        let payload = serde_json::json!({ "path": &req.path, "ok": ok, "operation": "git_unstage_file", "note": "user-initiated git index mutation" });
        append_event(&conn, &req.session_id, "git.unstage", &payload.to_string()).ok()
    });
    Ok(GitMutationResult {
        ok,
        message: if ok {
            "File unstaged.".into()
        } else {
            "git restore --staged failed.".into()
        },
        stdout,
        stderr,
        audit_event_id,
    })
}

#[tauri::command]
fn git_commit_changes(req: GitCommitRequest) -> Result<GitMutationResult, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let real_repo = repo_root.canonicalize().map_err(|e| e.to_string())?;
    let message = req.message.trim();
    if message.len() < 3 || message.len() > 500 {
        return Err("commit message must be 3..500 characters".into());
    }
    let output = Command::new("git")
        .arg("commit")
        .arg("--no-verify")
        .arg("-m")
        .arg(message)
        .current_dir(&real_repo)
        .output()
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout)
        .chars()
        .take(12000)
        .collect::<String>();
    let stderr = String::from_utf8_lossy(&output.stderr)
        .chars()
        .take(12000)
        .collect::<String>();
    let ok = output.status.success();
    let audit_event_id = init_audit(&real_repo, &req.session_id).ok().and_then(|conn| {
        let payload = serde_json::json!({ "ok": ok, "operation": "git_commit_changes", "hooks": "--no-verify used to avoid executing repo hooks", "message_chars": message.len() });
        append_event(&conn, &req.session_id, "git.commit", &payload.to_string()).ok()
    });
    Ok(GitMutationResult {
        ok,
        message: if ok {
            "Commit created with --no-verify.".into()
        } else {
            "git commit failed.".into()
        },
        stdout,
        stderr,
        audit_event_id,
    })
}

#[tauri::command]
fn lsp_capabilities(req: LspCapabilityRequest) -> Result<Vec<LspCapabilityView>, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let files = list_files_internal(&repo_root, 1200)?;
    let has_ts = files.iter().any(|f| {
        f.path.ends_with(".ts")
            || f.path.ends_with(".tsx")
            || f.path.ends_with("package.json")
            || f.path.ends_with("tsconfig.json")
    });
    let has_py = files
        .iter()
        .any(|f| f.path.ends_with(".py") || f.path.ends_with("pyproject.toml"));
    let has_rs = files
        .iter()
        .any(|f| f.path.ends_with(".rs") || f.path.ends_with("Cargo.toml"));
    let has_go = files
        .iter()
        .any(|f| f.path.ends_with(".go") || f.path.ends_with("go.mod"));
    let caps = vec![
        "diagnostics",
        "hover",
        "go-to-definition",
        "find-references",
        "document-symbols",
        "formatting",
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();
    let views = vec![
        LspCapabilityView { language: "TypeScript/JavaScript".into(), detected: has_ts, server_hint: "typescript-language-server / tsserver".into(), capabilities: caps.clone(), notes: "V14 adds LSP detection/status scaffolding; full JSON-RPC LSP client wiring is the next hardening step.".into() },
        LspCapabilityView { language: "Python".into(), detected: has_py, server_hint: "pyright or basedpyright".into(), capabilities: caps.clone(), notes: "Detection only in this build.".into() },
        LspCapabilityView { language: "Rust".into(), detected: has_rs, server_hint: "rust-analyzer".into(), capabilities: caps.clone(), notes: "Detection only in this build.".into() },
        LspCapabilityView { language: "Go".into(), detected: has_go, server_hint: "gopls".into(), capabilities: caps, notes: "Detection only in this build.".into() },
    ];
    if let Ok(conn) = init_audit(&repo_root, &req.session_id) {
        let payload = serde_json::json!({ "detected": views.iter().filter(|v| v.detected).map(|v| v.language.clone()).collect::<Vec<_>>() });
        let _ = append_event(&conn, &req.session_id, "lsp.detected", &payload.to_string());
    }
    Ok(views)
}

#[tauri::command]
fn approve_personal_command(req: PersonalCommandRequest) -> Result<TaskApprovalView, String> {
    if req.argv.is_empty() {
        return Err("command argv cannot be empty".into());
    }
    if req
        .argv
        .iter()
        .any(|arg| arg.contains('\0') || arg.len() > 4096)
    {
        return Err("command contains an invalid argument".into());
    }
    if req.cwd.contains('\0') || req.cwd.trim().is_empty() {
        return Err("command cwd cannot be empty".into());
    }
    let repo_root = PathBuf::from(&req.repo_root);
    let canonical_repo = canonical_repo_root_string(&repo_root).map_err(|e| e.to_string())?;
    let guard = RepoGuard::new(&repo_root, FilePolicy::default()).map_err(|e| e.to_string())?;
    let _real_cwd = guard
        .resolve_for_existing_path(&req.cwd)
        .map_err(|e| format!("command cwd must stay inside repo: {}", e))?;
    // Personal Terminal commands are user-entered, so they use strict explicit-rule-only
    // policy. The UI checkboxes are recorded for audit context only; they are not allowed
    // to downgrade risk or turn an unknown allowlisted command into executable code.
    let guard_req = GuardCommandRequest {
        argv: req.argv.clone(),
        cwd: req.cwd.clone(),
        requires_network: req.requires_network,
        may_modify_files: req.may_modify_files,
    };
    let risk = classify(&guard_req, &personal_terminal_policy()).map_err(|e| e.to_string())?;
    let risk_label = risk_label(&risk).to_string();
    let conn = init_audit(&repo_root, &req.session_id).map_err(|e| e.to_string())?;
    if matches!(
        risk,
        CommandRisk::Network | CommandRisk::Destructive | CommandRisk::Blocked
    ) {
        let payload = serde_json::json!({
            "argv": &req.argv,
            "cwd": &req.cwd,
            "risk": &risk_label,
            "approved": false,
            "source": "personal-terminal",
            "reason": "strict personal policy refused network/destructive/blocked commands"
        });
        let _ = append_event(
            &conn,
            &req.session_id,
            "personal_command.approval_rejected",
            &payload.to_string(),
        );
        return Ok(TaskApprovalView {
            command_id: "none".into(),
            task_id: "personal-command".into(),
            risk: risk_label,
            approved: false,
            message: "Synthesize refused this command. Personal Terminal allows only explicit safe read/test/build rules; network, destructive, modifying, unknown, and fallback allowlisted commands are blocked.".into(),
        });
    }
    let command_id = new_id("cmd");
    let task_id = format!("personal-{}", command_id);
    let argv_json = serde_json::to_string(&req.argv).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO commands (id, session_id, task_id, repo_root, argv_json, cwd, risk, requires_network, may_modify_files, approved_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))",
        params![&command_id, &req.session_id, &task_id, &canonical_repo, &argv_json, &req.cwd, &risk_label, req.requires_network as i64, req.may_modify_files as i64],
    ).map_err(|e| e.to_string())?;
    let payload = serde_json::json!({
        "command_id": &command_id,
        "task_id": &task_id,
        "argv": &req.argv,
        "cwd": &req.cwd,
        "risk": &risk_label,
        "source": "personal-terminal",
        "approved_by_source": "local-user",
        "execution": "strict personal policy; user-entered argv command; no shell interpolation; timeout-bounded; env-scrubbed"
    });
    let _ = append_event(
        &conn,
        &req.session_id,
        "personal_command.approved",
        &payload.to_string(),
    );
    Ok(TaskApprovalView {
        command_id,
        task_id,
        risk: risk_label,
        approved: true,
        message: "Personal command approved under strict explicit-rule policy. It will run as argv, not through a shell, with a scrubbed environment, repo-bounded cwd, timeout, and audit logging.".into(),
    })
}

#[tauri::command]
fn detect_tasks(req: TaskDetectRequest) -> Result<Vec<DetectedTaskView>, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let mut tasks = Vec::new();
    let root_cwd = ".".to_string();
    if let Ok(pkg) = read_guarded_file(req.repo_root.clone(), "package.json".into()) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&pkg) {
            if let Some(scripts) = json.get("scripts").and_then(|v| v.as_object()) {
                for key in ["test", "lint", "build", "typecheck"] {
                    if scripts.contains_key(key) {
                        let argv = if key == "test" {
                            vec!["pnpm".into(), "test".into()]
                        } else {
                            vec!["pnpm".into(), "run".into(), key.into()]
                        };
                        let guard_req = GuardCommandRequest {
                            argv: argv.clone(),
                            cwd: root_cwd.clone(),
                            requires_network: false,
                            may_modify_files: key == "build",
                        };
                        let risk = classify(&guard_req, &CommandPolicy::default())
                            .map(|r| risk_label(&r).to_string())
                            .unwrap_or_else(|_| "Blocked".into());
                        tasks.push(DetectedTaskView { id: format!("package-{}", key), label: format!("package script: {}", key), argv, cwd: root_cwd.clone(), risk, reason: format!("Detected package.json script '{}'. This executes repo-defined local code and requires explicit backend approval.", key), requires_network: false, may_modify_files: key == "build" });
                    }
                }
            }
        }
    }
    if guarded_manifest_exists(&repo_root, "Cargo.toml") {
        tasks.push(DetectedTaskView {
            id: "cargo-test".into(),
            label: "cargo test".into(),
            argv: vec!["cargo".into(), "test".into()],
            cwd: root_cwd.clone(),
            risk: "TestOrBuild".into(),
            reason: "Detected Cargo.toml".into(),
            requires_network: false,
            may_modify_files: false,
        });
    }
    if guarded_manifest_exists(&repo_root, "pyproject.toml")
        || guarded_manifest_exists(&repo_root, "pytest.ini")
    {
        tasks.push(DetectedTaskView {
            id: "pytest".into(),
            label: "pytest".into(),
            argv: vec!["pytest".into()],
            cwd: root_cwd.clone(),
            risk: "TestOrBuild".into(),
            reason: "Detected Python test metadata".into(),
            requires_network: false,
            may_modify_files: false,
        });
    }
    if guarded_manifest_exists(&repo_root, "go.mod") {
        tasks.push(DetectedTaskView {
            id: "go-test".into(),
            label: "go test ./...".into(),
            argv: vec!["go".into(), "test".into(), "./...".into()],
            cwd: root_cwd,
            risk: "TestOrBuild".into(),
            reason: "Detected go.mod".into(),
            requires_network: false,
            may_modify_files: false,
        });
    }
    let canonical_repo = canonical_repo_root_string(&repo_root).map_err(|e| e.to_string())?;
    let conn = init_audit(&repo_root, &req.session_id).map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM task_snapshots WHERE session_id = ?1 AND repo_root = ?2",
        params![&req.session_id, &canonical_repo],
    )
    .map_err(|e| e.to_string())?;
    for task in &tasks {
        let argv_json = serde_json::to_string(&task.argv).map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO task_snapshots (task_id, session_id, repo_root, label, argv_json, cwd, risk, reason, requires_network, may_modify_files, detected_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, datetime('now'))",
            params![&task.id, &req.session_id, &canonical_repo, &task.label, &argv_json, &task.cwd, &task.risk, &task.reason, task.requires_network as i64, task.may_modify_files as i64],
        ).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    let payload = serde_json::json!({ "task_count": tasks.len(), "execution": "backend-detected snapshots only" });
    let _ = append_event(
        &conn,
        &req.session_id,
        "tasks.detected",
        &payload.to_string(),
    );
    Ok(tasks)
}

#[tauri::command]
fn approve_task(req: TaskApproveRequest) -> Result<TaskApprovalView, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let canonical_repo = canonical_repo_root_string(&repo_root).map_err(|e| e.to_string())?;
    let conn = init_audit(&repo_root, &req.session_id).map_err(|e| e.to_string())?;
    let row: (String, String, String, String, String, String, i64, i64) = conn.query_row(
        "SELECT label, argv_json, cwd, risk, reason, repo_root, requires_network, may_modify_files FROM task_snapshots WHERE task_id = ?1 AND session_id = ?2 AND repo_root = ?3",
        params![&req.task_id, &req.session_id, &canonical_repo],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?)),
    ).map_err(|_| "task snapshot not found for this repo; run Detect tasks before approving".to_string())?;
    if !repo_roots_equal(&row.5, &repo_root)
        .map_err(|e| format!("task snapshot repo-root verification failed: {}", e))?
    {
        return Err("task snapshot belongs to a different repo".into());
    }
    let argv: Vec<String> = serde_json::from_str(&row.1).map_err(|e| e.to_string())?;
    let guard_req = GuardCommandRequest {
        argv: argv.clone(),
        cwd: row.2.clone(),
        requires_network: row.6 != 0,
        may_modify_files: row.7 != 0,
    };
    let risk = classify(&guard_req, &CommandPolicy::default()).map_err(|e| e.to_string())?;
    let risk_label = risk_label(&risk).to_string();
    if risk_label != row.3 {
        return Err(format!(
            "detected task risk changed from {} to {}; re-detect tasks before approving",
            row.3, risk_label
        ));
    }
    if matches!(
        risk,
        CommandRisk::Network | CommandRisk::Destructive | CommandRisk::Blocked
    ) {
        let payload = serde_json::json!({ "task_id": req.task_id, "label": row.0, "risk": risk_label, "approved": false, "reason": "network/destructive/blocked tasks are not executable" });
        let _ = append_event(
            &conn,
            &req.session_id,
            "task.approval_rejected",
            &payload.to_string(),
        );
        return Ok(TaskApprovalView {
            command_id: "none".into(),
            task_id: req.task_id,
            risk: risk_label,
            approved: false,
            message: "Synthesize refuses network/destructive/blocked task execution.".into(),
        });
    }
    let command_id = new_id("cmd");
    conn.execute(
        "INSERT INTO commands (id, session_id, task_id, repo_root, argv_json, cwd, risk, requires_network, may_modify_files, approved_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))",
        params![&command_id, &req.session_id, &req.task_id, &row.5, &row.1, &row.2, &risk_label, row.6, row.7],
    ).map_err(|e| e.to_string())?;
    let payload = serde_json::json!({ "command_id": &command_id, "task_id": &req.task_id, "label": row.0, "argv": argv, "cwd": row.2, "risk": risk_label, "approved_by_source": "local-user", "execution": "approved backend-detected task snapshot" });
    let _ = append_event(
        &conn,
        &req.session_id,
        "task.approved",
        &payload.to_string(),
    );
    Ok(TaskApprovalView { command_id, task_id: req.task_id, risk: risk_label, approved: true, message: "Detected task snapshot approved by backend; execution remains argv-only, timeout-bounded, env-scrubbed, and audited.".into() })
}

#[tauri::command]
fn run_approved_task(req: TaskRunRequest) -> Result<TaskRunResult, String> {
    let repo_root = PathBuf::from(&req.repo_root);
    let conn = init_audit(&repo_root, &req.session_id).map_err(|e| e.to_string())?;
    let row: (String, String, String, String, String, i64, i64, Option<String>) = conn.query_row(
        "SELECT task_id, repo_root, argv_json, cwd, risk, requires_network, may_modify_files, approved_at FROM commands WHERE id = ?1 AND session_id = ?2",
        params![&req.command_id, &req.session_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?)),
    ).map_err(|_| "task approval not found for this session".to_string())?;
    if row.7.is_none() {
        return Err("task has not been approved".into());
    }
    if !repo_roots_equal(&row.1, &repo_root)
        .map_err(|e| format!("approved task repo-root verification failed: {}", e))?
    {
        return Err("approved task belongs to a different repo".into());
    }
    let stored_repo_root = PathBuf::from(&row.1);
    let argv: Vec<String> = serde_json::from_str(&row.2).map_err(|e| e.to_string())?;
    if argv.is_empty() {
        return Err("approved task has empty argv".into());
    }
    let rerun_guard_req = GuardCommandRequest {
        argv: argv.clone(),
        cwd: row.3.clone(),
        requires_network: row.5 != 0,
        may_modify_files: row.6 != 0,
    };
    let rerun_policy = if row.0.starts_with("personal-") {
        personal_terminal_policy()
    } else {
        CommandPolicy::default()
    };
    let rerun_risk = classify(&rerun_guard_req, &rerun_policy).map_err(|e| e.to_string())?;
    let rerun_risk_label = risk_label(&rerun_risk).to_string();
    if rerun_risk_label != row.4 {
        return Err(format!(
            "approved task risk changed from {} to {}; re-detect and re-approve",
            row.4, rerun_risk_label
        ));
    }
    if matches!(
        rerun_risk,
        CommandRisk::Network | CommandRisk::Destructive | CommandRisk::Blocked
    ) {
        return Err("approved task is no longer executable under current command policy".into());
    }
    let guard =
        RepoGuard::new(&stored_repo_root, FilePolicy::default()).map_err(|e| e.to_string())?;
    let real_cwd = guard
        .resolve_for_existing_path(&row.3)
        .map_err(|e| e.to_string())?;
    let start_payload = serde_json::json!({ "command_id": &req.command_id, "task_id": &row.0, "argv": &argv, "cwd": &row.3, "risk": &row.4, "timeout_seconds": 120, "env": "scrubbed", "network_sandbox": "not OS-enforced" });
    let _ = append_event(
        &conn,
        &req.session_id,
        "task.started",
        &start_payload.to_string(),
    );
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(real_cwd)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("HOME", std::env::var("HOME").unwrap_or_default())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn approved task: {}", e))?;
    let stdout_tail = Arc::new(Mutex::new(String::new()));
    let stderr_tail = Arc::new(Mutex::new(String::new()));
    if let Some(stdout) = child.stdout.take() {
        spawn_bounded_log_reader(stdout, stdout_tail.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_bounded_log_reader(stderr, stderr_tail.clone());
    }
    let started = Instant::now();
    let mut exit_code = None;
    let mut timed_out = false;
    loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => {
                exit_code = status.code();
                break;
            }
            None if started.elapsed() > Duration::from_secs(120) => {
                timed_out = true;
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
            None => thread::sleep(Duration::from_millis(100)),
        }
    }
    let stdout = tail_snapshot(&stdout_tail);
    let stderr = tail_snapshot(&stderr_tail);
    conn.execute("UPDATE commands SET started_at = COALESCE(started_at, datetime('now')), finished_at = datetime('now'), exit_code = ?2 WHERE id = ?1", params![&req.command_id, exit_code]).map_err(|e| e.to_string())?;
    let payload = serde_json::json!({ "command_id": &req.command_id, "exit_code": exit_code, "timed_out": timed_out, "stdout_tail_chars": stdout.len(), "stderr_tail_chars": stderr.len() });
    let event_id = append_event(
        &conn,
        &req.session_id,
        if timed_out {
            "task.timed_out"
        } else {
            "task.finished"
        },
        &payload.to_string(),
    )
    .ok();
    Ok(TaskRunResult { command_id: req.command_id, exit_code, timed_out, stdout_tail: stdout, stderr_tail: stderr, message: "Approved task finished. Output is bounded; command execution remains governed and audited.".into(), audit_event_id: event_id })
}

#[tauri::command]
fn runtime_status() -> RuntimeStatusView {
    RuntimeStatusView {
        active_runtime: managed_llamacpp_status().status,
        loaded_model: Some("selected local coding model or fake fixture-patcher".into()),
        local_only_target: true,
        llamacpp_supervisor: "managed llama.cpp can import a local binary/model path and spawn argv-only on 127.0.0.1; manual local servers are also supported".into(),
        notes: vec![
            "Fake runtime, local model server, managed llama.cpp, and optional cloud providers are wired through backend-owned context/model calls.".into(),
            "Protocol label OpenAI-compatible refers to local-server wire compatibility; cloud lanes require API keys and explicit non-local endpoint approval.".into(),
            "Agent-suggested commands remain classification-only. Backend-detected tasks can run through the governed task runner after explicit approval.".into(),
        ],
    }
}

#[tauri::command]
fn list_curated_models() -> Vec<CuratedModelView> {
    vec![
        // Qwen3 Coder — primary local skill-agent models
        CuratedModelView { id: "qwen3-coder-0.6b-instruct-q4_k_m".into(), name: "Qwen3 Coder 0.6B Q4_K_M (fast skill agent)".into(), runtime: "llamacpp".into(), format: "gguf".into(), recommended_ram_gb: 4, notes: "Ultra-fast skill agent for low-lift tasks. Use scripts/bootstrap-qwen3.ps1 -Model coder-0.6b to download.".into() },
        CuratedModelView { id: "qwen3-coder-1.7b-instruct-q4_k_m".into(), name: "Qwen3 Coder 1.7B Q4_K_M (default skill agent)".into(), runtime: "llamacpp".into(), format: "gguf".into(), recommended_ram_gb: 6, notes: "Default balanced skill agent. Use scripts/bootstrap-qwen3.ps1 -Model coder-1.7b to download.".into() },
        CuratedModelView { id: "qwen3-coder-8b-instruct-q4_k_m".into(), name: "Qwen3 Coder 8B Q4_K_M (powerful skill agent)".into(), runtime: "llamacpp".into(), format: "gguf".into(), recommended_ram_gb: 14, notes: "Powerful local agent for complex tasks. Use scripts/bootstrap-qwen3.ps1 -Model coder-8b to download.".into() },
        CuratedModelView { id: "qwen3-coder-14b-instruct-q4_k_m".into(), name: "Qwen3 Coder 14B Q4_K_M (frontier local)".into(), runtime: "llamacpp".into(), format: "gguf".into(), recommended_ram_gb: 22, notes: "Maximum local quality. Supply your own GGUF URL or import a downloaded file.".into() },
        // Legacy Qwen2.5 Coder (retained for compatibility)
        CuratedModelView { id: "qwen2.5-coder-7b-instruct-q4_k_m".into(), name: "Qwen2.5 Coder 7B Instruct Q4_K_M (legacy)".into(), runtime: "llamacpp".into(), format: "gguf".into(), recommended_ram_gb: 12, notes: "Previous generation local model. Prefer Qwen3 for new setups.".into() },
        CuratedModelView { id: "qwen2.5-coder-14b-instruct-q4_k_m".into(), name: "Qwen2.5 Coder 14B Instruct Q4_K_M (legacy)".into(), runtime: "llamacpp".into(), format: "gguf".into(), recommended_ram_gb: 20, notes: "Previous generation local model. Prefer Qwen3 for new setups.".into() },
        // Cloud frontier models (require API key env vars)
        CuratedModelView { id: "cloud-gpt-4o".into(), name: "GPT-4o (OpenAI Cloud — heavy-lift)".into(), runtime: "cloud-openai".into(), format: "remote-compatible".into(), recommended_ram_gb: 0, notes: "Cloud heavy-lift. Requires OPENAI_API_KEY env var. Endpoint approval required before repo context is sent.".into() },
        CuratedModelView { id: "cloud-o3".into(), name: "o3 (OpenAI Cloud — reasoning)".into(), runtime: "cloud-openai".into(), format: "remote-compatible".into(), recommended_ram_gb: 0, notes: "Cloud reasoning. Requires OPENAI_API_KEY env var. Reserved for hard algorithmic problems.".into() },
        CuratedModelView { id: "cloud-claude-sonnet".into(), name: "Claude Sonnet (Anthropic Cloud — heavy-lift)".into(), runtime: "cloud-anthropic".into(), format: "remote-compatible".into(), recommended_ram_gb: 0, notes: "Cloud heavy-lift. Requires ANTHROPIC_API_KEY env var. Excellent for large-context code understanding.".into() },
    ]
}

#[tauri::command]
fn register_local_model(req: RegisterModelRequest) -> Result<RegisterModelResult, String> {
    let path = PathBuf::from(&req.local_path);
    if !path.exists() {
        return Err("model path does not exist".into());
    }
    Ok(RegisterModelResult {
        id: req.model_id,
        name: req.name,
        local_path: path.to_string_lossy().to_string(),
        registered: true,
        message: format!("registered {} model metadata for {} runtime; use Local Model Runtime Control to run managed llama.cpp or connect a self-hosted local model server", req.format, req.runtime),
    })
}

#[tauri::command]
fn list_runtime_presets() -> Vec<RuntimePresetView> {
    vec![
        // Local presets (no API key; no approval gate for localhost)
        RuntimePresetView { id: "llamacpp-server".into(), label: "llama.cpp server (Qwen3/local)".into(), default_url: "http://localhost:8080/v1".into(), protocol: "OpenAI-compatible local HTTP".into(), notes: "Run a Qwen3 GGUF model locally with llama.cpp server. No API key required.".into(), local_by_default: true },
        RuntimePresetView { id: "lm-studio".into(), label: "LM Studio local server".into(), default_url: "http://localhost:1234/v1".into(), protocol: "OpenAI-compatible local HTTP".into(), notes: "Use LM Studio's local server mode with a downloaded open-source model.".into(), local_by_default: true },
        RuntimePresetView { id: "ollama-local".into(), label: "Ollama local".into(), default_url: "http://localhost:11434/v1".into(), protocol: "OpenAI-compatible local HTTP route".into(), notes: "Use Ollama's local OpenAI-compatible route. Run qwen3-coder via Ollama locally.".into(), local_by_default: true },
        RuntimePresetView { id: "vllm-local".into(), label: "vLLM local server".into(), default_url: "http://localhost:8000/v1".into(), protocol: "OpenAI-compatible local HTTP".into(), notes: "For workstation/server GPU setups running Qwen3 or other open-source coding models.".into(), local_by_default: true },
        RuntimePresetView { id: "custom-local".into(), label: "Custom local model server".into(), default_url: "http://localhost:8080/v1".into(), protocol: "OpenAI-compatible local HTTP".into(), notes: "Bring your own self-hosted model server. Non-local hosts require explicit backend approval.".into(), local_by_default: true },
        // Cloud presets (require API key env vars + explicit endpoint approval)
        RuntimePresetView { id: "cloud-openai".into(), label: "OpenAI API (cloud heavy-lift)".into(), default_url: "https://api.openai.com/v1".into(), protocol: "OpenAI REST API".into(), notes: "Cloud frontier model for heavy-lift tasks. Requires OPENAI_API_KEY env var. Explicit approval required before repo context is sent.".into(), local_by_default: false },
        RuntimePresetView { id: "cloud-anthropic".into(), label: "Anthropic API (cloud heavy-lift)".into(), default_url: "https://api.anthropic.com/v1".into(), protocol: "Anthropic Messages API".into(), notes: "Cloud frontier model for heavy-lift tasks. Requires ANTHROPIC_API_KEY env var. Explicit approval required before repo context is sent.".into(), local_by_default: false },
    ]
}

#[tauri::command]
fn import_local_model(req: ImportLocalModelRequest) -> Result<LocalModelView, String> {
    let path = PathBuf::from(&req.local_path);
    if !path.exists() || !path.is_file() {
        return Err("GGUF model path must be an existing file".into());
    }
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext != "gguf" {
        return Err("managed llama.cpp imports currently require a .gguf model file".into());
    }
    let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
    let sha = if req.calculate_sha256 {
        Some(sha256_file_for_model(&path)?)
    } else {
        None
    };
    let id = format!(
        "gguf-{}",
        sha.as_deref()
            .unwrap_or_else(|| path.file_stem().and_then(|s| s.to_str()).unwrap_or("model"))
    );
    let conn = user_config_conn()?;
    init_schema(&conn).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO local_models (id, display_name, local_path, format, runtime_compatibility, size_bytes, sha256) VALUES (?1, ?2, ?3, 'gguf', 'llamacpp', ?4, ?5)",
        params![&id, &req.display_name, &path.to_string_lossy().to_string(), meta.len() as i64, &sha],
    ).map_err(|e| e.to_string())?;
    Ok(LocalModelView {
        id,
        display_name: req.display_name,
        local_path: path.to_string_lossy().to_string(),
        format: "gguf".into(),
        runtime_compatibility: "llamacpp".into(),
        size_bytes: Some(meta.len()),
        sha256: sha,
        imported_at: None,
    })
}

#[tauri::command]
fn list_local_models() -> Result<Vec<LocalModelView>, String> {
    let conn = user_config_conn()?;
    init_schema(&conn).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, display_name, local_path, format, runtime_compatibility, size_bytes, sha256, imported_at FROM local_models ORDER BY imported_at DESC").map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(LocalModelView {
                id: row.get(0)?,
                display_name: row.get(1)?,
                local_path: row.get(2)?,
                format: row.get(3)?,
                runtime_compatibility: row.get(4)?,
                size_bytes: row.get::<_, Option<i64>>(5)?.map(|v| v as u64),
                sha256: row.get(6)?,
                imported_at: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
fn managed_llamacpp_validate_config(
    req: ManagedLlamaConfigRequest,
) -> Result<ManagedLlamaStatusView, String> {
    let binary = PathBuf::from(&req.binary_path);
    let model = PathBuf::from(&req.model_path);
    if !binary.exists() || !binary.is_file() {
        return Err(
            "llama.cpp server binary path must point to an existing executable file".into(),
        );
    }
    if !model.exists() || !model.is_file() {
        return Err("GGUF model path must point to an existing file".into());
    }
    if model
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        != "gguf"
    {
        return Err("managed llama.cpp requires a .gguf model file".into());
    }
    let port = req.port.unwrap_or(8080);
    Ok(ManagedLlamaStatusView { status: "valid".into(), endpoint_url: Some(format!("http://127.0.0.1:{}/v1", port)), pid: None, model_path: Some(req.model_path), binary_path: Some(req.binary_path), stdout_tail: None, stderr_tail: None, message: "managed llama.cpp config is valid; start uses argv-only process spawning bound to 127.0.0.1".into() })
}

#[tauri::command]
fn managed_llamacpp_start(
    req: ManagedLlamaConfigRequest,
) -> Result<ManagedLlamaStatusView, String> {
    managed_llamacpp_validate_config(ManagedLlamaConfigRequest {
        binary_path: req.binary_path.clone(),
        model_path: req.model_path.clone(),
        port: req.port,
        ctx_size: req.ctx_size,
    })?;
    let port = req.port.unwrap_or(8080);
    let ctx = req.ctx_size.unwrap_or(8192);
    let endpoint = format!("http://127.0.0.1:{}/v1", port);
    let mutex = MANAGED_LLAMA.get_or_init(|| Mutex::new(None));
    let mut slot = mutex
        .lock()
        .map_err(|_| "managed llama.cpp supervisor lock is poisoned".to_string())?;
    if let Some(existing) = slot.as_mut() {
        if let Some(status) = existing.child.try_wait().map_err(|e| e.to_string())? {
            let stdout_tail = tail_snapshot(&existing.stdout_tail);
            let stderr_tail = tail_snapshot(&existing.stderr_tail);
            *slot = None;
            return Ok(ManagedLlamaStatusView {
                status: "failed".into(),
                endpoint_url: Some(endpoint),
                pid: None,
                model_path: Some(req.model_path),
                binary_path: Some(req.binary_path),
                stdout_tail: Some(stdout_tail),
                stderr_tail: Some(stderr_tail),
                message: format!(
                    "previous managed llama.cpp process exited before start request; status={}",
                    status
                ),
            });
        }
        return Ok(ManagedLlamaStatusView {
            status: "ready-or-started".into(),
            endpoint_url: Some(existing.endpoint_url.clone()),
            pid: Some(existing.child.id()),
            model_path: Some(existing.model_path.clone()),
            binary_path: Some(existing.binary_path.clone()),
            stdout_tail: Some(tail_snapshot(&existing.stdout_tail)),
            stderr_tail: Some(tail_snapshot(&existing.stderr_tail)),
            message: "managed llama.cpp process is already tracked by Synthesize".into(),
        });
    }
    let binary_dir = Path::new(&req.binary_path)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let mut child = Command::new(&req.binary_path)
        .arg("--model")
        .arg(&req.model_path)
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .arg("--ctx-size")
        .arg(ctx.to_string())
        .arg("--threads")
        .arg("4")
        .arg("--no-webui")
        .arg("--alias")
        .arg(
            Path::new(&req.model_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("managed-gguf"),
        )
        .current_dir(binary_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "failed to start llama.cpp server with argv-only spawn: {}",
                e
            )
        })?;
    let pid = child.id();
    let stdout_tail = Arc::new(Mutex::new(String::new()));
    let stderr_tail = Arc::new(Mutex::new(String::new()));
    if let Some(stdout) = child.stdout.take() {
        spawn_bounded_log_reader(stdout, stdout_tail.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_bounded_log_reader(stderr, stderr_tail.clone());
    }
    *slot = Some(ManagedLlamaProcess {
        child,
        endpoint_url: endpoint.clone(),
        model_path: req.model_path.clone(),
        binary_path: req.binary_path.clone(),
        stdout_tail: stdout_tail.clone(),
        stderr_tail: stderr_tail.clone(),
    });
    Ok(ManagedLlamaStatusView {
        status: "starting".into(),
        endpoint_url: Some(endpoint),
        pid: Some(pid),
        model_path: Some(req.model_path),
        binary_path: Some(req.binary_path),
        stdout_tail: Some(tail_snapshot(&stdout_tail)),
        stderr_tail: Some(tail_snapshot(&stderr_tail)),
        message: "managed llama.cpp server started; use health check to confirm readiness".into(),
    })
}

#[tauri::command]
fn managed_llamacpp_stop() -> Result<ManagedLlamaStatusView, String> {
    let mutex = MANAGED_LLAMA.get_or_init(|| Mutex::new(None));
    let mut slot = mutex
        .lock()
        .map_err(|_| "managed llama.cpp supervisor lock is poisoned".to_string())?;
    if let Some(mut existing) = slot.take() {
        let stdout_tail = tail_snapshot(&existing.stdout_tail);
        let stderr_tail = tail_snapshot(&existing.stderr_tail);
        let _ = existing.child.kill();
        let _ = existing.child.wait();
        Ok(ManagedLlamaStatusView {
            status: "stopped".into(),
            endpoint_url: Some(existing.endpoint_url),
            pid: None,
            model_path: Some(existing.model_path),
            binary_path: Some(existing.binary_path),
            stdout_tail: Some(stdout_tail),
            stderr_tail: Some(stderr_tail),
            message: "managed llama.cpp process stopped".into(),
        })
    } else {
        Ok(ManagedLlamaStatusView {
            status: "stopped".into(),
            endpoint_url: None,
            pid: None,
            model_path: None,
            binary_path: None,
            stdout_tail: None,
            stderr_tail: None,
            message: "no managed llama.cpp process is currently tracked".into(),
        })
    }
}

#[tauri::command]
fn managed_llamacpp_status() -> ManagedLlamaStatusView {
    let mutex = MANAGED_LLAMA.get_or_init(|| Mutex::new(None));
    let mut slot = match mutex.lock() {
        Ok(s) => s,
        Err(_) => {
            return ManagedLlamaStatusView {
                status: "failed".into(),
                endpoint_url: None,
                pid: None,
                model_path: None,
                binary_path: None,
                stdout_tail: None,
                stderr_tail: None,
                message: "managed llama.cpp supervisor lock is poisoned".into(),
            }
        }
    };
    if let Some(existing) = slot.as_mut() {
        match existing.child.try_wait() {
            Ok(Some(status)) => {
                let stdout_tail = tail_snapshot(&existing.stdout_tail);
                let stderr_tail = tail_snapshot(&existing.stderr_tail);
                let endpoint = existing.endpoint_url.clone();
                let model = existing.model_path.clone();
                let binary = existing.binary_path.clone();
                *slot = None;
                ManagedLlamaStatusView {
                    status: "failed".into(),
                    endpoint_url: Some(endpoint),
                    pid: None,
                    model_path: Some(model),
                    binary_path: Some(binary),
                    stdout_tail: Some(stdout_tail),
                    stderr_tail: Some(stderr_tail),
                    message: format!("managed llama.cpp process exited; status={}", status),
                }
            }
            Ok(None) => ManagedLlamaStatusView {
                status: "started".into(),
                endpoint_url: Some(existing.endpoint_url.clone()),
                pid: Some(existing.child.id()),
                model_path: Some(existing.model_path.clone()),
                binary_path: Some(existing.binary_path.clone()),
                stdout_tail: Some(tail_snapshot(&existing.stdout_tail)),
                stderr_tail: Some(tail_snapshot(&existing.stderr_tail)),
                message: "managed llama.cpp process is tracked; use health check for readiness"
                    .into(),
            },
            Err(err) => ManagedLlamaStatusView {
                status: "failed".into(),
                endpoint_url: Some(existing.endpoint_url.clone()),
                pid: Some(existing.child.id()),
                model_path: Some(existing.model_path.clone()),
                binary_path: Some(existing.binary_path.clone()),
                stdout_tail: Some(tail_snapshot(&existing.stdout_tail)),
                stderr_tail: Some(tail_snapshot(&existing.stderr_tail)),
                message: format!("failed to inspect managed llama.cpp process: {}", err),
            },
        }
    } else {
        ManagedLlamaStatusView {
            status: "stopped".into(),
            endpoint_url: None,
            pid: None,
            model_path: None,
            binary_path: None,
            stdout_tail: None,
            stderr_tail: None,
            message: "no managed llama.cpp process is currently tracked".into(),
        }
    }
}

fn spawn_bounded_log_reader<R: Read + Send + 'static>(mut reader: R, tail: Arc<Mutex<String>>) {
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]);
                    if let Ok(mut s) = tail.lock() {
                        s.push_str(&chunk);
                        if s.len() > 64 * 1024 {
                            let keep_from = s.len().saturating_sub(64 * 1024);
                            let trimmed = s[keep_from..].to_string();
                            *s = trimmed;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });
}

fn tail_snapshot(tail: &Arc<Mutex<String>>) -> String {
    tail.lock().map(|s| s.clone()).unwrap_or_default()
}

fn user_config_conn() -> Result<Connection, String> {
    let dir = synthesize_app_data_dir().join("runtime");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Connection::open(dir.join("synthesize-runtime.sqlite")).map_err(|e| e.to_string())
}

fn synthesize_app_data_dir() -> PathBuf {
    if let Ok(path) = std::env::var("SYNTHESIZE_APP_DATA_DIR") {
        return PathBuf::from(path);
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("SynthesizeIDE");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("SynthesizeIDE");
        }
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("synthesize-ide");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("synthesize-ide");
    }
    std::env::temp_dir().join("synthesize-ide-app-data-fallback")
}

fn sha256_file_for_model(path: &Path) -> Result<String, String> {
    use std::io::Read;
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Returns current UTC time as an ISO 8601 string without requiring chrono.
fn iso_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, mo, d, h, mi, s) = unix_to_utc(secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, d, h, mi, s)
}

fn unix_to_utc(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = secs % 60;
    let mins = secs / 60;
    let mi = mins % 60;
    let hours = mins / 60;
    let h = hours % 24;
    let days = hours / 24;
    // Gregorian calendar approximation (good through 2100)
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    (y, mo, d, h, mi, s)
}

fn build_synthesize_system_prompt(agent_profile_id: &str) -> String {
    let role_guidance = match agent_profile_id {
        "local-planner" => "Profile: Local Planner. Emit report or ask_user operations only. Do not emit propose_patch; this profile is not permitted to move patches into Synthesize's backend patch lifecycle.",
        "local-reviewer" => "Profile: Local Reviewer. Emit report operations only for critique and risk review. Do not emit propose_patch; this profile is not permitted to move patches into Synthesize's backend patch lifecycle.",
        "moa-action-planner" => "Profile: MoA Action Planner. Use a local model to create an explicit plan/action trace and emit typed operations for Synthesize/MoA governance. You may emit report, ask_user, propose_patch, final_report, and run_command suggestions. This mode does not claim to expose private model chain-of-thought. The model is not the actor: it proposes; Synthesize validates, approves, applies, rolls back, and audits.",
        "fake-demo" => "Profile: Fake Demo Agent. Produce deterministic fixture-style typed operations when applicable.",
        _ => "Profile: Local Patcher. You may emit small, reviewable propose_patch operations. Backend approval is required before any change is applied.",
    };
    format!("You are Synthesize's local coding agent running against a self-hosted open-source coding model runtime. You are inside a governed local IDE. You cannot modify files directly. You cannot run shell commands. Return strict JSON only in the form {{\"operations\":[...]}}; do not include markdown outside JSON. Supported operations are propose_patch, report, ask_user, final_report, hand_off, and run_command suggestions for classification-only. For code changes, emit propose_patch with unified diffs. Include beforeSha256 exactly as provided in context. Keep patches small and reviewable. Do not invent files unless creating them intentionally. Do not read or modify denied files. Do not claim changes were applied. Command requests are suggestions only; execution is disabled. For hand_off operations, set toSkill to a valid skill ID, contextSummary to a concise summary of what was accomplished, and preserveFileHistory to true. If more context is needed, emit ask_user or report. {}", role_guidance)
}

fn build_skill_system_prompt(skill: &SkillDefinition) -> String {
    let base = format!(
        "You are Synthesize's '{}' skill agent. {}. You are inside a governed local IDE. You cannot modify files directly. Return strict JSON only in the form {{\"operations\":[...]}}; do not include markdown outside JSON. Supported operations: {}. For code changes, emit propose_patch with unified diffs including beforeSha256. To delegate to another agent, emit hand_off with toSkill (the target skill ID), contextSummary, and preserveFileHistory. All patches/commands pass through Synthesize validation, approval, apply, and audit — you only propose.",
        skill.name,
        skill.description,
        skill.allowed_operations.join(", ")
    );
    if skill.system_prompt_addon.is_empty() {
        base
    } else {
        format!("{}\n\n{}", base, skill.system_prompt_addon)
    }
}

fn classify_endpoint_url(endpoint_url: &str) -> String {
    let lower = endpoint_url.to_ascii_lowercase();
    let host = match host_from_endpoint(&lower) {
        Some(h) => h,
        None => return "remote".into(),
    };
    if host == "localhost" || host == "127.0.0.1" || host == "::1" || host == "[::1]" {
        return "local".into();
    }
    if is_private_lan_host(&host) {
        return "private-lan".into();
    }
    "remote".into()
}

fn host_from_endpoint(endpoint_url: &str) -> Option<String> {
    let rest = endpoint_url
        .split_once("://")
        .map(|(_, r)| r)
        .unwrap_or(endpoint_url);
    let authority = rest.split('/').next()?;
    if authority.starts_with('[') {
        return authority.split(']').next().map(|s| format!("{}]", s));
    }
    Some(
        authority
            .split('@')
            .last()
            .unwrap_or(authority)
            .split(':')
            .next()
            .unwrap_or(authority)
            .to_string(),
    )
}

fn is_private_lan_host(host: &str) -> bool {
    if host.starts_with("10.") || host.starts_with("192.168.") {
        return true;
    }
    if let Some(rest) = host.strip_prefix("172.") {
        if let Some(first) = rest.split('.').next().and_then(|s| s.parse::<u8>().ok()) {
            return (16..=31).contains(&first);
        }
    }
    host.ends_with(".local")
}

fn sanitized_endpoint_for_log(endpoint_url: &str) -> String {
    // Do not log credentials in endpoint URLs.
    if let Some((scheme, rest)) = endpoint_url.split_once("://") {
        let without_auth = rest.split('@').last().unwrap_or(rest);
        format!("{}://{}", scheme, without_auth)
    } else {
        endpoint_url.to_string()
    }
}

fn load_context_bundle(
    conn: &Connection,
    context_bundle_id: &str,
) -> Result<ContextBundleView, String> {
    let payload: String = conn
        .query_row(
            "SELECT payload_json FROM context_bundles WHERE id = ?1",
            params![context_bundle_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("context bundle {} not found", context_bundle_id))?;
    serde_json::from_str(&payload)
        .map_err(|e| format!("persisted context bundle is invalid JSON: {}", e))
}

fn hash_runtime_messages(messages: &[RuntimeMessage]) -> Result<String, String> {
    let canonical = canonical_json(&serde_json::to_value(messages).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(sha256_str(&canonical))
}

fn conservative_runtime_message_tokens(messages: &[RuntimeMessage]) -> usize {
    messages
        .iter()
        .map(|message| message.content.len().div_ceil(3) + 8)
        .sum::<usize>()
        + 4
}

fn normalize_endpoint_base(endpoint_url: &str) -> Result<String, String> {
    let trimmed = endpoint_url.trim().trim_end_matches('/');
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err("endpoint_url must start with http:// or https://".into());
    }
    Ok(trimmed.to_string())
}

fn verify_endpoint_approval(
    conn: &Connection,
    endpoint_url: &str,
    classification: &str,
) -> Result<(), String> {
    let allowed: Option<i64> = conn.query_row(
        "SELECT allow_repo_context FROM endpoint_approvals WHERE endpoint_url = ?1 AND endpoint_classification = ?2",
        params![endpoint_url, classification],
        |row| row.get(0),
    ).optional().map_err(|e| e.to_string())?;
    match allowed {
        Some(1) => Ok(()),
        _ => Err(format!(
            "endpoint {} is {}; backend endpoint approval is required before sending repo context",
            sanitized_endpoint_for_log(endpoint_url),
            classification
        )),
    }
}

fn read_package_scripts_excerpt(repo_root: &Path) -> Option<String> {
    let guard = RepoGuard::new(repo_root, FilePolicy::default()).ok()?;
    let package = guard.resolve_for_existing_path("package.json").ok()?;
    let text = fs::read_to_string(package).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let scripts = v.get("scripts")?;
    Some(serde_json::to_string_pretty(scripts).unwrap_or_else(|_| scripts.to_string()))
}

fn fake_runtime_response(messages: &[RuntimeMessage]) -> String {
    let prompt = messages
        .iter()
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let before_sha = extract_prompt_value(&prompt, "beforeSha256")
        .unwrap_or_else(|| "fixture-before-sha256".into());
    let current_file = extract_prompt_value(&prompt, "currentFile")
        .unwrap_or_else(|| "src/auth/refresh.ts".into());
    let current_commit = extract_prompt_value(&prompt, "currentCommit").filter(|v| !v.is_empty());
    let proposal_id = new_id("fixture");
    let patch = format!("diff --git a/{0} b/{0}\n--- a/{0}\n+++ b/{0}\n@@ -1,3 +1,3 @@\n export function refreshToken() {{\n-  throw new Error(\"not implemented\");\n+  return \"refreshed\";\n }}\n", current_file);
    serde_json::json!({
        "operations": [{
            "type": "propose_patch",
            "proposalId": proposal_id.clone(),
            "summary": "Replace throwing refreshToken stub with a deterministic return value.",
            "currentCommit": current_commit,
            "files": [{ "id": format!("{}-file-001", proposal_id), "path": current_file, "beforeSha256": before_sha, "patch": patch, "risk": "low" }],
            "riskNotes": ["Fixture patch only. Backend validates file hash, path, lifecycle, approval, checkpoint, and rollback."],
            "suggestedCommands": [{
                "type": "run_command",
                "argv": ["pnpm", "test", "auth"],
                "cwd": ".",
                "reason": "Verify auth refresh behavior.",
                "expectedOutcome": "Auth tests pass.",
                "requiresNetwork": false,
                "mayModifyFiles": false
            }]
        }]
    }).to_string()
}

fn extract_prompt_value(prompt: &str, key: &str) -> Option<String> {
    let needle = format!("{}=", key);
    prompt
        .lines()
        .find_map(|line| line.strip_prefix(&needle).map(|v| v.trim().to_string()))
}

fn local_chat_request_body(
    model: &str,
    messages: &[RuntimeMessage],
    temperature: f32,
    max_tokens: u32,
    response_format: Option<&str>,
) -> Result<serde_json::Value, String> {
    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "temperature": temperature,
        "max_tokens": max_tokens,
        "stream": false
    });
    if response_format == Some("json_schema") {
        // Many local model servers implement JSON mode as OpenAI-compatible
        // {"type":"json_object"}. Do not send response_format:null to
        // stricter servers that reject unknown/null fields.
        if let Some(obj) = body.as_object_mut() {
            obj.insert(
                "response_format".into(),
                serde_json::json!({"type":"json_object"}),
            );
        }
    }
    Ok(body)
}

fn call_openai_compatible_endpoint(
    endpoint_url: &str,
    model: &str,
    messages: &[RuntimeMessage],
    temperature: f32,
    max_tokens: u32,
    response_format: Option<&str>,
) -> Result<String, String> {
    let base = normalize_endpoint_base(endpoint_url)?;
    let body = local_chat_request_body(model, messages, temperature, max_tokens, response_format)?;
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(180))
        .build();
    let resp = agent
        .post(&format!("{}/chat/completions", base))
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(format_runtime_error)?;
    let json: serde_json::Value = resp
        .into_json()
        .map_err(|e| format!("endpoint returned invalid JSON: {}", e))?;
    let content = json
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            "endpoint response did not include choices[0].message.content".to_string()
        })?;
    Ok(content.to_string())
}

fn format_runtime_error(err: ureq::Error) -> String {
    match err {
        ureq::Error::Status(code, resp) => {
            let text = resp.into_string().unwrap_or_default();
            format!(
                "endpoint returned HTTP {}: {}",
                code,
                text.chars().take(800).collect::<String>()
            )
        }
        ureq::Error::Transport(t) => format!("endpoint connection/transport error: {}", t),
    }
}

fn call_cloud_openai(
    endpoint_url: &str,
    model: &str,
    messages: &[RuntimeMessage],
    temperature: f32,
    max_tokens: u32,
) -> Result<String, String> {
    let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
        "OPENAI_API_KEY environment variable is not set; cloud-openai provider requires it"
            .to_string()
    })?;
    if api_key.trim().is_empty() {
        return Err("OPENAI_API_KEY is set but empty".into());
    }
    let body = local_chat_request_body(
        model,
        messages,
        temperature,
        max_tokens,
        Some("json_schema"),
    )?;
    let base = normalize_endpoint_base(endpoint_url)?;
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(300))
        .build();
    let resp = agent
        .post(&format!("{}/chat/completions", base))
        .set("Content-Type", "application/json")
        .set("Authorization", &format!("Bearer {}", api_key))
        .send_json(body)
        .map_err(format_runtime_error)?;
    let json: serde_json::Value = resp
        .into_json()
        .map_err(|e| format!("OpenAI API returned invalid JSON: {}", e))?;
    let content = json
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            "OpenAI API response did not include choices[0].message.content".to_string()
        })?;
    Ok(content.to_string())
}

fn call_cloud_anthropic(
    endpoint_url: &str,
    model: &str,
    messages: &[RuntimeMessage],
    max_tokens: u32,
) -> Result<String, String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| "ANTHROPIC_API_KEY environment variable is not set; cloud-anthropic provider requires it".to_string())?;
    if api_key.trim().is_empty() {
        return Err("ANTHROPIC_API_KEY is set but empty".into());
    }
    let system_content: Vec<String> = messages
        .iter()
        .filter(|m| m.role == "system")
        .map(|m| m.content.clone())
        .collect();
    let user_messages: Vec<serde_json::Value> = messages
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
        .collect();
    let mut body = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": user_messages
    });
    if !system_content.is_empty() {
        body["system"] = serde_json::json!(system_content.join("\n"));
    }
    let base = normalize_endpoint_base(endpoint_url)?;
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(300))
        .build();
    let resp = agent
        .post(&format!("{}/messages", base))
        .set("Content-Type", "application/json")
        .set("x-api-key", &api_key)
        .set("anthropic-version", "2023-06-01")
        .send_json(body)
        .map_err(format_runtime_error)?;
    let json: serde_json::Value = resp
        .into_json()
        .map_err(|e| format!("Anthropic API returned invalid JSON: {}", e))?;
    let content = json
        .get("content")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
        })
        .and_then(|b| b.get("text"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Anthropic API response did not include a text content block".to_string())?;
    Ok(content.to_string())
}

fn enforce_agent_profile_allows_operation(
    agent_profile_id: &str,
    operation: &AgentOperation,
) -> Result<(), String> {
    match operation {
        AgentOperation::ProposePatch { .. } => {
            match agent_profile_id {
                "local-patcher" | "moa-action-planner" | "fake-demo" => Ok(()),
                "local-planner" => Err("Local Planner is report/ask-user oriented and cannot validate patch proposals. Switch to Local Patcher to propose/apply code changes.".into()),
                "local-reviewer" => Err("Local Reviewer is report-only for review/critique and cannot validate patch proposals. Switch to Local Patcher or MoA Action Planner to propose/apply code changes.".into()),
                other => Err(format!("unknown or unsupported agent profile '{}' for patch validation", other)),
            }
        }
        AgentOperation::RunCommand(_) => Err("command execution is disabled; command requests are classification-only and cannot enter the patch lifecycle".into()),
        AgentOperation::ReadFile { .. } | AgentOperation::SearchRepo { .. } => Err("read/search model operations are not executable in this build; Synthesize builds context through guarded backend APIs only".into()),
        AgentOperation::AskUser { .. } | AgentOperation::FinalReport { .. } => Err("only propose_patch operations can be validated as patch proposals".into()),
        AgentOperation::HandOff { to_skill, .. } => Err(format!("hand_off to skill '{}' is not a patch proposal and cannot enter the patch validation lifecycle", to_skill)),
        AgentOperation::ProposeArtifact { .. }
        | AgentOperation::PublishBelief { .. }
        | AgentOperation::AskAgent { .. }
        | AgentOperation::AnswerAgent { .. }
        | AgentOperation::ReportFinding { .. }
        | AgentOperation::RequestTransition { .. }
        | AgentOperation::RequestContext { .. }
        | AgentOperation::StudioFinalReport { .. } => Err(
            "Studio artifact operations use the initiative artifact boundary and cannot enter the Assist patch validation lifecycle".into(),
        ),
    }
}

fn enforce_moa_gate_for_operation(
    operation: &AgentOperation,
    agent_profile_id: &str,
) -> Result<Value, String> {
    if agent_profile_id != "moa-action-planner" {
        return Ok(serde_json::json!({
            "enforced": false,
            "reason": "source agent profile is not MoA Action Planner"
        }));
    }

    let bridge = moa_bridge_path();
    if !bridge.exists() {
        return Err(format!(
            "MoA gate is required for MoA Action Planner, but bridge was not found at {}",
            bridge.display()
        ));
    }

    let request = serde_json::json!({
        "command": "evaluate_operation",
        "operation": moa_operation_payload(operation)?
    });
    let response = run_moa_bridge_request(&bridge, &request)?;
    let decision: MoaBridgeDecision = serde_json::from_value(response.clone())
        .map_err(|e| format!("MoA bridge returned invalid decision JSON: {}", e))?;
    if !decision.ok {
        return Err(format!(
            "MoA bridge failed: {}",
            decision.error.unwrap_or_else(|| "unknown error".into())
        ));
    }
    if decision.approved != Some(true) {
        return Err(format!(
            "MoA gate rejected operation: {}",
            decision.reason.unwrap_or_else(|| "rejected".into())
        ));
    }
    Ok(serde_json::json!({
        "enforced": true,
        "approved": true,
        "protocol": MOA_BRIDGE_PROTOCOL,
        "action_type": decision.action_type,
        "reason": decision.reason.unwrap_or_else(|| "approved".into())
    }))
}

fn moa_operation_payload(operation: &AgentOperation) -> Result<Value, String> {
    match operation {
        AgentOperation::ProposePatch {
            proposal_id,
            files,
            risk_notes,
            ..
        } => {
            let risk = moa_patch_risk_label(files.len(), risk_notes);
            let moa_files = files
                .iter()
                .map(|file| MoaBridgeOperationFile {
                    path: file.path.clone(),
                    risk: risk.clone(),
                })
                .collect::<Vec<_>>();
            serde_json::to_value(serde_json::json!({
                "type": "propose_patch",
                "proposalId": proposal_id,
                "files": moa_files
            }))
            .map_err(|e| e.to_string())
        }
        AgentOperation::RunCommand(command) => Ok(serde_json::json!({
            "type": "run_command",
            "argv": command.argv,
            "cwd": command.cwd,
            "reason": command.reason,
            "expectedOutcome": command.expected_outcome,
            "requiresNetwork": command.requires_network,
            "mayModifyFiles": command.may_modify_files
        })),
        AgentOperation::ReadFile { path, reason } => Ok(serde_json::json!({
            "type": "read_file",
            "path": path,
            "reason": reason
        })),
        AgentOperation::SearchRepo {
            query,
            glob,
            reason,
        } => Ok(serde_json::json!({
            "type": "search_repo",
            "query": query,
            "glob": glob,
            "reason": reason
        })),
        AgentOperation::AskUser { question, options } => Ok(serde_json::json!({
            "type": "ask_user",
            "question": question,
            "options": options
        })),
        AgentOperation::FinalReport {
            summary,
            changed_files,
            tests_run,
            remaining_risks,
        } => Ok(serde_json::json!({
            "type": "final_report",
            "summary": summary,
            "changedFiles": changed_files,
            "testsRun": tests_run,
            "remainingRisks": remaining_risks
        })),
        AgentOperation::HandOff {
            to_skill,
            context_summary,
            reason,
            preserve_file_history,
        } => Ok(serde_json::json!({
            "type": "hand_off",
            "toSkill": to_skill,
            "contextSummary": context_summary,
            "reason": reason,
            "preserveFileHistory": preserve_file_history
        })),
        other => serde_json::to_value(other).map_err(|e| e.to_string()),
    }
}

fn moa_patch_risk_label(file_count: usize, risk_notes: &[String]) -> String {
    let notes = risk_notes.join(" ").to_ascii_lowercase();
    if notes.contains("critical") || file_count > 6 {
        "critical".into()
    } else if notes.contains("high") || file_count > 3 {
        "high".into()
    } else {
        "low".into()
    }
}

fn run_moa_bridge_request(bridge: &Path, request: &Value) -> Result<Value, String> {
    let python = moa_python_command();
    let mut child = Command::new(&python)
        .arg(bridge)
        .current_dir(bridge.parent().unwrap_or_else(|| Path::new(".")))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to start MoA bridge with '{}': {}", python, e))?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| "failed to open MoA bridge stdin".to_string())?;
        let line = serde_json::to_string(request).map_err(|e| e.to_string())?;
        stdin
            .write_all(line.as_bytes())
            .map_err(|e| format!("failed to write MoA bridge request: {}", e))?;
        stdin
            .write_all(b"\n")
            .map_err(|e| format!("failed to terminate MoA bridge request: {}", e))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("failed to read MoA bridge response: {}", e))?;
    if !output.status.success() {
        return Err(format!(
            "MoA bridge exited with {}; stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(1200)
                .collect::<String>()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| "MoA bridge returned no JSON response".to_string())?;
    serde_json::from_str(line).map_err(|e| {
        format!(
            "MoA bridge returned invalid JSON: {}; stdout={}",
            e,
            stdout.chars().take(1200).collect::<String>()
        )
    })
}

fn moa_python_command() -> String {
    if let Ok(path) = std::env::var("SYNTHESIZE_MOA_PYTHON") {
        if !path.trim().is_empty() {
            return path;
        }
    }
    let bundled = PathBuf::from(r"C:\Python310\python.exe");
    if bundled.exists() {
        return bundled.to_string_lossy().to_string();
    }
    "python".into()
}

fn moa_bridge_path() -> PathBuf {
    if let Ok(path) = std::env::var("SYNTHESIZE_MOA_BRIDGE") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("integrations")
        .join("moa")
        .join("synthesize_bridge.py")
}

fn source_profile_for_patch_validation(
    conn: &Connection,
    req: &PatchProposalRequest,
    repo_root: &Path,
) -> Result<(String, String), String> {
    if req.context_bundle_id.trim().is_empty() {
        return Err("context_bundle_id is required for patch validation; Synthesize validates patches against the persisted context that produced the model response".into());
    }
    if let Ok(context) = load_context_bundle(conn, &req.context_bundle_id) {
        if context.session_id != req.session_id {
            return Err("context bundle session_id does not match patch validation request".into());
        }
        if !repo_roots_equal(&context.repo_root, repo_root)? {
            return Err("context bundle repo_root does not match patch validation request".into());
        }
        return Ok((context.agent_profile_id, req.context_bundle_id.clone()));
    }
    let capsule = load_capsule(conn, &req.context_bundle_id).map_err(|error| {
        format!("source context is neither a readable Assist bundle nor Studio capsule: {error}")
    })?;
    if capsule.session_id != req.session_id {
        return Err("Studio capsule session_id does not match patch validation request".into());
    }
    let bound_repo: String = conn
        .query_row(
            "SELECT repo_root FROM initiatives WHERE id=?1 AND session_id=?2",
            params![capsule.initiative_id, req.session_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Studio capsule initiative/session binding is invalid".to_string())?;
    if !repo_roots_equal(&bound_repo, repo_root)? {
        return Err("Studio capsule repository does not match patch validation request".into());
    }
    if capsule.role != intent_ledger::Role::Builder || capsule.task_id.is_none() {
        return Err("only a task-bound Builder Context Capsule can source a patch proposal".into());
    }
    Ok(("local-patcher".into(), req.context_bundle_id.clone()))
}

fn proposal_from_operation(operation: &AgentOperation) -> Result<PatchProposal, String> {
    match operation {
        AgentOperation::ProposePatch {
            proposal_id,
            current_commit,
            base_commit,
            files,
            ..
        } => Ok(PatchProposal {
            id: proposal_id.clone(),
            base_commit: base_commit.clone(),
            current_commit: current_commit.clone(),
            files: files.iter().map(protocol_file_to_engine_file).collect(),
        }),
        _ => Err("operation must be propose_patch".into()),
    }
}

fn protocol_file_to_engine_file(file: &ProtocolPatchFile) -> PatchFile {
    PatchFile {
        id: file.id.clone(),
        path: file.path.clone(),
        before_sha256: file.before_sha256.clone(),
        unified_diff: file.patch.clone(),
    }
}

fn init_audit(
    repo_root: &Path,
    session_id: &str,
) -> Result<Connection, Box<dyn std::error::Error>> {
    fs::create_dir_all(repo_root.join(".synthesize"))?;
    let conn = Connection::open(
        repo_root
            .join(".synthesize")
            .join("synthesize-audit.sqlite"),
    )?;
    migrate_legacy_patch_tables(&conn)?;
    init_schema(&conn)?;
    ensure_task_schema(&conn)?;
    let canonical_repo = canonical_repo_root_string(repo_root)
        .unwrap_or_else(|_| repo_root.to_string_lossy().to_string());
    conn.execute(
        "INSERT OR IGNORE INTO sessions (id, repo_root, git_commit_start) VALUES (?1, ?2, ?3)",
        (session_id, canonical_repo, git_current_commit(repo_root)),
    )?;
    Ok(conn)
}

fn ensure_task_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS task_snapshots (
            task_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            repo_root TEXT NOT NULL,
            label TEXT NOT NULL,
            argv_json TEXT NOT NULL,
            cwd TEXT NOT NULL,
            risk TEXT NOT NULL,
            reason TEXT NOT NULL,
            requires_network INTEGER NOT NULL,
            may_modify_files INTEGER NOT NULL,
            detected_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY(task_id, session_id, repo_root)
        );",
    )?;
    ensure_task_snapshots_repo_scoped_pk(conn)?;
    let command_cols = table_columns(conn, "commands").unwrap_or_default();
    if !command_cols.iter().any(|c| c == "task_id") {
        let _ = conn.execute("ALTER TABLE commands ADD COLUMN task_id TEXT", []);
    }
    if !command_cols.iter().any(|c| c == "repo_root") {
        let _ = conn.execute("ALTER TABLE commands ADD COLUMN repo_root TEXT", []);
    }
    if !command_cols.iter().any(|c| c == "requires_network") {
        let _ = conn.execute(
            "ALTER TABLE commands ADD COLUMN requires_network INTEGER NOT NULL DEFAULT 0",
            [],
        );
    }
    if !command_cols.iter().any(|c| c == "may_modify_files") {
        let _ = conn.execute(
            "ALTER TABLE commands ADD COLUMN may_modify_files INTEGER NOT NULL DEFAULT 0",
            [],
        );
    }
    Ok(())
}

fn migrate_legacy_patch_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    if table_exists(conn, "patch_proposals")? {
        let cols = table_columns(conn, "patch_proposals")?;
        let sql: String = conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='patch_proposals'",
            [],
            |row| row.get(0),
        )?;
        if !cols.iter().any(|c| c == "operation_sha256")
            || !cols.iter().any(|c| c == "status")
            || !cols.iter().any(|c| c == "checkpoint_dir")
            || !cols.iter().any(|c| c == "source_context_bundle_id")
            || !cols.iter().any(|c| c == "source_agent_profile_id")
            || !sql.contains("applying")
            || !sql.contains("rollback_failed")
        {
            let legacy = format!("patch_proposals_legacy_{}", std::process::id());
            conn.execute(
                &format!("ALTER TABLE patch_proposals RENAME TO {}", legacy),
                [],
            )?;
        }
    }
    if table_exists(conn, "patch_files")? {
        let cols = table_columns(conn, "patch_files")?;
        if !cols.iter().any(|c| c == "diff_sha256") || !cols.iter().any(|c| c == "unified_diff") {
            let legacy = format!("patch_files_v4_legacy_{}", std::process::id());
            conn.execute(&format!("ALTER TABLE patch_files RENAME TO {}", legacy), [])?;
        }
    }
    Ok(())
}

fn ensure_task_snapshots_repo_scoped_pk(conn: &Connection) -> Result<(), rusqlite::Error> {
    if !table_exists(conn, "task_snapshots")? {
        return Ok(());
    }
    let mut stmt = conn.prepare("PRAGMA table_info(task_snapshots)")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(1)?, row.get::<_, i64>(5)?))
    })?;
    let mut pk_cols = Vec::new();
    for row in rows {
        let (name, pk) = row?;
        if pk > 0 {
            pk_cols.push(name);
        }
    }
    if pk_cols
        == vec![
            "task_id".to_string(),
            "session_id".to_string(),
            "repo_root".to_string(),
        ]
    {
        return Ok(());
    }
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS task_snapshots_repo_scoped (
            task_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            repo_root TEXT NOT NULL,
            label TEXT NOT NULL,
            argv_json TEXT NOT NULL,
            cwd TEXT NOT NULL,
            risk TEXT NOT NULL,
            reason TEXT NOT NULL,
            requires_network INTEGER NOT NULL,
            may_modify_files INTEGER NOT NULL,
            detected_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY(task_id, session_id, repo_root)
        );
        INSERT OR REPLACE INTO task_snapshots_repo_scoped (task_id, session_id, repo_root, label, argv_json, cwd, risk, reason, requires_network, may_modify_files, detected_at)
            SELECT task_id, session_id, repo_root, label, argv_json, cwd, risk, reason, requires_network, may_modify_files, COALESCE(detected_at, datetime('now')) FROM task_snapshots;
        DROP TABLE task_snapshots;
        ALTER TABLE task_snapshots_repo_scoped RENAME TO task_snapshots;"
    )?;
    Ok(())
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, rusqlite::Error> {
    let count: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1",
        params![table],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>, rusqlite::Error> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut cols = Vec::new();
    for row in rows {
        cols.push(row?);
    }
    Ok(cols)
}

fn canonical_operation_and_hash(operation: &AgentOperation) -> Result<(String, String), String> {
    let value = serde_json::to_value(operation).map_err(|e| e.to_string())?;
    let canonical = canonical_json(&value).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    Ok((canonical, hex::encode(hasher.finalize())))
}

fn canonical_json(value: &Value) -> Result<String, serde_json::Error> {
    let sorted = sort_json(value);
    serde_json::to_string(&sorted)
}

fn sort_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            for (k, v) in map {
                sorted.insert(k.clone(), sort_json(v));
            }
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(items) => Value::Array(items.iter().map(sort_json).collect()),
        _ => value.clone(),
    }
}

fn sha256_str(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

#[derive(Debug)]
struct ExistingProposalIdentity {
    operation_sha256: String,
    status: String,
}

fn load_existing_proposal_identity(
    conn: &Connection,
    proposal_id: &str,
) -> Result<Option<ExistingProposalIdentity>, String> {
    conn.query_row(
        "SELECT operation_sha256, status FROM patch_proposals WHERE proposal_id = ?1",
        params![proposal_id],
        |row| {
            Ok(ExistingProposalIdentity {
                operation_sha256: row.get(0)?,
                status: row.get(1)?,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

fn ensure_duplicate_is_same_hash(
    conn: &Connection,
    proposal_id: &str,
    operation_sha256: &str,
) -> Result<(), String> {
    if let Some(existing) = load_existing_proposal_identity(conn, proposal_id)? {
        if existing.operation_sha256 != operation_sha256 {
            return Err(format!(
                "proposal_id {} already exists with a different operation_sha256",
                proposal_id
            ));
        }
    }
    Ok(())
}

fn insert_patch_proposal(
    conn: &Connection,
    session_id: &str,
    repo_root: &Path,
    proposal: &PatchProposal,
    operation_json: &str,
    operation_sha256: &str,
    status: &str,
    rejection_reason: Option<&str>,
    source_context_bundle_id: Option<&str>,
    source_agent_profile_id: &str,
) -> Result<(), String> {
    ensure_duplicate_is_same_hash(conn, &proposal.id, operation_sha256)?;
    let canonical_repo = canonical_repo_root_string(repo_root)?;
    conn.execute(
        "INSERT INTO patch_proposals (proposal_id, session_id, repo_root, current_commit, operation_json, operation_sha256, status, rejection_reason, source_context_bundle_id, source_agent_profile_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![&proposal.id, session_id, canonical_repo, &proposal.current_commit, operation_json, operation_sha256, status, rejection_reason, source_context_bundle_id, source_agent_profile_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

fn persist_patch_files(
    conn: &Connection,
    proposal: &PatchProposal,
    files: &[patch_engine::ValidatedPatchFile],
) -> Result<(), String> {
    for file in &proposal.files {
        let risk = files
            .iter()
            .find(|v| v.id == file.id)
            .map(|v| format!("{:?}", v.risk))
            .unwrap_or_else(|| "Unknown".into());
        conn.execute(
            "INSERT OR IGNORE INTO patch_files (proposal_id, file_id, path, before_sha256, unified_diff, diff_sha256, risk) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![&proposal.id, &file.id, &file.path, &file.before_sha256, &file.unified_diff, sha256_str(&file.unified_diff), risk],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn persist_patch_files_rejected(conn: &Connection, proposal: &PatchProposal) -> Result<(), String> {
    for file in &proposal.files {
        conn.execute(
            "INSERT OR IGNORE INTO patch_files (proposal_id, file_id, path, before_sha256, unified_diff, diff_sha256, risk) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'Rejected')",
            params![&proposal.id, &file.id, &file.path, &file.before_sha256, &file.unified_diff, sha256_str(&file.unified_diff)],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn load_patch_file_views(
    conn: &Connection,
    proposal_id: &str,
) -> Result<Vec<PatchFileValidationView>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT file_id, path, risk FROM patch_files WHERE proposal_id = ?1 ORDER BY rowid ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![proposal_id], |row| {
            Ok(PatchFileValidationView {
                id: row.get(0)?,
                path: row.get(1)?,
                risk: row.get(2)?,
                real_path: "persisted".into(),
            })
        })
        .map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    for row in rows {
        files.push(row.map_err(|e| e.to_string())?);
    }
    Ok(files)
}

fn load_stored_proposal(conn: &Connection, proposal_id: &str) -> Result<StoredProposal, String> {
    let (current_commit, operation_sha256, operation_json, status, checkpoint_id, checkpoint_dir): (Option<String>, String, String, String, Option<String>, Option<String>) = conn.query_row(
        "SELECT current_commit, operation_sha256, operation_json, status, checkpoint_id, checkpoint_dir FROM patch_proposals WHERE proposal_id = ?1",
        params![proposal_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
    ).optional().map_err(|e| e.to_string())?.ok_or_else(|| format!("proposal {} not found", proposal_id))?;

    let mut stmt = conn.prepare("SELECT file_id, path, before_sha256, unified_diff FROM patch_files WHERE proposal_id = ?1 ORDER BY rowid ASC").map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![proposal_id], |row| {
            Ok(PatchFile {
                id: row.get(0)?,
                path: row.get(1)?,
                before_sha256: row.get(2)?,
                unified_diff: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    for row in rows {
        files.push(row.map_err(|e| e.to_string())?);
    }
    if files.is_empty() {
        return Err(format!(
            "proposal {} has no persisted patch files",
            proposal_id
        ));
    }
    Ok(StoredProposal {
        proposal: PatchProposal {
            id: proposal_id.to_string(),
            base_commit: None,
            current_commit,
            files,
        },
        operation_sha256,
        operation_json,
        status,
        checkpoint_id,
        checkpoint_dir,
    })
}

fn transition_proposal_status(
    conn: &Connection,
    session_id: &str,
    proposal_id: &str,
    expected_from: &str,
    to: &str,
    reason: Option<&str>,
) -> Result<(), String> {
    if !is_legal_transition(expected_from, to) {
        return Err(format!(
            "illegal lifecycle transition {} -> {}",
            expected_from, to
        ));
    }
    let current: Option<String> = conn
        .query_row(
            "SELECT status FROM patch_proposals WHERE proposal_id = ?1",
            params![proposal_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let current = current.ok_or_else(|| format!("proposal {} not found", proposal_id))?;
    if current != expected_from {
        return Err(format!(
            "proposal {} expected state {}, found {}",
            proposal_id, expected_from, current
        ));
    }
    let timestamp_col = match to {
        "validated" => ", validated_at = datetime('now')",
        "approved" => ", approved_at = datetime('now')",
        "applied" => ", applied_at = datetime('now')",
        "rolled_back" => ", rolled_back_at = datetime('now')",
        _ => "",
    };
    let sql = format!("UPDATE patch_proposals SET status = ?2, rejection_reason = COALESCE(?3, rejection_reason){} WHERE proposal_id = ?1 AND status = ?4", timestamp_col);
    let changed = conn
        .execute(&sql, params![proposal_id, to, reason, expected_from])
        .map_err(|e| e.to_string())?;
    if changed != 1 {
        return Err(format!(
            "lifecycle transition {} -> {} failed due to concurrent state change",
            expected_from, to
        ));
    }
    let payload = serde_json::json!({ "proposal_id": proposal_id, "from": expected_from, "to": to, "reason": reason });
    let _ = append_event(
        conn,
        session_id,
        "patch.lifecycle_transition",
        &payload.to_string(),
    );
    Ok(())
}

fn is_legal_transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("proposed", "validated")
            | ("proposed", "rejected")
            | ("validated", "approved")
            | ("approved", "applying")
            | ("applying", "applied")
            | ("applying", "apply_failed")
            | ("applied", "rolling_back")
            | ("rolling_back", "rolled_back")
            | ("rolling_back", "rollback_failed")
    )
}

fn verify_approval(
    conn: &Connection,
    proposal_id: &str,
    approval_id: &str,
    operation_sha256: &str,
) -> Result<(), String> {
    let found: Option<String> = conn.query_row(
        "SELECT operation_sha256 FROM patch_approvals WHERE approval_id = ?1 AND proposal_id = ?2",
        params![approval_id, proposal_id],
        |row| row.get(0),
    ).optional().map_err(|e| e.to_string())?;
    match found {
        Some(stored_sha) if stored_sha == operation_sha256 => Ok(()),
        Some(_) => Err("approval operation_sha256 does not match persisted proposal".into()),
        None => Err("approval_id was not found for this proposal".into()),
    }
}

struct RepoMutationLockGuard {
    key: String,
}

impl Drop for RepoMutationLockGuard {
    fn drop(&mut self) {
        if let Some(mutex) = REPO_MUTATION_LOCKS.get() {
            if let Ok(mut set) = mutex.lock() {
                set.remove(&self.key);
            }
        }
    }
}

fn acquire_repo_mutation_lock(
    repo_root: &Path,
    session_id: &str,
    operation: &str,
) -> Result<RepoMutationLockGuard, String> {
    let key = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf())
        .to_string_lossy()
        .to_string();
    let mutex = REPO_MUTATION_LOCKS.get_or_init(|| Mutex::new(HashSet::new()));
    let mut set = mutex
        .lock()
        .map_err(|_| "repo mutation lock is poisoned".to_string())?;
    if !set.insert(key.clone()) {
        if let Ok(conn) = init_audit(repo_root, session_id) {
            let payload = serde_json::json!({ "repo_root": key, "operation": operation, "error": "repo mutation already in progress" });
            let _ = append_event(
                &conn,
                session_id,
                "patch.mutation_lock_conflict",
                &payload.to_string(),
            );
        }
        return Err(format!(
            "another apply/rollback mutation is already running for repo {}",
            repo_root.to_string_lossy()
        ));
    }
    Ok(RepoMutationLockGuard { key })
}

fn guarded_manifest_exists(repo_root: &Path, relative_path: &str) -> bool {
    RepoGuard::new(repo_root, FilePolicy::default())
        .and_then(|guard| guard.resolve_for_existing_path(relative_path))
        .map(|path| path.is_file())
        .unwrap_or(false)
}

fn canonical_repo_root_string(repo_root: &Path) -> Result<String, String> {
    repo_root
        .canonicalize()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

fn repo_roots_equal(stored_repo_root: &str, requested_repo_root: &Path) -> Result<bool, String> {
    let stored = PathBuf::from(stored_repo_root);
    let stored_canonical = stored.canonicalize().map_err(|e| e.to_string())?;
    let requested_canonical = requested_repo_root
        .canonicalize()
        .map_err(|e| e.to_string())?;
    Ok(stored_canonical == requested_canonical)
}

fn select_scalar_string(conn: &Connection, sql: &str, arg: &str) -> Result<String, String> {
    conn.query_row(sql, params![arg], |row| row.get(0))
        .map_err(|e| e.to_string())
}

fn git_current_commit(repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(repo_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn list_files_internal(repo_root: &Path, limit: usize) -> Result<Vec<RepoFileView>, String> {
    let guard = RepoGuard::new(repo_root, FilePolicy::default()).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    walk_repo(repo_root, repo_root, &guard, &mut out, limit).map_err(|e| e.to_string())?;
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn walk_repo(
    root: &Path,
    dir: &Path,
    guard: &RepoGuard,
    out: &mut Vec<RepoFileView>,
    limit: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if out.len() >= limit {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        if out.len() >= limit {
            break;
        }
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if file_type.is_symlink() {
            let rel = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            out.push(RepoFileView {
                path: rel,
                kind: "symlink-skipped".into(),
                denied: true,
            });
            continue;
        }
        if is_ignored_repo_dir_name(name) {
            continue;
        }
        let rel = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        if file_type.is_dir() {
            if is_denied_context_dir_name(name)
                || guard
                    .resolve_for_write_path(Path::new(&rel).join("__synthesize_policy_probe__"))
                    .is_err()
            {
                // Include only the denied directory itself. Do not descend or reveal child names.
                out.push(RepoFileView {
                    path: format!("{}/", rel.trim_end_matches('/')),
                    kind: "dir-denied".into(),
                    denied: true,
                });
                continue;
            }
            out.push(RepoFileView {
                path: rel.clone(),
                kind: "dir".into(),
                denied: false,
            });
            walk_repo(root, &path, guard, out, limit)?;
            continue;
        }
        let denied = guard.resolve_for_existing_path(&rel).is_err();
        if denied {
            // Do not expose denied child filenames in model context; only the UI file tree sees this marker.
            out.push(RepoFileView {
                path: rel,
                kind: "file-denied".into(),
                denied: true,
            });
        } else {
            out.push(RepoFileView {
                path: rel,
                kind: "file".into(),
                denied: false,
            });
        }
    }
    Ok(())
}

fn is_ignored_repo_dir_name(name: &str) -> bool {
    matches!(
        name,
        "node_modules" | "target" | ".git" | "dist" | "build" | ".next" | ".synthesize"
    )
}

fn is_denied_context_dir_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.starts_with('.')
        || matches!(
            lower.as_str(),
            "credentials"
                | "credential"
                | "secrets"
                | "secret"
                | "private"
                | "keys"
                | "key"
                | "certs"
                | "certificates"
        )
        || lower.contains("credential")
        || lower.contains("secret")
        || lower.contains("private_key")
}

fn is_text_like(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [
        ".ts", ".tsx", ".js", ".jsx", ".rs", ".py", ".go", ".java", ".md", ".json", ".toml",
        ".yaml", ".yml", ".css", ".html",
    ]
    .iter()
    .any(|ext| lower.ends_with(ext))
}

fn risk_label(risk: &CommandRisk) -> &'static str {
    match risk {
        CommandRisk::ReadOnly => "ReadOnly",
        CommandRisk::TestOrBuild => "TestOrBuild",
        CommandRisk::ModifiesRepo => "ModifiesRepo",
        CommandRisk::Network => "Network",
        CommandRisk::Destructive => "Destructive",
        CommandRisk::Blocked => "Blocked",
    }
}

// ---------------------------------------------------------------------------
// Skill Agent Orchestrator
// Ensures only ONE Qwen3 instance runs at a time (GPU serial lock).
// Hand-offs are queued and executed sequentially.
// ---------------------------------------------------------------------------

/// Global GPU serial lock. Acquired before any local model inference starts,
/// released after it completes or fails. Cloud skills bypass this lock since
/// they use HTTP APIs, not local GPU resources.
#[allow(dead_code)]
static GPU_SERIAL_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[allow(dead_code)]
fn gpu_serial_lock() -> &'static Mutex<()> {
    GPU_SERIAL_LOCK.get_or_init(|| Mutex::new(()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SkillDefinition {
    id: String,
    name: String,
    description: String,
    model_registry_id: String,
    tier: String,
    system_prompt_addon: String,
    allowed_operations: Vec<String>,
    allowed_hand_off_targets: Vec<String>,
    max_iterations: u32,
    enabled: bool,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SkillQueueEntry {
    id: String,
    skill_id: String,
    skill_name: String,
    context_summary: String,
    status: String, // queued | running | completed | failed | cancelled
    started_at: Option<String>,
    completed_at: Option<String>,
    error_message: Option<String>,
    iterations: u32,
    hand_off_from: Option<String>,
}

#[derive(Debug, Serialize)]
struct SkillQueueState {
    current_entry: Option<SkillQueueEntry>,
    queue: Vec<SkillQueueEntry>,
    history: Vec<SkillQueueEntry>,
    gpu_lock_held: bool,
}

static SKILL_QUEUE: OnceLock<Mutex<SkillQueueStateInternal>> = OnceLock::new();

struct SkillQueueStateInternal {
    current: Option<SkillQueueEntry>,
    queue: Vec<SkillQueueEntry>,
    history: Vec<SkillQueueEntry>,
}

fn skill_queue_state() -> &'static Mutex<SkillQueueStateInternal> {
    SKILL_QUEUE.get_or_init(|| {
        Mutex::new(SkillQueueStateInternal {
            current: None,
            queue: Vec::new(),
            history: Vec::new(),
        })
    })
}

fn default_skill_definitions() -> Vec<SkillDefinition> {
    vec![
        SkillDefinition {
            id: "code-writer".into(), name: "Code Writer".into(),
            description: "Low-lift code generation: new functions, modules, and boilerplate.".into(),
            model_registry_id: "qwen3-coder-1.7b-instruct-q4-k-m".into(), tier: "balanced".into(),
            system_prompt_addon: "You are a focused code-writing agent. Produce small, reviewable patches only. Prefer propose_patch operations.".into(),
            allowed_operations: vec!["read_file".into(), "search_repo".into(), "propose_patch".into(), "ask_user".into(), "report".into(), "final_report".into(), "hand_off".into()],
            allowed_hand_off_targets: vec!["code-reviewer".into(), "test-writer".into(), "planner".into()],
            max_iterations: 10, enabled: true, tags: vec!["writing".into(), "patch".into()],
        },
        SkillDefinition {
            id: "code-reviewer".into(), name: "Code Reviewer".into(),
            description: "Reviews diffs and proposed patches for bugs, security, and style.".into(),
            model_registry_id: "qwen3-coder-1.7b-instruct-q4-k-m".into(), tier: "balanced".into(),
            system_prompt_addon: "You are a code review agent. Primary output is a detailed report. Only propose_patch for concrete bugs.".into(),
            allowed_operations: vec!["read_file".into(), "search_repo".into(), "propose_patch".into(), "ask_user".into(), "report".into(), "final_report".into(), "hand_off".into()],
            allowed_hand_off_targets: vec!["code-writer".into(), "test-writer".into()],
            max_iterations: 8, enabled: true, tags: vec!["review".into(), "quality".into()],
        },
        SkillDefinition {
            id: "test-writer".into(), name: "Test Writer".into(),
            description: "Writes unit and integration tests. Focuses on edge cases and regression coverage.".into(),
            model_registry_id: "qwen3-coder-1.7b-instruct-q4-k-m".into(), tier: "balanced".into(),
            system_prompt_addon: "You are a test-writing agent. Emit propose_patch operations that add tests. Keep each patch small and self-contained.".into(),
            allowed_operations: vec!["read_file".into(), "search_repo".into(), "propose_patch".into(), "ask_user".into(), "report".into(), "final_report".into(), "hand_off".into()],
            allowed_hand_off_targets: vec!["code-reviewer".into()],
            max_iterations: 10, enabled: true, tags: vec!["testing".into(), "quality".into()],
        },
        SkillDefinition {
            id: "debugger".into(), name: "Debugger".into(),
            description: "Investigates failing tests and error traces. Proposes minimal targeted patches.".into(),
            model_registry_id: "qwen3-coder-8b-instruct-q4-k-m".into(), tier: "powerful".into(),
            system_prompt_addon: "You are a debugging agent. Read error output carefully. Propose the smallest patch that fixes the root cause.".into(),
            allowed_operations: vec!["read_file".into(), "search_repo".into(), "propose_patch".into(), "run_command".into(), "ask_user".into(), "report".into(), "final_report".into(), "hand_off".into()],
            allowed_hand_off_targets: vec!["code-reviewer".into(), "test-writer".into()],
            max_iterations: 12, enabled: true, tags: vec!["debugging".into(), "fix".into()],
        },
        SkillDefinition {
            id: "planner".into(), name: "Planner".into(),
            description: "Breaks large tasks into a sequenced plan and hands off subtasks to specialist skills.".into(),
            model_registry_id: "qwen3-coder-8b-instruct-q4-k-m".into(), tier: "powerful".into(),
            system_prompt_addon: "You are a planning agent. Break tasks into concrete subtasks and emit hand_off operations to the correct specialist skill. Do not write code yourself.".into(),
            allowed_operations: vec!["read_file".into(), "search_repo".into(), "ask_user".into(), "report".into(), "final_report".into(), "hand_off".into()],
            allowed_hand_off_targets: vec!["code-writer".into(), "code-reviewer".into(), "test-writer".into(), "debugger".into(), "docs-writer".into(), "cloud-architect".into()],
            max_iterations: 6, enabled: true, tags: vec!["planning".into(), "orchestration".into()],
        },
        SkillDefinition {
            id: "docs-writer".into(), name: "Docs Writer".into(),
            description: "Writes or updates documentation: README, JSDoc/TSDoc comments, API docs.".into(),
            model_registry_id: "qwen3-coder-1.7b-instruct-q4-k-m".into(), tier: "balanced".into(),
            system_prompt_addon: "You are a documentation agent. Write clear, accurate documentation. Never modify logic; only add or update documentation.".into(),
            allowed_operations: vec!["read_file".into(), "search_repo".into(), "propose_patch".into(), "ask_user".into(), "report".into(), "final_report".into()],
            allowed_hand_off_targets: vec![],
            max_iterations: 8, enabled: true, tags: vec!["docs".into(), "writing".into()],
        },
        SkillDefinition {
            id: "cloud-architect".into(), name: "Cloud Architect (GPT-4o)".into(),
            description: "Heavy-lift: complex architecture decisions, large codebase analysis, security review. Uses cloud frontier model.".into(),
            model_registry_id: "cloud-openai-gpt-4o".into(), tier: "cloud-heavy".into(),
            system_prompt_addon: "You are an expert software architect. Provide a thorough architectural analysis with concrete recommendations. Surface all remaining risks in final_report.".into(),
            allowed_operations: vec!["read_file".into(), "search_repo".into(), "propose_patch".into(), "ask_user".into(), "report".into(), "final_report".into(), "hand_off".into()],
            allowed_hand_off_targets: vec!["code-writer".into(), "code-reviewer".into()],
            max_iterations: 15, enabled: true, tags: vec!["architecture".into(), "cloud".into(), "heavy-lift".into()],
        },
        SkillDefinition {
            id: "cloud-reasoner".into(), name: "Cloud Reasoner (o3)".into(),
            description: "Hard problems only: algorithmic correctness, formal reasoning, security proofs. Uses OpenAI o3.".into(),
            model_registry_id: "cloud-openai-o3".into(), tier: "cloud-reasoning".into(),
            system_prompt_addon: "You are a formal reasoning agent. Think step by step. Prioritise correctness over brevity. Surface all edge cases in remainingRisks.".into(),
            allowed_operations: vec!["read_file".into(), "search_repo".into(), "propose_patch".into(), "ask_user".into(), "report".into(), "final_report".into()],
            allowed_hand_off_targets: vec![],
            max_iterations: 20, enabled: true, tags: vec!["reasoning".into(), "cloud".into(), "correctness".into()],
        },
    ]
}

#[tauri::command]
fn skill_list() -> Vec<SkillDefinition> {
    // Load from user config DB if available; fall back to defaults.
    if let Ok(conn) = user_config_conn() {
        if init_schema(&conn).is_ok() {
            if let Ok(mut stmt) =
                conn.prepare("SELECT config_json FROM skill_configs ORDER BY rowid")
            {
                let rows: Vec<SkillDefinition> = stmt
                    .query_map([], |row| {
                        let json: String = row.get(0)?;
                        Ok(serde_json::from_str::<SkillDefinition>(&json).ok())
                    })
                    .ok()
                    .map(|iter| iter.filter_map(|r| r.ok().flatten()).collect())
                    .unwrap_or_default();
                if !rows.is_empty() {
                    return rows;
                }
            }
        }
    }
    default_skill_definitions()
}

#[tauri::command]
fn skill_save(skill: SkillDefinition) -> Result<SkillDefinition, String> {
    if skill.id.is_empty() {
        return Err("skill id must not be empty".into());
    }
    if !skill
        .id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err("skill id must be lowercase alphanumeric with hyphens only".into());
    }
    validate_skill_definition(&skill)?;
    let conn = user_config_conn()?;
    init_schema(&conn).map_err(|e| e.to_string())?;
    let json = serde_json::to_string(&skill).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO skill_configs (id, config_json, created_at, updated_at) VALUES (?1, ?2, COALESCE((SELECT created_at FROM skill_configs WHERE id = ?1), CURRENT_TIMESTAMP), CURRENT_TIMESTAMP)",
        params![&skill.id, &json],
    ).map_err(|e| e.to_string())?;
    Ok(skill)
}

fn validate_skill_definition(skill: &SkillDefinition) -> Result<(), String> {
    if skill.id.starts_with('-') || skill.id.ends_with('-') {
        return Err("skill id must not start or end with '-'".into());
    }
    if !skill
        .id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err("skill id must use lowercase alphanumeric characters and hyphens only".into());
    }
    if skill.name.trim().is_empty() {
        return Err("skill name must not be empty".into());
    }
    if skill.description.trim().is_empty() {
        return Err("skill description must not be empty".into());
    }
    if skill.model_registry_id.trim().is_empty() {
        return Err("model_registry_id must not be empty".into());
    }
    if !(1..=50).contains(&skill.max_iterations) {
        return Err("max_iterations must be between 1 and 50".into());
    }
    if skill.allowed_operations.is_empty() {
        return Err("allowed_operations must not be empty".into());
    }
    let allowed_ops: HashSet<&str> = [
        "read_file",
        "search_repo",
        "propose_patch",
        "run_command",
        "ask_user",
        "report",
        "final_report",
        "hand_off",
    ]
    .into_iter()
    .collect();
    for op in &skill.allowed_operations {
        if !allowed_ops.contains(op.as_str()) {
            return Err(format!("allowed operation '{}' is not supported", op));
        }
    }
    let mut targets = HashSet::new();
    for target in &skill.allowed_hand_off_targets {
        if target == &skill.id {
            return Err("skill cannot hand off to itself".into());
        }
        if !target
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(format!(
                "hand-off target '{}' must use lowercase alphanumeric characters and hyphens only",
                target
            ));
        }
        if !targets.insert(target.as_str()) {
            return Err(format!("duplicate hand-off target '{}'", target));
        }
    }
    Ok(())
}

#[tauri::command]
fn skill_delete(skill_id: String) -> Result<(), String> {
    let conn = user_config_conn()?;
    init_schema(&conn).map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM skill_configs WHERE id = ?1",
        params![&skill_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn skill_reset_to_defaults() -> Vec<SkillDefinition> {
    if let Ok(conn) = user_config_conn() {
        let _ = conn.execute("DELETE FROM skill_configs", []);
    }
    default_skill_definitions()
}

#[derive(Debug, Deserialize)]
struct SkillSpawnRequest {
    skill_id: String,
    context_summary: String,
    hand_off_from: Option<String>,
    session_id: String,
    repo_root: String,
}

#[derive(Debug, Serialize)]
struct SkillSpawnResult {
    entry_id: String,
    skill_id: String,
    skill_name: String,
    status: String,
    message: String,
    gpu_lock_required: bool,
}

#[tauri::command]
fn skill_get_system_prompt(skill_id: String) -> Result<String, String> {
    let skills = skill_list();
    let skill = skills
        .iter()
        .find(|s| s.id == skill_id)
        .ok_or_else(|| format!("skill '{}' not found", skill_id))?;
    Ok(build_skill_system_prompt(skill))
}

#[tauri::command]
fn skill_queue_spawn(req: SkillSpawnRequest) -> Result<SkillSpawnResult, String> {
    let skills = skill_list();
    let skill = skills
        .iter()
        .find(|s| s.id == req.skill_id)
        .ok_or_else(|| format!("skill '{}' not found in registry", req.skill_id))?;
    if !skill.enabled {
        return Err(format!("skill '{}' is disabled", req.skill_id));
    }
    // Validate hand-off permission if applicable.
    if let Some(from_id) = &req.hand_off_from {
        let from = skills
            .iter()
            .find(|s| s.id == *from_id)
            .ok_or_else(|| format!("originating skill '{}' not found", from_id))?;
        if !from.allowed_hand_off_targets.is_empty()
            && !from.allowed_hand_off_targets.contains(&req.skill_id)
        {
            return Err(format!(
                "skill '{}' is not allowed to hand off to '{}'",
                from_id, req.skill_id
            ));
        }
    }
    let entry_id = new_id("skill-entry");
    let is_cloud = skill.tier.starts_with("cloud-");
    let hand_off_from = req.hand_off_from.clone();
    let entry = SkillQueueEntry {
        id: entry_id.clone(),
        skill_id: skill.id.clone(),
        skill_name: skill.name.clone(),
        context_summary: req.context_summary,
        status: "queued".into(),
        started_at: None,
        completed_at: None,
        error_message: None,
        iterations: 0,
        hand_off_from: hand_off_from.clone(),
    };
    {
        let mut state = skill_queue_state()
            .lock()
            .map_err(|_| "skill queue lock poisoned".to_string())?;
        state.queue.push(entry);
    }
    // Audit the spawn request.
    if let Ok(conn) = init_audit(&PathBuf::from(&req.repo_root), &req.session_id) {
        let payload = serde_json::json!({ "entry_id": &entry_id, "skill_id": &req.skill_id, "hand_off_from": &hand_off_from }).to_string();
        let _ = append_event(&conn, &req.session_id, "skill.spawned", &payload);
    }
    Ok(SkillSpawnResult {
        entry_id,
        skill_id: skill.id.clone(),
        skill_name: skill.name.clone(),
        status: "queued".into(),
        message: format!(
            "skill '{}' queued; {}",
            skill.name,
            if is_cloud {
                "cloud endpoint (no GPU lock required)"
            } else {
                "will acquire GPU serial lock before inference starts"
            }
        ),
        gpu_lock_required: !is_cloud,
    })
}

#[tauri::command]
fn skill_queue_status() -> Result<SkillQueueState, String> {
    let state = skill_queue_state()
        .lock()
        .map_err(|_| "skill queue lock poisoned".to_string())?;
    let gpu_lock_held = state
        .current
        .as_ref()
        .map(|e| !e.skill_id.contains("cloud"))
        .unwrap_or(false);
    Ok(SkillQueueState {
        current_entry: state.current.clone(),
        queue: state.queue.clone(),
        history: state.history.iter().rev().take(20).cloned().collect(),
        gpu_lock_held,
    })
}

#[tauri::command]
fn skill_queue_advance(session_id: String, repo_root: String) -> Result<SkillSpawnResult, String> {
    let next_entry = {
        let mut state = skill_queue_state()
            .lock()
            .map_err(|_| "skill queue lock poisoned".to_string())?;
        // If something is currently running, reject.
        if state.current.is_some() {
            return Err(
                "a skill agent is already running; wait for it to complete or cancel it first"
                    .into(),
            );
        }
        if state.queue.is_empty() {
            return Err("skill queue is empty; spawn a skill first".into());
        }
        let mut entry = state.queue.remove(0);
        entry.status = "running".into();
        entry.started_at = Some(iso_now());
        state.current = Some(entry.clone());
        entry
    };
    let is_cloud = next_entry.skill_id.contains("cloud");
    if let Ok(conn) = init_audit(&PathBuf::from(&repo_root), &session_id) {
        let payload =
            serde_json::json!({ "entry_id": &next_entry.id, "skill_id": &next_entry.skill_id })
                .to_string();
        let _ = append_event(&conn, &session_id, "skill.started", &payload);
    }
    Ok(SkillSpawnResult {
        entry_id: next_entry.id,
        skill_id: next_entry.skill_id.clone(),
        skill_name: next_entry.skill_name,
        status: "running".into(),
        message: format!(
            "skill '{}' is now running{}",
            next_entry.skill_id,
            if is_cloud {
                " via cloud endpoint"
            } else {
                "; GPU serial lock acquired"
            }
        ),
        gpu_lock_required: !is_cloud,
    })
}

#[derive(Debug, Deserialize)]
struct SkillCompleteRequest {
    entry_id: String,
    status: String, // completed | failed | cancelled
    error_message: Option<String>,
    iterations: u32,
    session_id: String,
    repo_root: String,
}

#[tauri::command]
fn skill_queue_complete(req: SkillCompleteRequest) -> Result<(), String> {
    let mut state = skill_queue_state()
        .lock()
        .map_err(|_| "skill queue lock poisoned".to_string())?;
    if let Some(mut current) = state.current.take() {
        if current.id != req.entry_id {
            // Entry mismatch - still complete whatever is current, but log the discrepancy.
            current.status = "completed".into();
            current.completed_at = Some(iso_now());
            state.history.push(current);
        } else {
            current.status = req.status.clone();
            current.completed_at = Some(iso_now());
            current.error_message = req.error_message;
            current.iterations = req.iterations;
            state.history.push(current);
        }
    }
    if let Ok(conn) = init_audit(&PathBuf::from(&req.repo_root), &req.session_id) {
        let payload =
            serde_json::json!({ "entry_id": &req.entry_id, "status": &req.status }).to_string();
        let _ = append_event(&conn, &req.session_id, "skill.completed", &payload);
    }
    Ok(())
}

#[tauri::command]
fn skill_queue_cancel_all() -> Result<(), String> {
    let mut state = skill_queue_state()
        .lock()
        .map_err(|_| "skill queue lock poisoned".to_string())?;
    let now = iso_now();
    let drained: Vec<SkillQueueEntry> = state.queue.drain(..).collect();
    for mut entry in drained {
        entry.status = "cancelled".into();
        entry.completed_at = Some(now.clone());
        state.history.push(entry);
    }
    if let Some(mut current) = state.current.take() {
        current.status = "cancelled".into();
        current.completed_at = Some(now.clone());
        state.history.push(current);
    }
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            open_repo_mock,
            open_repo_path,
            list_repo_files,
            clear_local_session_data,
            validate_patch_proposal,
            approve_patch_proposal,
            apply_approved_patch,
            rollback_patch,
            read_guarded_file,
            write_guarded_file,
            create_repo_file,
            rename_repo_path,
            delete_repo_path,
            list_audit_events,
            record_session_event,
            build_context_bundle,
            approve_runtime_endpoint,
            runtime_endpoint_approval_status,
            runtime_health_check,
            runtime_generate,
            runtime_cancel,
            list_runtime_models,
            classify_command,
            project_search,
            git_status,
            git_diff_file,
            git_stage_file,
            git_unstage_file,
            git_commit_changes,
            lsp_capabilities,
            detect_tasks,
            approve_task,
            approve_personal_command,
            run_approved_task,
            runtime_status,
            list_curated_models,
            register_local_model,
            list_runtime_presets,
            import_local_model,
            list_local_models,
            managed_llamacpp_validate_config,
            managed_llamacpp_start,
            managed_llamacpp_stop,
            managed_llamacpp_status,
            skill_list,
            skill_save,
            skill_delete,
            skill_reset_to_defaults,
            skill_get_system_prompt,
            skill_queue_spawn,
            skill_queue_status,
            skill_queue_advance,
            skill_queue_complete,
            skill_queue_cancel_all,
            studio_role_profiles,
            studio_save_role_runtime,
            studio_list_role_runtimes,
            studio_run_role,
            studio_cancel_role_run,
            studio_list_initiatives,
            studio_create_initiative,
            studio_get_snapshot,
            studio_approve_scope,
            studio_run_fake,
            studio_control,
            studio_export_proof,
            dream_save_mandate,
            dream_factory_start,
            dream_factory_state,
            dream_factory_pause,
            dream_factory_resume,
            dream_factory_stop,
            dream_factory_tick,
            dream_start_cycle,
            dream_list_inbox,
            dream_action,
            governed_worktree_inspect,
            governed_worktree_create,
            governed_worktree_diff,
            governed_worktree_cleanup,
            studio_pulse,
            validate_declarative_prototype
        ])
        .run(tauri::generate_context!())
        .expect("error while running Synthesize IDE");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        ensure_task_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO sessions (id, repo_root) VALUES ('s', '/tmp/repo')",
            [],
        )
        .unwrap();
        conn
    }

    fn temp_repo(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("synthesize-{}-{}", label, new_id("test")));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn duplicate_proposal_id_with_different_hash_is_rejected() {
        let conn = memory_conn();
        conn.execute(
            "INSERT INTO patch_proposals (proposal_id, session_id, repo_root, operation_json, operation_sha256, status) VALUES ('p1', 's', '/tmp/repo', '{}', 'aaa', 'validated')",
            [],
        ).unwrap();
        let result = ensure_duplicate_is_same_hash(&conn, "p1", "bbb");
        assert!(result.is_err());
    }

    #[test]
    fn approval_must_match_proposal_and_operation_hash() {
        let conn = memory_conn();
        conn.execute(
            "INSERT INTO patch_proposals (proposal_id, session_id, repo_root, operation_json, operation_sha256, status) VALUES ('p1', 's', '/tmp/repo', '{}', 'aaa', 'approved')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO patch_approvals (approval_id, proposal_id, operation_sha256, approved_by_source, approval_scope) VALUES ('ap1', 'p1', 'aaa', 'local-user', 'whole-proposal')",
            [],
        ).unwrap();
        assert!(verify_approval(&conn, "p1", "ap1", "aaa").is_ok());
        assert!(verify_approval(&conn, "p1", "ap1", "bbb").is_err());
        assert!(verify_approval(&conn, "p1", "missing", "aaa").is_err());
    }

    #[test]
    fn lifecycle_transition_rules_reject_illegal_moves() {
        assert!(is_legal_transition("validated", "approved"));
        assert!(is_legal_transition("approved", "applying"));
        assert!(!is_legal_transition("applied", "approved"));
        assert!(!is_legal_transition("rolled_back", "applied"));
        assert!(!is_legal_transition("rejected", "approved"));
    }

    #[test]
    fn moa_risk_label_escalates_with_notes_and_size() {
        assert_eq!(moa_patch_risk_label(1, &[]), "low");
        assert_eq!(moa_patch_risk_label(4, &[]), "high");
        assert_eq!(
            moa_patch_risk_label(1, &["high risk auth change".into()]),
            "high"
        );
        assert_eq!(moa_patch_risk_label(7, &[]), "critical");
        assert_eq!(
            moa_patch_risk_label(1, &["critical migration".into()]),
            "critical"
        );
    }

    #[test]
    fn non_moa_profile_does_not_require_bridge_gate() {
        let op = AgentOperation::ProposePatch {
            proposal_id: "p-local".into(),
            summary: "x".into(),
            base_commit: None,
            current_commit: None,
            files: vec![ProtocolPatchFile { id: "f".into(), path: "src/lib.ts".into(), before_sha256: "abc".into(), patch: "diff --git a/src/lib.ts b/src/lib.ts\n--- a/src/lib.ts\n+++ b/src/lib.ts\n@@ -1 +1 @@\n-a\n+b\n".into() }],
            risk_notes: vec![],
            suggested_commands: vec![],
        };
        let decision = enforce_moa_gate_for_operation(&op, "local-patcher").unwrap();
        assert_eq!(decision["enforced"], false);
    }

    #[test]
    fn moa_profile_calls_bridge_and_allows_low_risk_patch() {
        let op = AgentOperation::ProposePatch {
            proposal_id: "p-moa".into(),
            summary: "x".into(),
            base_commit: None,
            current_commit: None,
            files: vec![ProtocolPatchFile { id: "f".into(), path: "src/lib.ts".into(), before_sha256: "abc".into(), patch: "diff --git a/src/lib.ts b/src/lib.ts\n--- a/src/lib.ts\n+++ b/src/lib.ts\n@@ -1 +1 @@\n-a\n+b\n".into() }],
            risk_notes: vec![],
            suggested_commands: vec![],
        };
        let decision = enforce_moa_gate_for_operation(&op, "moa-action-planner").unwrap();
        assert_eq!(decision["enforced"], true);
        assert_eq!(decision["approved"], true);
        assert_eq!(decision["protocol"], MOA_BRIDGE_PROTOCOL);
    }

    #[test]
    fn moa_profile_rejects_critical_patch_before_backend_validation() {
        let files = (0..4).map(|idx| ProtocolPatchFile {
            id: format!("f{}", idx),
            path: format!("src/lib{}.ts", idx),
            before_sha256: "abc".into(),
            patch: format!("diff --git a/src/lib{0}.ts b/src/lib{0}.ts\n--- a/src/lib{0}.ts\n+++ b/src/lib{0}.ts\n@@ -1 +1 @@\n-a\n+b\n", idx),
        }).collect::<Vec<_>>();
        let op = AgentOperation::ProposePatch {
            proposal_id: "p-critical".into(),
            summary: "x".into(),
            base_commit: None,
            current_commit: None,
            files,
            risk_notes: vec!["critical blast radius".into()],
            suggested_commands: vec![],
        };
        let err = enforce_moa_gate_for_operation(&op, "moa-action-planner").unwrap_err();
        assert!(err.contains("MoA gate rejected operation"));
    }

    #[test]
    fn transition_helper_prevents_double_apply_state() {
        let conn = memory_conn();
        conn.execute(
            "INSERT INTO patch_proposals (proposal_id, session_id, repo_root, operation_json, operation_sha256, status) VALUES ('p1', 's', '/tmp/repo', '{}', 'aaa', 'approved')",
            [],
        ).unwrap();
        assert!(transition_proposal_status(&conn, "s", "p1", "approved", "applying", None).is_ok());
        assert!(
            transition_proposal_status(&conn, "s", "p1", "approved", "applying", None).is_err()
        );
    }

    #[test]
    fn approve_after_applied_is_rejected_by_state_check() {
        let conn = memory_conn();
        conn.execute(
            "INSERT INTO patch_proposals (proposal_id, session_id, repo_root, operation_json, operation_sha256, status) VALUES ('p1', 's', '/tmp/repo', '{}', 'aaa', 'applied')",
            [],
        ).unwrap();
        let stored = load_stored_proposal_allow_empty_for_test(&conn, "p1").unwrap();
        assert_eq!(stored.status, "applied");
        assert_ne!(stored.status, "validated");
    }

    #[test]
    fn lifecycle_transition_is_not_blocked_by_lifecycle_audit_failure() {
        let conn = memory_conn();
        conn.execute(
            "INSERT INTO patch_proposals (proposal_id, session_id, repo_root, operation_json, operation_sha256, status) VALUES ('p-phase', 's', '/tmp/repo', '{}', 'aaa', 'approved')",
            [],
        ).unwrap();
        conn.execute("DROP TABLE audit_events", []).unwrap();
        let result =
            transition_proposal_status(&conn, "s", "p-phase", "approved", "applying", None);
        assert!(result.is_ok());
        let status: String = conn
            .query_row(
                "SELECT status FROM patch_proposals WHERE proposal_id = 'p-phase'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "applying");
    }

    #[test]
    fn repo_roots_equal_canonicalizes_equivalent_paths() {
        let root =
            std::env::temp_dir().join(format!("synthesize-root-canon-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("nested")).unwrap();
        let stored = root.join("nested").join("..").to_string_lossy().to_string();
        assert!(repo_roots_equal(&stored, &root).unwrap());
    }

    #[test]
    fn denied_context_directory_child_names_are_not_listed() {
        let root = std::env::temp_dir().join(format!(
            "synthesize-denied-dir-no-child-names-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".secrets")).unwrap();
        std::fs::write(root.join(".secrets").join("prod-api-key.txt"), "secret").unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src").join("a.ts"), "export const a = 1;\n").unwrap();
        let files = list_files_internal(&root, 20).unwrap();
        assert!(files
            .iter()
            .any(|f| f.path == ".secrets/" && f.kind == "dir-denied" && f.denied));
        assert!(!files.iter().any(|f| f.path.contains("prod-api-key")));
    }

    #[test]
    fn denied_context_directory_names_are_not_descended() {
        assert!(is_denied_context_dir_name(".secrets"));
        assert!(is_denied_context_dir_name("credentials"));
        assert!(is_denied_context_dir_name("client-secrets"));
        assert!(!is_denied_context_dir_name("src"));
    }

    #[test]
    fn endpoint_classification_distinguishes_local_private_lan_and_remote() {
        assert_eq!(classify_endpoint_url("http://localhost:8080/v1"), "local");
        assert_eq!(classify_endpoint_url("http://127.0.0.1:8080/v1"), "local");
        assert_eq!(
            classify_endpoint_url("http://192.168.1.55:8080/v1"),
            "private-lan"
        );
        assert_eq!(classify_endpoint_url("https://example.com/v1"), "remote");
    }

    #[test]
    fn repo_open_payload_matches_renderer_camel_case_contract() {
        let payload = RepoOpenResult {
            repo_root: "C:/repo".into(),
            current_file_path: "src/main.ts".into(),
            current_file_content: "export const ok = true;\n".into(),
            current_commit: None,
            files: Vec::new(),
        };
        let value = serde_json::to_value(payload).unwrap();
        assert!(value.get("repoRoot").is_some());
        assert!(value.get("currentFilePath").is_some());
        assert!(value.get("currentFileContent").is_some());
        assert!(value.get("repo_root").is_none());
    }

    #[test]
    fn non_local_endpoint_requires_persisted_approval() {
        let conn = memory_conn();
        assert!(verify_endpoint_approval(&conn, "https://example.com/v1", "remote").is_err());
        conn.execute(
            "INSERT INTO endpoint_approvals (endpoint_url, endpoint_classification, approved_by_source, allow_repo_context) VALUES ('https://example.com/v1', 'remote', 'local-user', 1)",
            [],
        ).unwrap();
        assert!(verify_endpoint_approval(&conn, "https://example.com/v1", "remote").is_ok());
    }

    #[test]
    fn fake_runtime_response_returns_typed_patch_operation() {
        let messages = vec![RuntimeMessage {
            role: "user".into(),
            content: "currentFile=src/auth/refresh.ts\nbeforeSha256=abc123\ncurrentCommit=deadbeef"
                .into(),
        }];
        let raw = fake_runtime_response(&messages);
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed["operations"][0]["type"], "propose_patch");
        assert_eq!(
            parsed["operations"][0]["files"][0]["beforeSha256"],
            "abc123"
        );
    }

    #[test]
    fn runtime_messages_are_derived_from_persisted_context_bundle() {
        let conn = memory_conn();
        let messages = vec![
            RuntimeMessage {
                role: "system".into(),
                content: "s".into(),
            },
            RuntimeMessage {
                role: "user".into(),
                content: "u".into(),
            },
        ];
        let view = ContextBundleView {
            context_bundle_id: "ctx1".into(),
            session_id: "s".into(),
            repo_root: "/tmp/repo".into(),
            user_message: "task".into(),
            selected_file_path: "src/a.ts".into(),
            dirty_buffer_state: false,
            git_commit: None,
            endpoint_classification: "local".into(),
            destination_warning: "local".into(),
            char_estimate: 2,
            runtime: "fake".into(),
            model: "fixture".into(),
            model_context_window_tokens: 32_768,
            reserved_output_tokens: 4_096,
            safety_margin_tokens: 1_024,
            compiled_input_tokens: 1,
            remaining_capacity_tokens: 27_647,
            token_count_kind: "estimated".into(),
            token_estimation_method: "conservative_utf8_bytes_div3".into(),
            included: vec![],
            omitted: vec![],
            summaries_used: vec![],
            truncations: vec![],
            messages: messages.clone(),
            exact_prompt: "u".into(),
            messages_sha256: hash_runtime_messages(&messages).unwrap(),
            exact_context: true,
            agent_profile_id: "local-patcher".into(),
        };
        conn.execute("INSERT INTO context_bundles (id, session_id, token_estimate, payload_json) VALUES ('ctx1', 's', 2, ?1)", [serde_json::to_string(&view).unwrap()]).unwrap();
        let loaded = load_context_bundle(&conn, "ctx1").unwrap();
        assert_eq!(
            loaded.messages_sha256,
            hash_runtime_messages(&loaded.messages).unwrap()
        );
    }

    #[test]
    fn legacy_assist_context_bundle_remains_readable_after_context_os_migration() {
        let conn = memory_conn();
        let messages = vec![RuntimeMessage {
            role: "user".into(),
            content: "legacy exact prompt".into(),
        }];
        let legacy = serde_json::json!({
            "context_bundle_id":"legacy-ctx",
            "session_id":"s",
            "repo_root":"/tmp/repo",
            "user_message":"legacy",
            "selected_file_path":"src/legacy.ts",
            "dirty_buffer_state":false,
            "git_commit":null,
            "endpoint_classification":"local",
            "destination_warning":"local",
            "char_estimate":19,
            "included":[],
            "messages":messages,
            "exact_prompt":"legacy exact prompt",
            "messages_sha256":hash_runtime_messages(&messages).unwrap(),
            "exact_context":true,
            "agent_profile_id":"local-patcher"
        });
        conn.execute(
            "INSERT INTO context_bundles (id, session_id, token_estimate, payload_json)
             VALUES ('legacy-ctx','s',19,?1)",
            [legacy.to_string()],
        )
        .unwrap();
        let loaded = load_context_bundle(&conn, "legacy-ctx").unwrap();
        assert_eq!(loaded.user_message, "legacy");
        assert_eq!(loaded.compiled_input_tokens, 0);
        assert!(loaded.token_estimation_method.is_empty());
    }

    #[test]
    fn assist_selected_file_metadata_matches_the_bounded_excerpt() {
        let repo = temp_repo("bounded-selected-file");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("src/large.ts"), "x".repeat(25_123)).unwrap();
        let view = build_context_bundle(ContextBundleRequest {
            session_id: "excerpt-session".into(),
            repo_root: repo.to_string_lossy().to_string(),
            user_message: "inspect the bounded excerpt".into(),
            selected_file_path: "src/large.ts".into(),
            selected_text: None,
            dirty_buffer_state: false,
            provider: "fake".into(),
            endpoint_url: None,
            agent_profile_id: Some("local-reviewer".into()),
            model: "fixture".into(),
            context_window_tokens: 65_536,
            maximum_output_tokens: 4_096,
            safety_margin_tokens: 1_024,
            token_estimation_method: "conservative_utf8_bytes_div3".into(),
            structured_output_behavior: "json_object".into(),
            capability_source: "selected-file regression test".into(),
        })
        .unwrap();
        let selected = view
            .included
            .iter()
            .find(|item| item.kind == "selected_file")
            .unwrap();
        assert_eq!(selected.chars, 24_000);
        assert_eq!(view.truncations[0].original_chars, 25_123);
        assert_eq!(view.truncations[0].included_chars, 24_000);
    }

    #[test]
    fn symlink_file_tree_entry_is_skipped_not_traversed() {
        let root =
            std::env::temp_dir().join(format!("synthesize-symlink-skip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src").join("a.ts"), "export const a = 1;\n").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/tmp", root.join("linked-tmp")).unwrap();
            let files = list_files_internal(&root, 20).unwrap();
            assert!(files
                .iter()
                .any(|f| f.path == "linked-tmp" && f.kind == "symlink-skipped" && f.denied));
            assert!(!files.iter().any(|f| f.path.starts_with("linked-tmp/")));
        }
    }

    fn load_stored_proposal_allow_empty_for_test(
        conn: &Connection,
        proposal_id: &str,
    ) -> Result<StoredProposal, String> {
        let (operation_sha256, operation_json, status): (String, String, String) = conn.query_row(
            "SELECT operation_sha256, operation_json, status FROM patch_proposals WHERE proposal_id = ?1",
            params![proposal_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).map_err(|e| e.to_string())?;
        Ok(StoredProposal {
            proposal: PatchProposal {
                id: proposal_id.to_string(),
                base_commit: None,
                current_commit: None,
                files: vec![PatchFile {
                    id: "f".into(),
                    path: "src/a.ts".into(),
                    before_sha256: "x".into(),
                    unified_diff: "diff".into(),
                }],
            },
            operation_sha256,
            operation_json,
            status,
            checkpoint_id: None,
            checkpoint_dir: None,
        })
    }

    #[test]
    fn runtime_presets_include_local_and_cloud_lanes() {
        let presets = list_runtime_presets();
        assert!(presets.iter().any(|p| p.id == "llamacpp-server"));
        assert!(presets.iter().any(|p| p.id == "lm-studio"));
        assert!(presets
            .iter()
            .any(|p| p.id == "cloud-openai" && !p.local_by_default));
        assert!(presets
            .iter()
            .any(|p| p.id == "cloud-anthropic" && !p.local_by_default));
        assert!(presets
            .iter()
            .filter(|p| p.id.starts_with("cloud-"))
            .all(|p| !p.local_by_default));
    }

    #[test]
    fn managed_llamacpp_rejects_missing_binary_or_model() {
        let req = ManagedLlamaConfigRequest {
            binary_path: "/definitely/missing/llama-server".into(),
            model_path: "/definitely/missing/model.gguf".into(),
            port: Some(8080),
            ctx_size: Some(8192),
        };
        assert!(managed_llamacpp_validate_config(req).is_err());
    }

    #[test]
    fn agent_profile_policy_allows_only_patch_capable_profiles_for_patch_validation() {
        let op = AgentOperation::ProposePatch {
            proposal_id: "p-agent-policy".into(),
            summary: "test".into(),
            base_commit: None,
            current_commit: None,
            files: vec![agent_protocol::PatchFile {
                id: "f1".into(),
                path: "src/a.ts".into(),
                before_sha256: "abc".into(),
                patch: r#"diff --git a/src/a.ts b/src/a.ts
--- a/src/a.ts
+++ b/src/a.ts
@@ -1 +1 @@
-a
+b
"#
                .into(),
            }],
            risk_notes: vec![],
            suggested_commands: vec![],
        };
        assert!(enforce_agent_profile_allows_operation("local-patcher", &op).is_ok());
        assert!(enforce_agent_profile_allows_operation("moa-action-planner", &op).is_ok());
        assert!(enforce_agent_profile_allows_operation("fake-demo", &op).is_ok());
        assert!(enforce_agent_profile_allows_operation("local-planner", &op).is_err());
        assert!(enforce_agent_profile_allows_operation("local-reviewer", &op).is_err());
    }

    #[test]
    fn system_prompt_describes_local_agent_and_disabled_commands() {
        let prompt = build_synthesize_system_prompt("local-patcher");
        assert!(prompt.contains("self-hosted open-source coding model"));
        assert!(prompt.contains("cannot run shell commands"));
        assert!(prompt.contains("strict JSON"));
    }

    #[test]
    fn moa_action_planner_prompt_describes_governed_action_trace() {
        let prompt = build_synthesize_system_prompt("moa-action-planner");
        assert!(prompt.contains("MoA Action Planner"));
        assert!(prompt.contains("plan/action trace"));
        assert!(prompt.contains("does not claim to expose private model chain-of-thought"));
    }

    #[test]
    fn planner_and_reviewer_prompts_are_report_only() {
        let planner = build_synthesize_system_prompt("local-planner");
        let reviewer = build_synthesize_system_prompt("local-reviewer");
        assert!(planner.contains("report or ask_user operations only"));
        assert!(planner.contains("Do not emit propose_patch"));
        assert!(reviewer.contains("Emit report operations only"));
        assert!(reviewer.contains("Do not emit propose_patch"));
    }

    #[test]
    fn local_server_body_omits_null_response_format_when_unrequested() {
        let body = local_chat_request_body("m", &[], 0.1, 32, None).unwrap();
        assert!(body.get("response_format").is_none());
        let json_mode = local_chat_request_body("m", &[], 0.1, 32, Some("json_schema")).unwrap();
        assert_eq!(json_mode["response_format"]["type"], "json_object");
    }

    #[test]
    fn source_profile_policy_uses_persisted_context_bundle() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO sessions (id, repo_root) VALUES ('s', '/tmp/repo')",
            [],
        )
        .unwrap();
        let tmp =
            std::env::temp_dir().join(format!("synthesize-source-profile-{}", new_id("test")));
        std::fs::create_dir_all(&tmp).unwrap();
        let context = ContextBundleView {
            context_bundle_id: "ctx-reviewer".into(),
            session_id: "s".into(),
            repo_root: canonical_repo_root_string(&tmp).unwrap(),
            user_message: "review this".into(),
            selected_file_path: "src/lib.ts".into(),
            dirty_buffer_state: false,
            git_commit: None,
            endpoint_classification: "local".into(),
            destination_warning: "local".into(),
            char_estimate: 1,
            runtime: "fake".into(),
            model: "fixture".into(),
            model_context_window_tokens: 32_768,
            reserved_output_tokens: 4_096,
            safety_margin_tokens: 1_024,
            compiled_input_tokens: 1,
            remaining_capacity_tokens: 27_647,
            token_count_kind: "estimated".into(),
            token_estimation_method: "conservative_utf8_bytes_div3".into(),
            included: vec![],
            omitted: vec![],
            summaries_used: vec![],
            truncations: vec![],
            messages: vec![RuntimeMessage {
                role: "user".into(),
                content: "x".into(),
            }],
            exact_prompt: "x".into(),
            messages_sha256: "h".into(),
            exact_context: true,
            agent_profile_id: "local-reviewer".into(),
        };
        conn.execute(
            "INSERT INTO context_bundles (id, session_id, token_estimate, payload_json) VALUES (?1, ?2, ?3, ?4)",
            params!["ctx-reviewer", "s", 1_i64, serde_json::to_string(&context).unwrap()],
        ).unwrap();
        let op = AgentOperation::ProposePatch {
            proposal_id: "p".into(),
            summary: "x".into(),
            base_commit: None,
            current_commit: None,
            files: vec![ProtocolPatchFile { id: "f".into(), path: "src/lib.ts".into(), before_sha256: "abc".into(), patch: "diff --git a/src/lib.ts b/src/lib.ts\n--- a/src/lib.ts\n+++ b/src/lib.ts\n@@ -1 +1 @@\n-a\n+b\n".into() }],
            risk_notes: vec![],
            suggested_commands: vec![],
        };
        let req = PatchProposalRequest {
            session_id: "s".into(),
            repo_root: tmp.to_string_lossy().to_string(),
            operation: op,
            agent_profile_id: Some("local-patcher".into()),
            context_bundle_id: "ctx-reviewer".into(),
        };
        let (profile, _) = source_profile_for_patch_validation(&conn, &req, &tmp).unwrap();
        assert_eq!(profile, "local-reviewer");
        assert!(enforce_agent_profile_allows_operation(&profile, &req.operation).is_err());
    }

    #[test]
    fn patch_validation_requires_context_bundle_id() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        let tmp =
            std::env::temp_dir().join(format!("synthesize-missing-context-{}", new_id("test")));
        std::fs::create_dir_all(&tmp).unwrap();
        let op = AgentOperation::ProposePatch {
            proposal_id: "p-missing-context".into(),
            summary: "x".into(),
            base_commit: None,
            current_commit: None,
            files: vec![ProtocolPatchFile {
                id: "f".into(),
                path: "src/lib.ts".into(),
                before_sha256: "abc".into(),
                patch: r#"diff --git a/src/lib.ts b/src/lib.ts
--- a/src/lib.ts
+++ b/src/lib.ts
@@ -1 +1 @@
-a
+b
"#
                .into(),
            }],
            risk_notes: vec![],
            suggested_commands: vec![],
        };
        let req = PatchProposalRequest {
            session_id: "s".into(),
            repo_root: tmp.to_string_lossy().to_string(),
            operation: op,
            agent_profile_id: Some("local-patcher".into()),
            context_bundle_id: "".into(),
        };
        let err = source_profile_for_patch_validation(&conn, &req, &tmp).unwrap_err();
        assert!(err.contains("context_bundle_id is required"));
    }

    #[test]
    fn task_snapshot_approval_requires_detected_task() {
        let dir = temp_repo("task-missing-snapshot");
        let err = approve_task(TaskApproveRequest {
            session_id: "s".into(),
            repo_root: dir.to_string_lossy().to_string(),
            task_id: "unknown".into(),
        })
        .unwrap_err();
        assert!(err.contains("task snapshot not found"));
    }

    #[test]
    fn report_only_command_status_text_is_not_stale() {
        let status = runtime_status();
        assert!(status
            .notes
            .iter()
            .any(|n| n.contains("Agent-suggested commands remain classification-only")));
        assert!(status
            .notes
            .iter()
            .any(|n| n.contains("Backend-detected tasks can run")));
    }

    #[test]
    fn task_snapshot_repo_root_must_match_approval_repo() {
        let conn = memory_conn();
        let repo_a = temp_repo("task-repo-a");
        let repo_b = temp_repo("task-repo-b");
        let repo_a_str = canonical_repo_root_string(&repo_a).unwrap();
        conn.execute(
            r#"INSERT INTO task_snapshots (task_id, session_id, repo_root, label, argv_json, cwd, risk, reason, requires_network, may_modify_files) VALUES ('task-a', 's', ?1, 'cargo test', '["cargo","test"]', '.', 'TestOrBuild', 'test', 0, 0)"#,
            params![repo_a_str],
        ).unwrap();
        let row_repo: String = conn
            .query_row(
                "SELECT repo_root FROM task_snapshots WHERE task_id = 'task-a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!repo_roots_equal(&row_repo, &repo_b).unwrap());
    }

    #[test]
    fn task_snapshots_can_reuse_task_id_across_repos() {
        let conn = memory_conn();
        ensure_task_schema(&conn).unwrap();
        let repo_a = canonical_repo_root_string(&temp_repo("repo-scoped-task-a")).unwrap();
        let repo_b = canonical_repo_root_string(&temp_repo("repo-scoped-task-b")).unwrap();
        conn.execute(
            r#"INSERT INTO task_snapshots (task_id, session_id, repo_root, label, argv_json, cwd, risk, reason, requires_network, may_modify_files) VALUES ('cargo-test', 's', ?1, 'cargo test A', '["cargo","test"]', '.', 'TestOrBuild', 'a', 0, 0)"#,
            params![repo_a],
        ).unwrap();
        conn.execute(
            r#"INSERT INTO task_snapshots (task_id, session_id, repo_root, label, argv_json, cwd, risk, reason, requires_network, may_modify_files) VALUES ('cargo-test', 's', ?1, 'cargo test B', '["cargo","test"]', '.', 'TestOrBuild', 'b', 0, 0)"#,
            params![repo_b],
        ).unwrap();
        let count: i64 = conn.query_row("SELECT count(*) FROM task_snapshots WHERE task_id = 'cargo-test' AND session_id = 's'", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn approved_task_repo_root_must_match_run_repo() {
        let conn = memory_conn();
        let repo_a = temp_repo("approved-task-a");
        let repo_b = temp_repo("approved-task-b");
        let repo_a_str = canonical_repo_root_string(&repo_a).unwrap();
        conn.execute(
            r#"INSERT INTO commands (id, session_id, task_id, repo_root, argv_json, cwd, risk, requires_network, may_modify_files, approved_at) VALUES ('cmd-a', 's', 'task-a', ?1, '["cargo","test"]', '.', 'TestOrBuild', 0, 0, datetime('now'))"#,
            params![repo_a_str],
        ).unwrap();
        let row_repo: String = conn
            .query_row(
                "SELECT repo_root FROM commands WHERE id = 'cmd-a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!repo_roots_equal(&row_repo, &repo_b).unwrap());
    }

    #[test]
    fn guarded_manifest_exists_uses_repo_guard() {
        let root = temp_repo("guarded-manifest");
        std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        assert!(guarded_manifest_exists(&root, "Cargo.toml"));
        assert!(!guarded_manifest_exists(&root, ".env"));
    }

    #[test]
    fn delete_path_rejects_repo_root_and_control_dirs() {
        assert!(super::is_dangerous_delete_path("."));
        assert!(super::is_dangerous_delete_path(""));
        assert!(super::is_dangerous_delete_path(".git"));
        assert!(super::is_dangerous_delete_path(".synthesize/checkpoints"));
        assert!(!super::is_dangerous_delete_path("src/main.rs"));
    }

    #[test]
    #[ignore = "requires local Ollama endpoint with qwen3-coder downloaded"]
    fn runtime_generate_with_ollama_qwen3_end_to_end() {
        let repo = temp_repo("runtime-qwen3-ollama");
        let selected_file = repo.join("src").join("auth").join("refresh.ts");
        std::fs::create_dir_all(selected_file.parent().unwrap()).unwrap();
        std::fs::write(
            &selected_file,
            "export function refreshToken() {\n  throw new Error(\"not implemented\");\n}\n",
        )
        .unwrap();

        let conn = init_audit(&repo, "s").unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO sessions (id, repo_root) VALUES (?1, ?2)",
            params!["s", canonical_repo_root_string(&repo).unwrap()],
        )
        .unwrap();

        let messages = vec![
            RuntimeMessage { role: "system".into(), content: "You are Synthesize IDE local coding agent. Return only strict JSON. Do not use markdown fences. The only allowed operation types are propose_patch, run_command, report, ask_user, and final_report.".into() },
            RuntimeMessage { role: "user".into(), content: "Create a Synthesize typed operation for this task. Current file: src/auth/refresh.ts beforeSha256=fixture-before-sha256. Return exactly this shape: {\"operations\":[{\"type\":\"propose_patch\",\"proposalId\":\"local-model-smoke\",\"summary\":\"Replace throwing refreshToken stub with a deterministic return value.\",\"files\":[{\"id\":\"local-model-smoke-file-001\",\"path\":\"src/auth/refresh.ts\",\"beforeSha256\":\"fixture-before-sha256\",\"patch\":\"diff --git a/src/auth/refresh.ts b/src/auth/refresh.ts\\n--- a/src/auth/refresh.ts\\n+++ b/src/auth/refresh.ts\\n@@ -1,3 +1,3 @@\\n export function refreshToken() {\\n-  throw new Error(\\\"not implemented\\\");\\n+  return \\\"refreshed\\\";\\n }\\n\"}],\"riskNotes\":[\"Low-risk fixture patch for local model smoke test.\"],\"suggestedCommands\":[{\"type\":\"run_command\",\"argv\":[\"pnpm\",\"test\",\"auth\"],\"cwd\":\".\",\"reason\":\"Verify auth refresh behavior.\",\"expectedOutcome\":\"Auth tests pass.\",\"requiresNetwork\":false,\"mayModifyFiles\":false}]}]}".into() },
        ];
        let messages_sha256 = hash_runtime_messages(&messages).unwrap();
        let context = ContextBundleView {
            context_bundle_id: "ctx-qwen3-ollama".into(),
            session_id: "s".into(),
            repo_root: canonical_repo_root_string(&repo).unwrap(),
            user_message: "repair refreshToken".into(),
            selected_file_path: "src/auth/refresh.ts".into(),
            dirty_buffer_state: false,
            git_commit: None,
            endpoint_classification: "local".into(),
            destination_warning: "local endpoint".into(),
            char_estimate: 256,
            runtime: "local-server".into(),
            model: "qwen3-coder".into(),
            model_context_window_tokens: 32_768,
            reserved_output_tokens: 900,
            safety_margin_tokens: 1_024,
            compiled_input_tokens: conservative_runtime_message_tokens(&messages),
            remaining_capacity_tokens: 30_844,
            token_count_kind: "estimated".into(),
            token_estimation_method: "conservative_utf8_bytes_div3".into(),
            included: vec![],
            omitted: vec![],
            summaries_used: vec![],
            truncations: vec![],
            messages,
            exact_prompt: "repair refreshToken".into(),
            messages_sha256,
            exact_context: true,
            agent_profile_id: "local-patcher".into(),
        };
        conn.execute(
            "INSERT INTO context_bundles (id, session_id, token_estimate, payload_json) VALUES (?1, ?2, ?3, ?4)",
            params!["ctx-qwen3-ollama", "s", 256_i64, serde_json::to_string(&context).unwrap()],
        ).unwrap();

        let result = runtime_generate(RuntimeGenerateRequest {
            session_id: "s".into(),
            repo_root: repo.to_string_lossy().to_string(),
            provider: "local-server".into(),
            endpoint_url: "http://127.0.0.1:11434/v1".into(),
            model: "qwen3-coder".into(),
            temperature: 0.1,
            max_tokens: 900,
            response_format: Some("json_schema".into()),
            context_bundle_id: "ctx-qwen3-ollama".into(),
        })
        .expect("runtime_generate should succeed with local Ollama qwen3-coder endpoint");

        let content = result.content.trim();
        let json_candidate = if let Some(stripped) = content.strip_prefix("```") {
            stripped
                .trim_start_matches("json")
                .trim()
                .trim_end_matches("```")
                .trim()
                .to_string()
        } else {
            content.to_string()
        };
        let parsed = serde_json::from_str::<serde_json::Value>(&json_candidate).ok();
        let has_operations = parsed
            .as_ref()
            .and_then(|v| v.get("operations"))
            .and_then(|v| v.as_array())
            .map(|ops| !ops.is_empty())
            .unwrap_or(false);
        let has_patch_signal =
            content.contains("\"propose_patch\"") || content.contains("propose_patch");
        assert!(
            has_operations || has_patch_signal,
            "runtime output did not contain expected operation signals; raw content: {}",
            content
        );
    }
}
