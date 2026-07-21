use super::{
    call_cloud_anthropic, call_cloud_openai, call_openai_compatible_endpoint,
    classify_endpoint_url, init_audit, validate_patch_proposal_with_connection,
    verify_endpoint_approval, PatchProposalRequest, RuntimeMessage,
};
use agent_protocol::{AgentOperation, StudioOperationHeader};
use app_foundry::ApplicationWorkspaceManager;
use audit_log::new_id;
use context_os::{
    projection_policy, record_context_request, upsert_runtime_capability,
    validate_capsule_freshness, CapsuleCompileRequest, ContextCompiler, ContextRequest,
    RetrievalKind, RetrievalSelector, RuntimeCapability,
};
use dream_factory::DreamFactoryController;
use intent_ledger::{
    ArtifactEnvelope, DreamStatus, InitiativeMode, InitiativeStatus, Ledger, Mandate,
    OrchestrationEvent, Role, StudioTask, TaskStatus,
};
use orchestration_core::{
    approve_studio_scope, bootstrap_studio, complete_role_run, fake_role_artifact,
    prepare_role_run, role_profiles, run_fake_delivery, validate_prototype, FakeScenario,
    PrototypeDocument, RoleScheduler,
};
use patch_engine::{
    apply_patch_proposal_transactional, PatchFile as EnginePatchFile,
    PatchProposal as EnginePatchProposal,
};
use pulse_engine::{
    route_intervention, BeliefSnapshot, PulseEvent, RuleBasedTemporalObserver, SymbolicMonitor,
    TemporalObserver,
};
use rusqlite::{params, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{mpsc, OnceLock};
use std::time::{Duration, Instant};
use worktree_manager::WorktreeManager;

static STUDIO_ROLE_SCHEDULER: OnceLock<RoleScheduler> = OnceLock::new();

fn studio_role_scheduler() -> &'static RoleScheduler {
    STUDIO_ROLE_SCHEDULER.get_or_init(RoleScheduler::default)
}

#[derive(Debug, Deserialize)]
pub(crate) struct StudioRepoRequest {
    session_id: String,
    repo_root: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StudioCreateRequest {
    session_id: String,
    repo_root: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StudioInitiativeRequest {
    session_id: String,
    repo_root: String,
    initiative_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StudioControlRequest {
    session_id: String,
    repo_root: String,
    initiative_id: String,
    action: String,
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StudioFakeRunRequest {
    session_id: String,
    repo_root: String,
    initiative_id: String,
    scenario: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MandateRequest {
    session_id: String,
    repo_root: String,
    mandate: Mandate,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DreamCycleRequest {
    session_id: String,
    repo_root: String,
    mandate_id: String,
    focus: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DreamFactoryRequest {
    session_id: String,
    repo_root: String,
    mandate_id: String,
    output_root_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DreamOutputRootRequest {
    session_id: String,
    repo_root: String,
    output_root: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DreamFactoryControlRequest {
    session_id: String,
    repo_root: String,
    after_current: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DreamFactoryTickRequest {
    session_id: String,
    repo_root: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DreamActionRequest {
    session_id: String,
    repo_root: String,
    dream_id: String,
    action: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WorktreeCreateRequest {
    session_id: String,
    repo_root: String,
    initiative_id: String,
    approved_base_commit: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WorktreeRequest {
    session_id: String,
    repo_root: String,
    worktree_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WorktreeCleanupRequest {
    session_id: String,
    repo_root: String,
    worktree_id: String,
    confirmation_token: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PrototypeValidationRequest {
    document: Value,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RoleRuntimeConfigRequest {
    session_id: String,
    repo_root: String,
    role: String,
    runtime: String,
    model: String,
    endpoint_url: Option<String>,
    timeout_seconds: i64,
    context_window_tokens: usize,
    maximum_output_tokens: usize,
    token_estimation_method: String,
    safety_margin_tokens: usize,
    structured_output_behavior: String,
    capability_source: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StudioRoleRunRequest {
    session_id: String,
    repo_root: String,
    initiative_id: String,
    task_id: Option<String>,
    role: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StudioRoleCancelRequest {
    run_id: String,
}

#[tauri::command]
pub(crate) fn studio_role_profiles() -> Result<Value, String> {
    serde_json::to_value(role_profiles()).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn studio_save_role_runtime(req: RoleRuntimeConfigRequest) -> Result<Value, String> {
    let role = Role::try_from(req.role.as_str()).map_err(|error| error.to_string())?;
    if matches!(role, Role::Human | Role::System) {
        return Err("human/system are not model runtime roles".into());
    }
    if req.runtime.trim().is_empty() || req.model.trim().is_empty() {
        return Err("runtime and model are required".into());
    }
    if !(1..=1800).contains(&req.timeout_seconds) {
        return Err("role timeout must be within 1..=1800 seconds".into());
    }
    let canonical_repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let mut conn = init_audit(&PathBuf::from(canonical_repo), &req.session_id)
        .map_err(|error| error.to_string())?;
    let tx = conn.transaction().map_err(|error| error.to_string())?;
    let validated_at: String = tx
        .query_row("SELECT datetime('now')", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    upsert_runtime_capability(
        &tx,
        &RuntimeCapability {
            id: new_id("CAPABILITY"),
            session_id: req.session_id.clone(),
            runtime: req.runtime.clone(),
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
    tx.execute(
        "INSERT INTO role_runtime_configs (session_id, role, runtime, model, endpoint_url, timeout_seconds)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(session_id, role) DO UPDATE SET runtime=excluded.runtime, model=excluded.model,
             endpoint_url=excluded.endpoint_url, timeout_seconds=excluded.timeout_seconds, updated_at=datetime('now')",
        params![req.session_id, role.as_str(), req.runtime, req.model, req.endpoint_url, req.timeout_seconds],
    ).map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(
        json!({"role":role.as_str(),"saved":true,"capabilityValidated":true,"endpointApprovalStillRequired":true}),
    )
}

#[tauri::command]
pub(crate) fn studio_list_role_runtimes(req: StudioRepoRequest) -> Result<Value, String> {
    let canonical_repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let conn = init_audit(&PathBuf::from(canonical_repo), &req.session_id)
        .map_err(|error| error.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT json_object('role', c.role, 'runtime', c.runtime, 'model', c.model,
                            'endpointUrl', c.endpoint_url, 'timeoutSeconds', c.timeout_seconds,
                            'contextWindowTokens', p.context_window_tokens,
                            'maximumOutputTokens', p.maximum_output_tokens,
                            'tokenEstimationMethod', p.token_estimation_method,
                            'safetyMarginTokens', p.safety_margin_tokens,
                            'structuredOutputBehavior', p.structured_output_behavior,
                            'capabilitySource', p.capability_source,
                            'lastValidatedAt', p.last_validated_at, 'updatedAt', c.updated_at)
         FROM role_runtime_configs c LEFT JOIN runtime_capabilities p
           ON p.session_id=c.session_id AND p.runtime=c.runtime AND p.model=c.model
         WHERE c.session_id=?1 ORDER BY c.role",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map([req.session_id], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?;
    let values: std::result::Result<Vec<Value>, String> = rows
        .map(|row| {
            row.map_err(|error| error.to_string())
                .and_then(|value| serde_json::from_str(&value).map_err(|error| error.to_string()))
        })
        .collect();
    serde_json::to_value(values?).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn studio_run_role(req: StudioRoleRunRequest) -> Result<Value, String> {
    let binding = StudioInitiativeRequest {
        session_id: req.session_id,
        repo_root: req.repo_root,
        initiative_id: req.initiative_id,
    };
    let conn = bound_connection(&binding)?;
    let role = Role::try_from(req.role.as_str()).map_err(|error| error.to_string())?;
    if matches!(role, Role::Human | Role::System) {
        return Err("human/system are not model runtime roles".into());
    }
    if matches!(role, Role::Builder | Role::Verifier | Role::Reviewer) && req.task_id.is_none() {
        return Err(format!(
            "{} requires an explicit task binding",
            role.as_str()
        ));
    }
    let config: (String, String, Option<String>, i64) = conn
        .query_row(
            "SELECT runtime, model, endpoint_url, timeout_seconds FROM role_runtime_configs
             WHERE session_id=?1 AND role=?2",
            params![binding.session_id, role.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("configure the {} runtime before running it", role.as_str()))?;
    let prepared = prepare_role_run(
        &conn,
        &binding.initiative_id,
        req.task_id.as_deref(),
        role,
        &config.0,
        &config.1,
    )
    .map_err(|error| error.to_string())?;
    let scheduler = studio_role_scheduler();
    let lease = match scheduler.acquire(&prepared.run_id) {
        Ok(lease) => lease,
        Err(error) => {
            complete_role_run(
                &conn,
                &prepared,
                "failed",
                "scheduler_busy",
                Some(&error.to_string()),
            )
            .map_err(|completion| completion.to_string())?;
            return Err(error.to_string());
        }
    };
    let result = if config.0 == "fake" {
        run_fake_single_role(&conn, &prepared)
    } else {
        run_configured_role(
            &conn,
            &prepared,
            &config.0,
            &config.1,
            config.2.as_deref(),
            config.3,
        )
    };
    drop(lease);
    match result {
        Ok(operation) => {
            complete_role_run(&conn, &prepared, "completed", "valid_typed_operation", None)
                .map_err(|error| error.to_string())?;
            Ok(json!({
                "runId": prepared.run_id,
                "operation": operation,
                "snapshot": Ledger::new(&conn)
                    .workspace_snapshot(&binding.initiative_id)
                    .map_err(|error| error.to_string())?
            }))
        }
        Err(error) => {
            let status = if error.contains("cancelled") {
                "cancelled"
            } else if error.contains("timed out") {
                "timed_out"
            } else {
                "failed"
            };
            let bounded_error: String = error.chars().take(800).collect();
            complete_role_run(&conn, &prepared, status, "rejected", Some(&bounded_error))
                .map_err(|completion| completion.to_string())?;
            Err(error)
        }
    }
}

#[tauri::command]
pub(crate) fn studio_cancel_role_run(req: StudioRoleCancelRequest) -> Result<Value, String> {
    if req.run_id.trim().is_empty() {
        return Err("run_id is required".into());
    }
    studio_role_scheduler().cancel(&req.run_id);
    Ok(json!({"runId":req.run_id,"cancellationRequested":true}))
}

#[tauri::command]
pub(crate) fn studio_list_initiatives(req: StudioRepoRequest) -> Result<Value, String> {
    with_ledger(&req.session_id, &req.repo_root, |ledger| {
        serde_json::to_value(ledger.list_initiatives(&req.session_id, &canonical(&req.repo_root)?)?)
            .map_err(Into::into)
    })
}

#[tauri::command]
pub(crate) fn studio_create_initiative(req: StudioCreateRequest) -> Result<Value, String> {
    let repo_root = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let conn = init_audit(&PathBuf::from(&repo_root), &req.session_id)
        .map_err(|error| error.to_string())?;
    bootstrap_studio(&conn, &req.session_id, &repo_root, &req.prompt)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn studio_get_snapshot(req: StudioInitiativeRequest) -> Result<Value, String> {
    with_bound_ledger(&req, |ledger| ledger.workspace_snapshot(&req.initiative_id))
}

#[tauri::command]
pub(crate) fn studio_approve_scope(req: StudioInitiativeRequest) -> Result<Value, String> {
    let conn = bound_connection(&req)?;
    approve_studio_scope(&conn, &req.initiative_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn studio_run_fake(req: StudioFakeRunRequest) -> Result<Value, String> {
    let binding = StudioInitiativeRequest {
        session_id: req.session_id,
        repo_root: req.repo_root,
        initiative_id: req.initiative_id,
    };
    let conn = bound_connection(&binding)?;
    let scenario = parse_fake_scenario(&req.scenario)?;
    run_fake_delivery(&conn, &binding.initiative_id, scenario).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn studio_control(req: StudioControlRequest) -> Result<Value, String> {
    let binding = StudioInitiativeRequest {
        session_id: req.session_id,
        repo_root: req.repo_root,
        initiative_id: req.initiative_id,
    };
    let conn = bound_connection(&binding)?;
    let ledger = Ledger::new(&conn);
    let reason = req.reason.as_deref().unwrap_or("local user control");
    match req.action.as_str() {
        "pause" => {
            ledger
                .transition_initiative(
                    &binding.initiative_id,
                    InitiativeStatus::Paused,
                    Role::Human,
                    reason,
                )
                .map_err(|error| error.to_string())?;
        }
        "resume" => {
            ledger
                .resume_initiative(&binding.initiative_id, reason)
                .map_err(|error| error.to_string())?;
        }
        "stop" | "abandon" => {
            ledger
                .transition_initiative(
                    &binding.initiative_id,
                    InitiativeStatus::Abandoned,
                    Role::Human,
                    reason,
                )
                .map_err(|error| error.to_string())?;
        }
        "lower_autonomy" => {
            conn.execute(
                "UPDATE initiatives SET autonomy_level=MAX(0, autonomy_level-1), updated_at=datetime('now') WHERE id=?1",
                [&binding.initiative_id],
            )
            .map_err(|error| error.to_string())?;
        }
        "request_alignment_review" => {
            ledger
                .record_event(OrchestrationEvent {
                    id: new_id("EVENT"),
                    initiative_id: binding.initiative_id.clone(),
                    task_id: None,
                    actor_role: Role::Human,
                    kind: "alignment.review_requested".into(),
                    requirement_ids: vec![],
                    adr_ids: vec![],
                    assumption_ids: vec![],
                    features: BTreeMap::from([("coordination_deterioration".into(), 0.5)]),
                    provenance: "local-user".into(),
                    redacted_summary: reason.into(),
                    created_at: None,
                })
                .map_err(|error| error.to_string())?;
        }
        "complete_review" => {
            let initiative = ledger
                .get_initiative(&binding.initiative_id)
                .map_err(|error| error.to_string())?;
            if initiative.status != InitiativeStatus::AwaitingMergeReview {
                return Err(
                    "final candidate review is only available in awaiting_merge_review".into(),
                );
            }
            ledger
                .transition_initiative(
                    &binding.initiative_id,
                    InitiativeStatus::Completed,
                    Role::Human,
                    reason,
                )
                .map_err(|error| error.to_string())?;
        }
        other => return Err(format!("unknown Studio control action: {other}")),
    }
    ledger
        .workspace_snapshot(&binding.initiative_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn studio_export_proof(req: StudioInitiativeRequest) -> Result<String, String> {
    let conn = bound_connection(&req)?;
    let report = Ledger::new(&conn)
        .generate_proof_report(&req.initiative_id)
        .map_err(|error| error.to_string())?;
    serde_json::to_string_pretty(&json!({
        "privacyWarning": "Exact context bundles and sensitive business context are excluded by default.",
        "report": report
    }))
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn dream_save_mandate(req: MandateRequest) -> Result<Value, String> {
    let canonical_repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    with_ledger(&req.session_id, &canonical_repo, |ledger| {
        ledger.upsert_mandate(&req.session_id, &canonical_repo, &req.mandate, "local-user")?;
        serde_json::to_value(&req.mandate).map_err(Into::into)
    })
}

/// Human approval boundary for standalone generated applications. The model
/// never supplies this path; it can only propose a bounded slug below it.
#[tauri::command]
pub(crate) fn dream_approve_output_root(req: DreamOutputRootRequest) -> Result<Value, String> {
    let repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let conn =
        init_audit(&PathBuf::from(&repo), &req.session_id).map_err(|error| error.to_string())?;
    let id = ApplicationWorkspaceManager::new(&conn)
        .approve_output_root(&req.session_id, &PathBuf::from(req.output_root))
        .map_err(|error| error.to_string())?;
    Ok(json!({"id":id,"approved":true}))
}

#[tauri::command]
pub(crate) fn dream_list_applications(req: StudioRepoRequest) -> Result<Value, String> {
    let repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let conn =
        init_audit(&PathBuf::from(&repo), &req.session_id).map_err(|error| error.to_string())?;
    let mut statement = conn.prepare("SELECT manifest_json FROM dream_applications a JOIN initiatives i ON i.id=a.initiative_id WHERE i.session_id=?1 AND i.repo_root=?2 ORDER BY a.created_at DESC").map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![req.session_id, repo], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?;
    let applications: Result<Vec<Value>, String> = rows
        .map(|row| {
            row.map_err(|error| error.to_string())
                .and_then(|value| serde_json::from_str(&value).map_err(|error| error.to_string()))
        })
        .collect();
    Ok(json!(applications?))
}

#[tauri::command]
pub(crate) fn dream_factory_start(req: DreamFactoryRequest) -> Result<Value, String> {
    let repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let conn =
        init_audit(&PathBuf::from(&repo), &req.session_id).map_err(|error| error.to_string())?;
    let controller = DreamFactoryController::new(&conn);
    let state = controller
        .start_factory(&req.session_id, &repo, &req.mandate_id)
        .map_err(|error| error.to_string())?;
    let approved: i64 = conn.query_row("SELECT COUNT(*) FROM dream_output_roots WHERE id=?1 AND session_id=?2 AND enabled=1 AND approved_by_source='local-user'", params![req.output_root_id,req.session_id], |row| row.get(0)).map_err(|error| error.to_string())?;
    if approved != 1 {
        return Err("Dream Factory requires a user-approved output root".into());
    }
    conn.execute(
        "UPDATE dream_factories SET output_root_id=?2,dream_target='new_application' WHERE id=?1",
        params![state.id, req.output_root_id],
    )
    .map_err(|error| error.to_string())?;
    serde_json::to_value(
        controller
            .load(&req.session_id, &repo)
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn dream_factory_state(req: StudioRepoRequest) -> Result<Value, String> {
    let repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let conn =
        init_audit(&PathBuf::from(&repo), &req.session_id).map_err(|error| error.to_string())?;
    serde_json::to_value(
        DreamFactoryController::new(&conn)
            .load(&req.session_id, &repo)
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn dream_factory_pause(req: DreamFactoryControlRequest) -> Result<Value, String> {
    let repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let conn =
        init_audit(&PathBuf::from(&repo), &req.session_id).map_err(|error| error.to_string())?;
    serde_json::to_value(
        DreamFactoryController::new(&conn)
            .pause_factory(&req.session_id, &repo)
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn dream_factory_resume(req: DreamFactoryControlRequest) -> Result<Value, String> {
    let repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let conn =
        init_audit(&PathBuf::from(&repo), &req.session_id).map_err(|error| error.to_string())?;
    serde_json::to_value(
        DreamFactoryController::new(&conn)
            .resume_factory(&req.session_id, &repo)
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn dream_factory_stop(req: DreamFactoryControlRequest) -> Result<Value, String> {
    let repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let conn =
        init_audit(&PathBuf::from(&repo), &req.session_id).map_err(|error| error.to_string())?;
    serde_json::to_value(
        DreamFactoryController::new(&conn)
            .stop_factory(&req.session_id, &repo, req.after_current.unwrap_or(false))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

/// Runs at most one backend-selected factory action. The UI may poll this
/// command, but it supplies neither a role nor a task; persistence determines
/// both. Long model calls are deliberately represented by their own tick.
#[tauri::command]
pub(crate) fn dream_factory_tick(req: DreamFactoryTickRequest) -> Result<Value, String> {
    let repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let conn =
        init_audit(&PathBuf::from(&repo), &req.session_id).map_err(|error| error.to_string())?;
    let controller = DreamFactoryController::new(&conn);
    let state = controller
        .recover_active_runs(&req.session_id, &repo)
        .map_err(|error| error.to_string())?;
    if state.status == dream_factory::FactoryStatus::Waiting
        && state.current_initiative_id.is_none()
    {
        controller
            .resume_factory(&req.session_id, &repo)
            .map_err(|error| error.to_string())?;
        return serde_json::to_value(
            controller
                .load(&req.session_id, &repo)
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string());
    }
    if !matches!(
        state.status,
        dream_factory::FactoryStatus::Running | dream_factory::FactoryStatus::StopAfterCurrent
    ) {
        return serde_json::to_value(state).map_err(|error| error.to_string());
    }
    if state.current_initiative_id.is_none() {
        drop(conn);
        let _ = dream_start_cycle(DreamCycleRequest {
            session_id: req.session_id.clone(),
            repo_root: repo.clone(),
            mandate_id: state.mandate_id.clone(),
            focus: String::new(),
        })?;
        let conn = init_audit(&PathBuf::from(&repo), &req.session_id)
            .map_err(|error| error.to_string())?;
        return serde_json::to_value(
            DreamFactoryController::new(&conn)
                .load(&req.session_id, &repo)
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string());
    }
    let initiative_id = state.current_initiative_id.clone().expect("checked");
    match state.stage.as_deref() {
        Some("scope_gate") => {
            Ledger::new(&conn)
                .raise_dream_mode_by_mandate(&initiative_id)
                .map_err(|error| error.to_string())?;
            let output_root_id: Option<String> = conn
                .query_row(
                    "SELECT output_root_id FROM dream_factories WHERE id=?1",
                    [&state.id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|error| error.to_string())?
                .flatten();
            if let Some(output_root_id) = output_root_id {
                let title: String = conn.query_row("SELECT json_extract(payload_json,'$.title') FROM dream_contracts WHERE initiative_id=?1 ORDER BY rowid DESC LIMIT 1", [&initiative_id], |row| row.get(0)).map_err(|error| error.to_string())?;
                let slug = format!(
                    "dream-{}",
                    initiative_id
                        .to_ascii_lowercase()
                        .replace('_', "-")
                        .replace(
                            |character: char| !character.is_ascii_alphanumeric()
                                && character != '-',
                            ""
                        )
                );
                ApplicationWorkspaceManager::new(&conn)
                    .create_offline_web_app(
                        &req.session_id,
                        &initiative_id,
                        &output_root_id,
                        &title,
                        &slug,
                    )
                    .map_err(|error| error.to_string())?;
                controller
                    .set_stage(
                        &state.id,
                        dream_factory::FactoryStage::ScopeGate,
                        dream_factory::FactoryStage::TaskSelection,
                        Some("ready_task"),
                    )
                    .map_err(|error| error.to_string())?;
            } else {
                controller
                    .set_stage(
                        &state.id,
                        dream_factory::FactoryStage::ScopeGate,
                        dream_factory::FactoryStage::WorktreePending,
                        Some("governed_worktree"),
                    )
                    .map_err(|error| error.to_string())?;
            }
        }
        Some("worktree_pending") => {
            let manager = WorktreeManager::new(&conn, PathBuf::from(&repo));
            let git = manager.inspect().map_err(|error| error.to_string())?;
            manager
                .create(
                    &initiative_id,
                    &git.current_commit,
                    "mandate-bound-worktree-policy",
                )
                .map_err(|error| error.to_string())?;
            controller
                .set_stage(
                    &state.id,
                    dream_factory::FactoryStage::WorktreePending,
                    dream_factory::FactoryStage::TaskSelection,
                    Some("ready_task"),
                )
                .map_err(|error| error.to_string())?;
        }
        Some("task_selection") => {
            let selected = controller
                .select_next_ready_task(&state.id, &initiative_id)
                .map_err(|error| error.to_string())?;
            if selected.is_none() {
                let unfinished: i64 = conn.query_row("SELECT COUNT(*) FROM studio_tasks WHERE initiative_id=?1 AND status!='passed'", [&initiative_id], |row| row.get(0)).map_err(|error| error.to_string())?;
                if unfinished == 0 {
                    controller
                        .set_stage(
                            &state.id,
                            dream_factory::FactoryStage::TaskSelection,
                            dream_factory::FactoryStage::FinalUxReview,
                            Some("ux_conformance_report"),
                        )
                        .map_err(|error| error.to_string())?;
                } else {
                    controller
                        .wait_with_reason(
                            &state.id,
                            "No dependency-ready task exists; a dependency is blocked or stale.",
                        )
                        .map_err(|error| error.to_string())?;
                }
            }
        }
        Some("task_build_pending") => {
            let task_id = state
                .active_task_id
                .as_deref()
                .ok_or_else(|| "factory task_build_pending state has no bound task".to_string())?;
            factory_build_and_review(&conn, &state.id, &initiative_id, task_id)?;
        }
        Some("task_revising") => {
            let task_id = state
                .active_task_id
                .as_deref()
                .ok_or_else(|| "factory task_revising state has no bound task".to_string())?;
            conn.execute(
                "UPDATE studio_tasks SET status='ready', updated_at=datetime('now') WHERE id=?1 AND status='revising'",
                [task_id],
            )
            .map_err(|error| error.to_string())?;
            controller
                .set_stage(
                    &state.id,
                    dream_factory::FactoryStage::TaskRevising,
                    dream_factory::FactoryStage::TaskSelection,
                    Some("ready_task"),
                )
                .map_err(|error| error.to_string())?;
        }
        Some("final_ux_review") => {
            invoke_factory_final_role(&conn, &initiative_id, Role::UxDesigner)?;
            controller
                .set_stage(
                    &state.id,
                    dream_factory::FactoryStage::FinalUxReview,
                    dream_factory::FactoryStage::FinalFdeReview,
                    Some("fde_outcome_review"),
                )
                .map_err(|error| error.to_string())?;
        }
        Some("final_fde_review") => {
            for role in [
                Role::Dreamer,
                Role::Skeptic,
                Role::Fde,
                Role::Architect,
                Role::Planner,
                Role::Builder,
                Role::Verifier,
                Role::Reviewer,
            ] {
                invoke_factory_final_role(&conn, &initiative_id, role)?;
            }
            let workspace: String = conn
                .query_row(
                    "SELECT workspace_path FROM dream_applications WHERE initiative_id=?1",
                    [&initiative_id],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            ApplicationWorkspaceManager::new(&conn)
                .validate_offline_web_app(&PathBuf::from(&workspace))
                .map_err(|error| format!("final launch validation failed: {error}"))?;
            verify_application_launch(&PathBuf::from(&workspace))?;
            let manifest: String = conn
                .query_row(
                    "SELECT manifest_json FROM dream_applications WHERE initiative_id=?1",
                    [&initiative_id],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            let mut manifest: Value =
                serde_json::from_str(&manifest).map_err(|error| error.to_string())?;
            manifest["status"] = json!("runnable_candidate");
            manifest["completedAt"] = json!(chrono_like_timestamp());
            conn.execute("UPDATE dream_applications SET status='runnable_candidate',manifest_json=?2,completed_at=datetime('now') WHERE initiative_id=?1",params![initiative_id,manifest.to_string()]).map_err(|error|error.to_string())?;
            controller
                .complete_concept(&state.id, dream_factory::FactoryStage::CandidateComplete)
                .map_err(|error| error.to_string())?;
        }
        _ => {}
    }
    serde_json::to_value(
        controller
            .load(&req.session_id, &repo)
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn chrono_like_timestamp() -> String {
    format!(
        "unix-nanos:{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or(0)
    )
}

fn verify_application_launch(workspace: &std::path::Path) -> Result<(), String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    drop(listener);
    let mut command = Command::new("python");
    command
        .args([
            "-m",
            "http.server",
            &port.to_string(),
            "--bind",
            "127.0.0.1",
            "--directory",
            &workspace.to_string_lossy(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("trusted preview server failed to start: {e}"))?;
    let url = format!("http://127.0.0.1:{port}/index.html");
    let mut healthy = false;
    for _ in 0..30 {
        match ureq::get(&url).timeout(Duration::from_secs(1)).call() {
            Ok(response) if response.status() == 200 => {
                healthy = true;
                break;
            }
            _ => std::thread::sleep(Duration::from_millis(100)),
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    if !healthy {
        return Err("application preview did not serve index.html successfully".into());
    }
    Ok(())
}

fn invoke_factory_final_role(
    conn: &rusqlite::Connection,
    initiative_id: &str,
    role: Role,
) -> Result<(), String> {
    let task = if matches!(role, Role::Builder | Role::Verifier | Role::Reviewer) {
        conn.query_row("SELECT id FROM studio_tasks WHERE initiative_id=?1 AND status='passed' ORDER BY rowid DESC LIMIT 1",[initiative_id],|row|row.get::<_,String>(0)).optional().map_err(|error|error.to_string())?
    } else {
        None
    };
    let config = factory_role_runtime(conn, initiative_id, role)?;
    let prepared = prepare_role_run(
        conn,
        initiative_id,
        task.as_deref(),
        role,
        &config.0,
        &config.1,
    )
    .map_err(|error| error.to_string())?;
    if config.0 == "fake" {
        let initiative = Ledger::new(conn)
            .get_initiative(initiative_id)
            .map_err(|error| error.to_string())?;
        let payload = fake_role_artifact(role, FakeScenario::SuccessfulStudio, &initiative);
        let artifact_type = payload
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("finding");
        Ledger::new(conn)
            .publish_artifact(
                &ArtifactEnvelope {
                    operation_id: new_id("OP"),
                    initiative_id: initiative_id.into(),
                    task_id: task.clone(),
                    role,
                    artifact_type: artifact_type.into(),
                    schema_version: 1,
                    spec_version: initiative.active_spec_version,
                    source_context_bundle_id: Some(prepared.context_capsule.id.clone()),
                    reason: "Deterministic final convergence fixture".into(),
                    expected_outcome: "Persist role-specific final evidence".into(),
                    payload,
                },
                Some(&prepared.run_id),
                "Final convergence fixture artifact.",
            )
            .map_err(|error| error.to_string())?;
    } else {
        let lease = studio_role_scheduler()
            .acquire(&prepared.run_id)
            .map_err(|error| error.to_string())?;
        let result = run_configured_role(
            conn,
            &prepared,
            &config.0,
            &config.1,
            config.2.as_deref(),
            config.3,
        );
        drop(lease);
        result?;
    }
    complete_role_run(
        conn,
        &prepared,
        "completed",
        "final_convergence_artifact",
        None,
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) fn dream_start_cycle(req: DreamCycleRequest) -> Result<Value, String> {
    let canonical_repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let focus = if req.focus.trim().is_empty() {
        "Invent a new local-first app that would make software development more trustworthy, useful, and delightful; challenge it, scope it, and define the smallest reversible build path."
    } else {
        req.focus.trim()
    };
    let conn = init_audit(&PathBuf::from(&canonical_repo), &req.session_id)
        .map_err(|error| error.to_string())?;
    let ledger = Ledger::new(&conn);
    let initiative = ledger
        .create_initiative(
            &req.session_id,
            &canonical_repo,
            &format!("Dream cycle: {}", focus),
            InitiativeMode::DreamIdeation,
            "standing_mandate_cycle",
            Some(&req.mandate_id),
        )
        .map_err(|error| error.to_string())?;
    let factory_id = DreamFactoryController::new(&conn)
        .active_factory_id(&req.session_id, &canonical_repo)
        .map_err(|error| error.to_string())?;
    if let Some(factory_id) = factory_id.as_deref() {
        DreamFactoryController::new(&conn)
            .begin_concept(factory_id, &initiative.id)
            .map_err(|error| error.to_string())?;
    }
    for status in [
        InitiativeStatus::Discovery,
        InitiativeStatus::Concepting,
        InitiativeStatus::Challenging,
    ] {
        ledger
            .transition_initiative(
                &initiative.id,
                status,
                Role::System,
                "bounded Dream ideation cycle",
            )
            .map_err(|error| error.to_string())?;
    }
    ensure_dream_context_seed(&conn, &initiative, focus)?;
    let config: (String, String, Option<String>, i64) = conn
        .query_row(
            "SELECT runtime, model, endpoint_url, timeout_seconds FROM role_runtime_configs
             WHERE session_id=?1 AND role='dreamer'",
            [&req.session_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .unwrap_or_else(|| ("fake".into(), "studio-fixture-v1".into(), None, 300));
    let prepared = prepare_role_run(
        &conn,
        &initiative.id,
        None,
        Role::Dreamer,
        &config.0,
        &config.1,
    )
    .map_err(|error| error.to_string())?;
    if config.0 == "fake" {
        let mut dream =
            fake_role_artifact(Role::Dreamer, FakeScenario::SuccessfulStudio, &initiative);
        // Fixtures must model novel consecutive Dreams rather than exercising
        // the deduplication branch on every factory iteration.
        if let Some(title) = dream.get("title").and_then(Value::as_str) {
            dream["title"] = Value::String(format!("{title} {}", initiative.id));
        }
        let artifact = ArtifactEnvelope {
            operation_id: new_id("OP"),
            initiative_id: initiative.id.clone(),
            task_id: None,
            role: Role::Dreamer,
            artifact_type: "dream_contract".into(),
            schema_version: 1,
            spec_version: initiative.active_spec_version,
            source_context_bundle_id: Some(prepared.context_capsule.id.clone()),
            reason: "Generate one mandate-bound candidate for the Dream inbox".into(),
            expected_outcome: "A deduplicated, reversible concept with explicit assumptions".into(),
            payload: dream.clone(),
        };
        ledger
            .publish_artifact(
                &artifact,
                Some(&prepared.run_id),
                "Fake Runtime produced a deterministic bounded Dream Contract fixture.",
            )
            .map_err(|error| error.to_string())?;
        ledger
            .create_dream(&initiative.id, &dream)
            .map_err(|error| error.to_string())?;
        ledger
            .publish_belief(
                &initiative.id,
                None,
                initiative.active_spec_version,
                Some(&prepared.run_id),
                Role::Dreamer,
                &json!({"candidateNovel":true,"safeToExplore":true,"repositoryModified":false,"fixture":true}),
            )
            .map_err(|error| error.to_string())?;
    } else {
        let lease = studio_role_scheduler()
            .acquire(&prepared.run_id)
            .map_err(|error| error.to_string())?;
        let result = run_configured_role(
            &conn,
            &prepared,
            &config.0,
            &config.1,
            config.2.as_deref(),
            config.3,
        );
        drop(lease);
        if let Err(error) = result {
            complete_role_run(
                &conn,
                &prepared,
                "failed",
                "runtime_or_protocol_error",
                Some(&error),
            )
            .map_err(|completion| completion.to_string())?;
            return Err(error);
        }
    }
    let dream_id: String = conn
        .query_row(
            "SELECT id FROM dream_contracts WHERE initiative_id=?1 ORDER BY created_at DESC, rowid DESC LIMIT 1",
            [&initiative.id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Dreamer completed without a required dream_contract candidate".to_string())?;
    let candidate_initiative_id: String = conn
        .query_row(
            "SELECT initiative_id FROM dream_contracts WHERE id=?1",
            [&dream_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let team = run_dream_team(
        &conn,
        &initiative,
        &config.0,
        &config.1,
        config.2.as_deref(),
        config.3,
        factory_id.as_deref(),
    )
    .map_err(|error| error.to_string())?;
    complete_role_run(&conn, &prepared, "completed", "valid_dream_contract", None)
        .map_err(|error| error.to_string())?;
    ledger
        .record_event(OrchestrationEvent {
            id: new_id("EVENT"),
            initiative_id: initiative.id.clone(),
            task_id: None,
            actor_role: Role::Dreamer,
            kind: "dream.candidate_created".into(),
            requirement_ids: vec![],
            adr_ids: vec![],
            assumption_ids: vec![],
            features: BTreeMap::new(),
            provenance: format!("{}:{}", config.0, config.1),
            redacted_summary: "Dream candidate entered the inbox; no repository changes were made."
                .into(),
            created_at: None,
        })
        .map_err(|error| error.to_string())?;
    Ok(json!({
        "initiative": ledger.get_initiative(&initiative.id).map_err(|error| error.to_string())?,
        "dreamId": dream_id,
        "candidateInitiativeId": candidate_initiative_id,
        "team": team,
        "repositoryModified": false,
        "continuousOperation": "bounded cycles while enabled and while the application is running"
    }))
}

fn ensure_dream_context_seed(
    conn: &rusqlite::Connection,
    initiative: &intent_ledger::Initiative,
    focus: &str,
) -> Result<(), String> {
    let objective_id = format!("OBJ-DREAM-{}", initiative.id);
    let assumption_id = format!("ASM-DREAM-{}", initiative.id);
    let constraint_id = format!("CON-DREAM-{}", initiative.id);
    let requirement_id = format!("REQ-DREAM-{}", initiative.id);
    conn.execute(
        "INSERT OR IGNORE INTO objectives (id, initiative_id, spec_version, status, payload_json)
         VALUES (?1, ?2, ?3, 'active', ?4)",
        params![
            objective_id,
            initiative.id,
            initiative.active_spec_version,
            serde_json::to_string(&json!({
                "title": "Discover and shape a valuable new app concept",
                "focus": focus,
                "source": "background_dreamer_mandate"
            }))
            .map_err(|error| error.to_string())?
        ],
    )
    .map_err(|error| error.to_string())?;
    conn.execute(
        "INSERT OR IGNORE INTO constraints (id, initiative_id, spec_version, kind, attributable_to, testable, payload_json)
         VALUES (?1, ?2, ?3, 'dream_safety', 'standing_mandate', 1, ?4)",
        params![
            constraint_id,
            initiative.id,
            initiative.active_spec_version,
            serde_json::to_string(&json!({
                "statement": "No active-branch writes, package installs, network expansion, or autonomous merge; work remains bounded and checkpointed."
            }))
            .map_err(|error| error.to_string())?
        ],
    )
    .map_err(|error| error.to_string())?;
    conn.execute(
        "INSERT OR IGNORE INTO assumptions (id, initiative_id, spec_version, kind, status, impact_if_false, confidence, payload_json)
         VALUES (?1, ?2, ?3, 'dream_discovery', 'active', 'The concept can be explored reversibly without active-branch writes.', 0.5, ?4)",
        params![
            assumption_id,
            initiative.id,
            initiative.active_spec_version,
            serde_json::to_string(&json!({"statement":"A small, testable concept can be found and challenged by the team."}))
                .map_err(|error| error.to_string())?
        ],
    )
    .map_err(|error| error.to_string())?;
    conn.execute(
        "INSERT OR IGNORE INTO requirements (id, initiative_id, spec_version, status, required_evidence_json, payload_json)
         VALUES (?1, ?2, ?3, 'proposed', ?4, ?5)",
        params![
            requirement_id,
            initiative.id,
            initiative.active_spec_version,
            serde_json::to_string(&vec!["dream_contract", "dream_team_handoff"]).map_err(|error| error.to_string())?,
            serde_json::to_string(&json!({"statement":"Produce one durable Dream Contract and a sequential team work-through."})).map_err(|error| error.to_string())?
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

/// Give every bounded Dream cycle a real team pass. Roles without an explicit
/// runtime configuration inherit the Dreamer lane, so configuring Qwen once
/// is enough to let the concept be challenged, shaped, and planned by the
/// sequential local team. Each role is independently audited; a specialist
/// failure is recorded and does not erase the valid candidate.
fn run_dream_team(
    conn: &rusqlite::Connection,
    initiative: &intent_ledger::Initiative,
    fallback_runtime: &str,
    fallback_model: &str,
    fallback_endpoint: Option<&str>,
    fallback_timeout: i64,
    factory_id: Option<&str>,
) -> Result<Vec<Value>, String> {
    let roles = [
        Role::Skeptic,
        Role::Fde,
        Role::UxDesigner,
        Role::Architect,
        Role::Planner,
        Role::Builder,
        Role::Verifier,
        Role::Reviewer,
    ];
    let mut results = Vec::new();
    for (index, role) in roles.into_iter().enumerate() {
        let configured: Option<(String, String, Option<String>, i64)> = conn
            .query_row(
                "SELECT runtime, model, endpoint_url, timeout_seconds FROM role_runtime_configs
                 WHERE session_id=?1 AND role=?2",
                params![initiative.session_id, role.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let (runtime, model, endpoint, timeout) = configured.unwrap_or_else(|| {
            (
                fallback_runtime.to_string(),
                fallback_model.to_string(),
                fallback_endpoint.map(str::to_string),
                fallback_timeout,
            )
        });
        let task_binding = if matches!(role, Role::Builder | Role::Verifier | Role::Reviewer) {
            conn.query_row(
                "SELECT id FROM studio_tasks WHERE initiative_id=?1 ORDER BY rowid LIMIT 1",
                [&initiative.id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
        } else {
            None
        };
        let prepared = prepare_role_run(
            conn,
            &initiative.id,
            task_binding.as_deref(),
            role,
            &runtime,
            &model,
        )
        .map_err(|error| error.to_string())?;
        // Fake Runtime is deterministic local fixture work, not model
        // inference.  It must not contend with a real single-inference lease
        // (in particular when integration fixtures run in parallel).
        let lease = if runtime == "fake" {
            None
        } else {
            Some(
                studio_role_scheduler()
                    .acquire(&prepared.run_id)
                    .map_err(|error| error.to_string())?,
            )
        };
        let result = if runtime == "fake" {
            run_fake_single_role(conn, &prepared)
        } else {
            run_configured_role(
                conn,
                &prepared,
                &runtime,
                &model,
                endpoint.as_deref(),
                timeout,
            )
        };
        drop(lease);
        match result {
            Ok(value) => {
                complete_role_run(conn, &prepared, "completed", "dream_team_handoff", None)
                    .map_err(|error| error.to_string())?;
                results.push(
                    json!({"role": role.as_str(), "status": "completed", "operation": value}),
                );
                if let Some(factory_id) = factory_id.filter(|_| index < 5) {
                    let (from, to, expected) = match index {
                        0 => (
                            dream_factory::FactoryStage::DreamPending,
                            dream_factory::FactoryStage::FdePending,
                            "fde_brief",
                        ),
                        1 => (
                            dream_factory::FactoryStage::FdePending,
                            dream_factory::FactoryStage::UxPending,
                            "ux_contract",
                        ),
                        2 => (
                            dream_factory::FactoryStage::UxPending,
                            dream_factory::FactoryStage::ArchitectPending,
                            "architecture_alternatives",
                        ),
                        3 => (
                            dream_factory::FactoryStage::ArchitectPending,
                            dream_factory::FactoryStage::PlanPending,
                            "task_graph",
                        ),
                        _ => (
                            dream_factory::FactoryStage::PlanPending,
                            dream_factory::FactoryStage::ScopeGate,
                            "implementation_spec",
                        ),
                    };
                    DreamFactoryController::new(conn)
                        .set_stage(factory_id, from, to, Some(expected))
                        .map_err(|error| error.to_string())?;
                }
            }
            Err(error) => {
                complete_role_run(
                    conn,
                    &prepared,
                    "failed",
                    "dream_team_role_error",
                    Some(&error),
                )
                .map_err(|completion| completion.to_string())?;
                return Err(format!(
                    "{} Product Council stage failed: {error}",
                    role.as_str()
                ));
            }
        }
    }
    // The Dreamer synthesizes the complete Council record into the next
    // concept version before construction. This is a real second invocation,
    // with the Council artifacts present in its Context Capsule.
    let synthesis = prepare_role_run(
        conn,
        &initiative.id,
        None,
        Role::Dreamer,
        fallback_runtime,
        fallback_model,
    )
    .map_err(|error| error.to_string())?;
    let synthesis_result = if fallback_runtime == "fake" {
        run_fake_single_role(conn, &synthesis)
    } else {
        let lease = studio_role_scheduler()
            .acquire(&synthesis.run_id)
            .map_err(|error| error.to_string())?;
        let result = run_configured_role(
            conn,
            &synthesis,
            fallback_runtime,
            fallback_model,
            fallback_endpoint,
            fallback_timeout,
        );
        drop(lease);
        result
    };
    match synthesis_result {
        Ok(value) => {
            complete_role_run(
                conn,
                &synthesis,
                "completed",
                "product_council_concept_revision",
                None,
            )
            .map_err(|error| error.to_string())?;
            results.push(json!({"role":"dreamer","status":"concept_revised","operation":value}));
        }
        Err(error) => {
            complete_role_run(
                conn,
                &synthesis,
                "failed",
                "concept_revision_failed",
                Some(&error),
            )
            .map_err(|completion| completion.to_string())?;
            return Err(error);
        }
    }
    Ok(results)
}

#[tauri::command]
pub(crate) fn dream_list_inbox(req: StudioRepoRequest) -> Result<Value, String> {
    let canonical_repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let conn = init_audit(&PathBuf::from(&canonical_repo), &req.session_id)
        .map_err(|error| error.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT json_object('id', d.id, 'initiativeId', d.initiative_id, 'horizon', d.horizon,
                                'status', d.status, 'payload', json(d.payload_json), 'createdAt', d.created_at)
             FROM dream_contracts d JOIN initiatives i ON i.id=d.initiative_id
             WHERE i.session_id=?1 AND i.repo_root=?2 ORDER BY d.created_at DESC, d.rowid DESC",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(params![req.session_id, canonical_repo], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|error| error.to_string())?;
    let values: std::result::Result<Vec<Value>, String> = rows
        .map(|row| {
            row.map_err(|error| error.to_string())
                .and_then(|value| serde_json::from_str(&value).map_err(|error| error.to_string()))
        })
        .collect();
    serde_json::to_value(values?).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn dream_action(req: DreamActionRequest) -> Result<Value, String> {
    let canonical_repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let conn = init_audit(&PathBuf::from(&canonical_repo), &req.session_id)
        .map_err(|error| error.to_string())?;
    let (initiative_id, payload): (String, String) = conn
        .query_row(
            "SELECT d.initiative_id, d.payload_json FROM dream_contracts d
             JOIN initiatives i ON i.id=d.initiative_id
             WHERE d.id=?1 AND i.session_id=?2 AND i.repo_root=?3",
            params![req.dream_id, req.session_id, canonical_repo],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Dream candidate binding was not found".to_string())?;
    conn.execute_batch("SAVEPOINT human_dream_action")
        .map_err(|error| error.to_string())?;
    let outcome = (|| -> Result<Value, String> {
        let ledger = Ledger::new(&conn);
        let (status, promoted, mode) = match req.action.as_str() {
            "reject" => {
                ledger
                    .transition_dream(&req.dream_id, DreamStatus::Rejected, Role::Human)
                    .map_err(|error| error.to_string())?;
                (DreamStatus::Rejected.as_str(), None, None)
            }
            "archive" => {
                ledger
                    .transition_dream(&req.dream_id, DreamStatus::Archived, Role::Human)
                    .map_err(|error| error.to_string())?;
                (DreamStatus::Archived.as_str(), None, None)
            }
            "approve_prototype" => {
                ledger
                    .set_dream_mode(&initiative_id, InitiativeMode::DreamPrototype, Role::Human)
                    .map_err(|error| error.to_string())?;
                ledger
                    .transition_dream(&req.dream_id, DreamStatus::PrototypeApproved, Role::Human)
                    .map_err(|error| error.to_string())?;
                (
                    DreamStatus::PrototypeApproved.as_str(),
                    None,
                    Some(InitiativeMode::DreamPrototype.as_str()),
                )
            }
            "enable_incubator" => {
                ledger
                    .set_dream_mode(&initiative_id, InitiativeMode::DreamIncubator, Role::Human)
                    .map_err(|error| error.to_string())?;
                (
                    "incubator_enabled",
                    None,
                    Some(InitiativeMode::DreamIncubator.as_str()),
                )
            }
            "promote" => {
                ledger
                    .transition_dream(&req.dream_id, DreamStatus::PromotedToGoal, Role::Human)
                    .map_err(|error| error.to_string())?;
                let dream: Value =
                    serde_json::from_str(&payload).map_err(|error| error.to_string())?;
                let title = dream
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("Promoted Dream");
                let studio = bootstrap_studio(&conn, &req.session_id, &canonical_repo, title)
                    .map_err(|error| error.to_string())?;
                (DreamStatus::PromotedToGoal.as_str(), Some(studio), None)
            }
            other => return Err(format!("unknown Dream action: {other}")),
        };
        Ok(json!({
            "dreamId": req.dream_id,
            "status": status,
            "mode": mode,
            "promotedInitiative": promoted,
            "sourceInitiativeId": initiative_id,
            "humanApproved": true
        }))
    })();
    match outcome {
        Ok(value) => {
            conn.execute_batch("RELEASE human_dream_action")
                .map_err(|error| error.to_string())?;
            Ok(value)
        }
        Err(error) => {
            let _ =
                conn.execute_batch("ROLLBACK TO human_dream_action; RELEASE human_dream_action");
            Err(error)
        }
    }
}

#[tauri::command]
pub(crate) fn governed_worktree_inspect(req: StudioRepoRequest) -> Result<Value, String> {
    let canonical_repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let conn = init_audit(&PathBuf::from(&canonical_repo), &req.session_id)
        .map_err(|error| error.to_string())?;
    serde_json::to_value(
        WorktreeManager::new(&conn, &canonical_repo)
            .inspect()
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn governed_worktree_create(req: WorktreeCreateRequest) -> Result<Value, String> {
    let binding = StudioInitiativeRequest {
        session_id: req.session_id,
        repo_root: req.repo_root,
        initiative_id: req.initiative_id,
    };
    let conn = bound_connection(&binding)?;
    serde_json::to_value(
        WorktreeManager::new(
            &conn,
            canonical(&binding.repo_root).map_err(|error| error.to_string())?,
        )
        .create(
            &binding.initiative_id,
            &req.approved_base_commit,
            "local-user",
        )
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn governed_worktree_diff(req: WorktreeRequest) -> Result<Value, String> {
    let canonical_repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let conn = init_audit(&PathBuf::from(&canonical_repo), &req.session_id)
        .map_err(|error| error.to_string())?;
    serde_json::to_value(
        WorktreeManager::new(&conn, &canonical_repo)
            .candidate_diff(&req.worktree_id)
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn governed_worktree_cleanup(req: WorktreeCleanupRequest) -> Result<Value, String> {
    let canonical_repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let conn = init_audit(&PathBuf::from(&canonical_repo), &req.session_id)
        .map_err(|error| error.to_string())?;
    serde_json::to_value(
        WorktreeManager::new(&conn, &canonical_repo)
            .cleanup(&req.worktree_id, &req.confirmation_token)
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn studio_pulse(req: StudioInitiativeRequest) -> Result<Value, String> {
    let conn = bound_connection(&req)?;
    let ledger = Ledger::new(&conn);
    let source_events = ledger
        .list_events(&req.initiative_id, 1000)
        .map_err(|error| error.to_string())?;
    let mut events: Vec<PulseEvent> = source_events
        .iter()
        .rev()
        .enumerate()
        .map(|(index, event)| PulseEvent::from_orchestration(event, index as f64))
        .collect();
    for event in &mut events {
        event
            .features
            .entry("intervention_urgency".into())
            .or_insert(0.0);
    }
    let mut belief_stmt = conn
        .prepare(
            "SELECT id, role, payload_json FROM agent_beliefs
             WHERE initiative_id=?1 ORDER BY created_at, rowid",
        )
        .map_err(|error| error.to_string())?;
    let belief_rows = belief_stmt
        .query_map([&req.initiative_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut beliefs = Vec::new();
    for row in belief_rows {
        let (id, role, payload) = row.map_err(|error| error.to_string())?;
        let payload: Value = serde_json::from_str(&payload).map_err(|error| error.to_string())?;
        let parse_truth_map = |key: &str| -> BTreeMap<String, bool> {
            payload
                .get(key)
                .and_then(Value::as_object)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|(id, value)| value.as_bool().map(|truth| (id.clone(), truth)))
                        .collect()
                })
                .unwrap_or_default()
        };
        beliefs.push(BeliefSnapshot {
            id,
            role: Role::try_from(role.as_str()).map_err(|error| error.to_string())?,
            requirement_complete: parse_truth_map("requirementComplete"),
            adr_followed: parse_truth_map("adrFollowed"),
            confidence: payload
                .get("confidence")
                .and_then(Value::as_f64)
                .unwrap_or(0.5),
        });
    }
    let findings = SymbolicMonitor.evaluate(&events, &beliefs);
    let mut temporal = RuleBasedTemporalObserver::default();
    let mut last = None;
    for event in &events {
        let delta = last
            .map(|prior| event.timestamp_seconds - prior)
            .unwrap_or(0.0);
        temporal
            .ingest(event, delta)
            .map_err(|error| error.to_string())?;
        last = Some(event.timestamp_seconds);
    }
    let temporal_findings = temporal.evaluate();
    for finding in findings.iter().chain(temporal_findings.iter()) {
        let finding_id = new_id("PULSE");
        conn.execute(
            "INSERT INTO pulse_findings (id, initiative_id, task_id, kind, severity, source, experimental, payload_json)
             VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7)",
            params![finding_id, req.initiative_id, finding.kind, finding.severity, finding.source, i64::from(finding.experimental), serde_json::to_string(finding).map_err(|error| error.to_string())?],
        ).map_err(|error| error.to_string())?;
        if let Some(proposal) = route_intervention(finding) {
            conn.execute(
                "INSERT INTO interventions (id, initiative_id, kind, source_finding_id, status, rationale)
                 VALUES (?1, ?2, ?3, ?4, 'proposed', ?5)",
                params![new_id("INTERVENTION"), req.initiative_id, format!("{:?}", proposal.kind), finding_id, proposal.rationale],
            ).map_err(|error| error.to_string())?;
        }
    }
    Ok(json!({
        "symbolicFindings": findings,
        "temporalFindings": temporal_findings,
        "beliefCount": beliefs.len(),
        "ruleObserver": temporal.model_metadata(),
        "liquidObserver": {
            "observerKind":"liquid_shadow","experimental":true,"shadowOnly":true,"hasAuthority":false,
            "available":false,"reason":"No validated local calibrated weights are loaded; random weights are never used."
        }
    }))
}

#[tauri::command]
pub(crate) fn validate_declarative_prototype(
    req: PrototypeValidationRequest,
) -> Result<Value, String> {
    let document: PrototypeDocument =
        serde_json::from_value(req.document).map_err(|error| error.to_string())?;
    validate_prototype(&document).map_err(|error| error.to_string())?;
    serde_json::to_value(document).map_err(|error| error.to_string())
}

fn run_fake_single_role(
    conn: &rusqlite::Connection,
    prepared: &orchestration_core::PreparedRoleRun,
) -> Result<Value, String> {
    let ledger = Ledger::new(conn);
    let initiative = ledger
        .get_initiative(&prepared.context_capsule.initiative_id)
        .map_err(|error| error.to_string())?;
    let payload = fake_role_artifact(prepared.role, FakeScenario::SuccessfulStudio, &initiative);
    let artifact_type = payload
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "Fake Runtime artifact omitted type".to_string())?;
    let envelope = ArtifactEnvelope {
        operation_id: new_id("OP"),
        initiative_id: initiative.id.clone(),
        task_id: prepared.context_capsule.task_id.clone(),
        role: prepared.role,
        artifact_type: artifact_type.into(),
        schema_version: 1,
        spec_version: initiative.active_spec_version,
        source_context_bundle_id: Some(prepared.context_capsule.id.clone()),
        reason: "Exercise one configured role through the typed Studio protocol".into(),
        expected_outcome: "A persisted role-permitted artifact".into(),
        payload: payload.clone(),
    };
    ledger
        .publish_artifact(
            &envelope,
            Some(&prepared.run_id),
            "Fake Runtime role artifact accepted.",
        )
        .map_err(|error| error.to_string())?;
    let header = StudioOperationHeader {
        operation_id: envelope.operation_id.clone(),
        initiative_id: envelope.initiative_id.clone(),
        task_id: envelope.task_id.clone(),
        role: prepared.role.as_str().into(),
        artifact_type: envelope.artifact_type.clone(),
        schema_version: envelope.schema_version,
        spec_version: envelope.spec_version,
        reason: envelope.reason.clone(),
        expected_outcome: envelope.expected_outcome.clone(),
        source_context_bundle_id: envelope.source_context_bundle_id.clone(),
    };
    materialize_authoritative_artifact(conn, prepared, prepared.role, &header, &payload)?;
    if prepared.role == Role::Dreamer {
        ledger
            .create_dream(&initiative.id, &payload)
            .map_err(|error| error.to_string())?;
    }
    serde_json::to_value(json!({"type":"propose_artifact","header":envelope,"payload":payload}))
        .map_err(|error| error.to_string())
}

fn run_configured_role(
    conn: &rusqlite::Connection,
    prepared: &orchestration_core::PreparedRoleRun,
    provider: &str,
    model: &str,
    endpoint_url: Option<&str>,
    timeout_seconds: i64,
) -> Result<Value, String> {
    if let Err(error) = validate_capsule_freshness(conn, &prepared.context_capsule) {
        let detail = error.to_string();
        let kind = if detail.contains("ADR") {
            "context.stale_adr"
        } else if detail.contains("spec") {
            "context.stale_spec"
        } else {
            "context.binding_invalid"
        };
        Ledger::new(conn)
            .record_event(OrchestrationEvent {
                id: new_id("EVENT"),
                initiative_id: prepared.context_capsule.initiative_id.clone(),
                task_id: prepared.context_capsule.task_id.clone(),
                actor_role: Role::System,
                kind: kind.into(),
                requirement_ids: vec![],
                adr_ids: prepared
                    .context_capsule
                    .active_adr_versions
                    .keys()
                    .cloned()
                    .collect(),
                assumption_ids: vec![],
                features: BTreeMap::from([
                    ("context_drift".into(), 1.0),
                    ("severity".into(), 0.95),
                ]),
                provenance: "context-os-v1".into(),
                redacted_summary: detail,
                created_at: None,
            })
            .map_err(|record_error| record_error.to_string())?;
        return Err(error.to_string());
    }
    let endpoint = endpoint_url
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "configured non-Fake role requires an endpoint URL".to_string())?;
    let classification = classify_endpoint_url(endpoint);
    if classification != "local" {
        verify_endpoint_approval(conn, endpoint, &classification)?;
    }
    let messages: Vec<RuntimeMessage> = prepared
        .context_capsule
        .exact_messages
        .iter()
        .map(|message| RuntimeMessage {
            role: message.role.clone(),
            content: message.content.clone(),
        })
        .collect();
    let retry_messages = messages.clone();
    let (sender, receiver) = mpsc::sync_channel(1);
    let provider_owned = provider.to_string();
    let endpoint_owned = endpoint.to_string();
    let model_owned = model.to_string();
    let maximum_output_tokens = u32::try_from(prepared.context_capsule.reserved_output_tokens)
        .map_err(|_| "configured maximum output token limit exceeds runtime range".to_string())?;
    std::thread::spawn(move || {
        let generated = match provider_owned.as_str() {
            "cloud-openai" => call_cloud_openai(
                &endpoint_owned,
                &model_owned,
                &messages,
                0.2,
                maximum_output_tokens,
            ),
            "cloud-anthropic" => call_cloud_anthropic(
                &endpoint_owned,
                &model_owned,
                &messages,
                maximum_output_tokens,
            ),
            _ => call_openai_compatible_endpoint(
                &endpoint_owned,
                &model_owned,
                &messages,
                0.2,
                maximum_output_tokens,
                Some("json_object"),
            ),
        };
        let _ = sender.send(generated);
    });
    let started = Instant::now();
    let timeout = Duration::from_secs(timeout_seconds.clamp(1, 1800) as u64);
    let mut content = loop {
        studio_role_scheduler()
            .check(&prepared.run_id, started, timeout)
            .map_err(|error| error.to_string())?;
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(content)) => break content,
            Ok(Err(error)) => {
                record_context_generation_failure(conn, prepared, &error)?;
                return Err(error);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let error = "configured role runtime ended without a response".to_string();
                record_context_generation_failure(conn, prepared, &error)?;
                return Err(error);
            }
        }
    };
    if content.chars().count() > 1_000_000 {
        let error = "configured role output exceeded the 1,000,000 character limit".to_string();
        record_context_generation_failure(conn, prepared, &error)?;
        return Err(error);
    }
    let constructing: bool = conn.query_row("SELECT COUNT(*) FROM dream_applications WHERE initiative_id=?1 AND status IN ('building','polishing')",[&prepared.context_capsule.initiative_id],|row|row.get::<_,i64>(0)).unwrap_or(0)>0;
    if !role_output_shape_valid(&content, prepared.role, constructing) {
        let mut repair_messages = retry_messages;
        repair_messages.push(RuntimeMessage{role:"user".into(),content:format!("Protocol repair: return exactly one JSON object for the {} stage; no prose or fences. Required shape: {}",prepared.role.as_str(),role_repair_schema(prepared.role))});
        content = match provider {
            "cloud-openai" => call_cloud_openai(
                endpoint,
                model,
                &repair_messages,
                0.0,
                maximum_output_tokens,
            ),
            "cloud-anthropic" => {
                call_cloud_anthropic(endpoint, model, &repair_messages, maximum_output_tokens)
            }
            _ => call_openai_compatible_endpoint(
                endpoint,
                model,
                &repair_messages,
                0.0,
                maximum_output_tokens,
                Some("json_object"),
            ),
        }
        .map_err(|error| format!("{} protocol repair failed: {error}", prepared.role.as_str()))?;
    }
    let operation = match strict_parse_studio_operation(&content) {
        Ok(operation) => operation,
        Err(error) => {
            // Local structured-output models sometimes return the requested
            // role artifact without its transport envelope. Rebind that
            // bounded object to the persisted run/capsule; never accept model
            // supplied initiative, task, role, spec, or workspace authority.
            let direct = extract_role_json(&content);
            if let Some(mut payload) = direct.and_then(|value| {
                value
                    .get("payload")
                    .cloned()
                    .or_else(|| value.get("artifact").cloned())
                    .or_else(|| value.get("applicationConcept").cloned())
                    .or_else(|| value.get("concept").cloned())
                    .or_else(|| value.get("dreamContract").cloned())
                    .or(Some(value))
            }) {
                if prepared.role == Role::Dreamer {
                    payload = normalize_dream_contract_payload(payload);
                }
                let default_type = match prepared.role {
                    Role::Dreamer => "dream_contract",
                    Role::Skeptic => "finding",
                    Role::Fde => "fde_brief",
                    Role::UxDesigner => "ux_contract",
                    Role::Architect => "architecture_alternatives",
                    Role::Planner => "task_graph",
                    Role::Builder => "finding",
                    Role::Verifier => "verification_verdict",
                    Role::Reviewer => "review_verdict",
                    _ => "finding",
                };
                if payload.get("type").is_none() {
                    payload["type"] = json!(default_type)
                }
                if prepared.role == Role::Builder
                    && payload.get("type").and_then(Value::as_str) == Some("propose_patch")
                {
                    let patch = payload;
                    payload = json!({"type":"patch_proposal","operation":patch});
                }
                let artifact_type = payload
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or(default_type)
                    .to_string();
                AgentOperation::ProposeArtifact {
                    header: StudioOperationHeader {
                        operation_id: new_id("OP"),
                        initiative_id: prepared.context_capsule.initiative_id.clone(),
                        task_id: prepared.context_capsule.task_id.clone(),
                        role: prepared.role.as_str().into(),
                        artifact_type,
                        schema_version: 1,
                        spec_version: prepared.context_capsule.active_spec_version,
                        reason:
                            "Direct configured-role artifact normalized into the governed protocol."
                                .into(),
                        expected_outcome:
                            "A schema-validated, capsule-bound authoritative role result.".into(),
                        source_context_bundle_id: Some(prepared.context_capsule.id.clone()),
                    },
                    payload,
                }
            } else {
                record_context_generation_failure(conn, prepared, &error)?;
                return Err(format!(
                    "{error}; bounded model output: {}",
                    content.chars().take(2000).collect::<String>()
                ));
            }
        }
    };
    if let Some(context_capsule) = apply_studio_operation(conn, prepared, &operation)? {
        let mut continued = prepared.clone();
        continued.context_capsule = context_capsule;
        return run_configured_role(
            conn,
            &continued,
            provider,
            model,
            endpoint_url,
            timeout_seconds,
        );
    }
    serde_json::to_value(operation).map_err(|error| error.to_string())
}

fn extract_role_json(content: &str) -> Option<Value> {
    let trimmed = content.trim();
    if let Ok(value) = serde_json::from_str(trimmed) {
        return Some(value);
    }
    let unfenced = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|value| value.strip_suffix("```"));
    if let Some(value) = unfenced.and_then(|value| serde_json::from_str(value.trim()).ok()) {
        return Some(value);
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end <= start {
        return None;
    }
    serde_json::from_str(&trimmed[start..=end]).ok()
}

fn role_output_shape_valid(content: &str, role: Role, constructing: bool) -> bool {
    let Some(value) = extract_role_json(content) else {
        return false;
    };
    let kind = value
        .get("artifactType")
        .or_else(|| value.get("payload").and_then(|p| p.get("type")))
        .or_else(|| value.get("artifact").and_then(|p| p.get("type")))
        .or_else(|| value.get("type"))
        .and_then(Value::as_str);
    match role {
        Role::Dreamer => kind.is_none() || kind == Some("dream_contract"),
        Role::Skeptic => matches!(kind, Some("finding" | "review_verdict")),
        Role::Fde => kind == Some("fde_brief"),
        Role::UxDesigner => kind == Some("ux_contract"),
        Role::Architect => matches!(kind, Some("architecture_alternatives" | "adr")),
        Role::Planner => kind == Some("task_graph"),
        Role::Builder => {
            if constructing {
                matches!(kind, Some("patch_proposal" | "propose_patch"))
            } else {
                kind == Some("finding")
            }
        }
        Role::Verifier => kind == Some("verification_verdict"),
        Role::Reviewer => kind == Some("review_verdict"),
        _ => false,
    }
}

fn role_repair_schema(role: Role) -> &'static str {
    match role {
        Role::Dreamer => {
            r#"{"type":"dream_contract","title":"App name","horizon":"strategic","problemObserved":"problem","proposedFuture":"solution","supportingEvidence":[],"expectedValue":["value"],"counterarguments":["risk"],"assumptions":["assumption"],"smallestExperiment":"offline web app","estimatedCost":"bounded","reversibility":"high","noveltyRationale":"novelty","confidence":0.6}"#
        }
        Role::Skeptic => {
            r#"{"type":"finding","severity":"medium","blocking":false,"summary":"challenge","recommendation":"proceed"}"#
        }
        Role::Fde => {
            r#"{"type":"fde_brief","targetUser":"user","objective":"outcome","assumptions":["assumption"],"constraints":["offline"],"nonGoals":["cloud"],"successCriteria":["journey works"],"verdict":"GO"}"#
        }
        Role::UxDesigner => {
            r#"{"type":"ux_contract","persona":"user","userJourney":["open","act","confirm"],"states":["empty","active","error","recovery"],"accessibility":["keyboard","visible focus"],"acceptanceCriteria":["primary journey works"]}"#
        }
        Role::Architect => {
            r#"{"type":"architecture_alternatives","options":[{"id":"A","design":"modular offline web app","reversibility":"high"},{"id":"B","design":"single-page modules","reversibility":"medium"}],"selected":"A","moduleStructure":["index.html","styles.css","app.js"],"persistenceModel":"localStorage","securityModel":"offline"}"#
        }
        Role::Planner => {
            r#"{"type":"task_graph","tasks":[{"id":"TASK-1","title":"Core journey","dependencies":[],"allowedPaths":["app.js"],"expectedFiles":["app.js"]},{"id":"TASK-2","title":"Visual system","dependencies":["TASK-1"],"allowedPaths":["styles.css"],"expectedFiles":["styles.css"]},{"id":"TASK-3","title":"Accessible shell","dependencies":["TASK-2"],"allowedPaths":["index.html"],"expectedFiles":["index.html"]}],"validation":["offline_web_v1 structural validation","offline_web_v1 launch check"]}"#
        }
        Role::Builder => {
            r#"{"type":"finding","blocking":false,"summary":"bounded tasks are buildable","verdict":"BUILDABLE"}"#
        }
        Role::Verifier => {
            r#"{"type":"verification_verdict","verdict":"PASS","missingEvidence":[],"testabilityNotes":["structural validation","launch check"]}"#
        }
        Role::Reviewer => r#"{"type":"review_verdict","verdict":"PASS","findings":[]}"#,
        _ => r#"{"type":"finding","blocking":false,"summary":"stage complete"}"#,
    }
}

fn normalize_dream_contract_payload(mut payload: Value) -> Value {
    let first_string = |value: &Value, key: &str| {
        value
            .get(key)
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    if payload.get("title").and_then(Value::as_str).is_none() {
        payload["title"] = json!(payload
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| first_string(&payload, "appNameCandidates"))
            .unwrap_or_else(|| "Untitled Dream Application".into()));
    }
    let mappings = [
        ("problemObserved", &["coreProblem", "problem"] as &[&str]),
        (
            "proposedFuture",
            &["uniqueValueProposition", "proposed_future"],
        ),
        (
            "smallestExperiment",
            &["smallestCoherentProduct", "smallest_experiment"],
        ),
        (
            "noveltyRationale",
            &["noveltyRationale", "novelty_rationale"],
        ),
    ];
    for (target, sources) in mappings {
        if payload.get(target).and_then(Value::as_str).is_none() {
            payload[target] = json!(sources
                .iter()
                .find_map(|source| payload.get(*source).and_then(Value::as_str))
                .unwrap_or("Requires Product Council clarification."));
        }
    }
    if payload.get("horizon").is_none() {
        payload["horizon"] = json!("strategic");
    }
    if payload.get("supportingEvidence").is_none() {
        payload["supportingEvidence"] = json!([]);
    }
    if payload.get("expectedValue").is_none() {
        payload["expectedValue"] = json!([payload
            .get("uniqueValueProposition")
            .and_then(Value::as_str)
            .unwrap_or("Useful offline workflow")]);
    }
    if payload.get("counterarguments").is_none() {
        payload["counterarguments"] = payload
            .get("risks")
            .cloned()
            .unwrap_or_else(|| json!(["Product Council validation required"]));
    }
    if payload.get("assumptions").is_none() {
        payload["assumptions"] = json!(["Target users value the proposed workflow"]);
    }
    if payload.get("estimatedCost").is_none() {
        payload["estimatedCost"] = json!("bounded");
    }
    if payload.get("reversibility").is_none() {
        payload["reversibility"] = json!("high");
    }
    if payload.get("confidence").is_none() {
        payload["confidence"] = json!(0.5);
    }
    payload["type"] = json!("dream_contract");
    payload
}

fn record_context_generation_failure(
    conn: &rusqlite::Connection,
    prepared: &orchestration_core::PreparedRoleRun,
    error: &str,
) -> Result<(), String> {
    if prepared.context_capsule.omitted_artifacts.is_empty() {
        return Ok(());
    }
    Ledger::new(conn)
        .record_event(OrchestrationEvent {
            id: new_id("EVENT"),
            initiative_id: prepared.context_capsule.initiative_id.clone(),
            task_id: prepared.context_capsule.task_id.clone(),
            actor_role: Role::System,
            kind: "context.generation_failed_after_omission".into(),
            requirement_ids: vec![],
            adr_ids: prepared
                .context_capsule
                .active_adr_versions
                .keys()
                .cloned()
                .collect(),
            assumption_ids: vec![],
            features: BTreeMap::from([
                ("context_pressure".into(), 1.0),
                (
                    "omitted_artifact_count".into(),
                    prepared.context_capsule.omitted_artifacts.len() as f64,
                ),
            ]),
            provenance: "context-os-v1".into(),
            redacted_summary: format!(
                "Configured role generation failed after bounded-context omission: {}",
                error.chars().take(240).collect::<String>()
            ),
            created_at: None,
        })
        .map(|_| ())
        .map_err(|record_error| record_error.to_string())
}

fn strict_parse_studio_operation(content: &str) -> Result<AgentOperation, String> {
    let normalized = content.trim().replace("\r\n", "\n");
    let json = if normalized.starts_with('{') && normalized.ends_with('}') {
        normalized.as_str()
    } else if normalized.starts_with("```json\n") && normalized.ends_with("\n```") {
        &normalized[8..normalized.len() - 4]
    } else {
        return Err("role output must be exactly one JSON object or one fenced json object".into());
    };
    let operation: AgentOperation = serde_json::from_str(json)
        .map_err(|error| format!("invalid typed role operation: {error}"))?;
    if matches!(
        operation,
        AgentOperation::ReadFile { .. }
            | AgentOperation::SearchRepo { .. }
            | AgentOperation::ProposePatch { .. }
            | AgentOperation::RunCommand(_)
            | AgentOperation::AskUser { .. }
            | AgentOperation::FinalReport { .. }
            | AgentOperation::HandOff { .. }
    ) {
        return Err("Assist operation is not valid in a configured Studio role run".into());
    }
    Ok(operation)
}

fn validate_operation_header(
    prepared: &orchestration_core::PreparedRoleRun,
    header: &StudioOperationHeader,
) -> Result<Role, String> {
    let role = Role::try_from(header.role.as_str()).map_err(|error| error.to_string())?;
    if role != prepared.role
        || header.initiative_id != prepared.context_capsule.initiative_id
        || header.task_id != prepared.context_capsule.task_id
        || header.spec_version != prepared.context_capsule.active_spec_version
        || header.source_context_bundle_id.as_deref() != Some(prepared.context_capsule.id.as_str())
    {
        return Err(
            "role operation header does not match its prepared role/context/task/spec binding"
                .into(),
        );
    }
    if header.operation_id.trim().is_empty()
        || header.reason.trim().is_empty()
        || header.expected_outcome.trim().is_empty()
        || header.schema_version != 1
    {
        return Err("role operation header is incomplete or uses an unsupported schema".into());
    }
    Ok(role)
}

fn apply_studio_operation(
    conn: &rusqlite::Connection,
    prepared: &orchestration_core::PreparedRoleRun,
    operation: &AgentOperation,
) -> Result<Option<context_os::ContextCapsule>, String> {
    let ledger = Ledger::new(conn);
    if let Err(error) = validate_capsule_freshness(conn, &prepared.context_capsule) {
        ledger
            .record_event(OrchestrationEvent {
                id: new_id("EVENT"),
                initiative_id: prepared.context_capsule.initiative_id.clone(),
                task_id: prepared.context_capsule.task_id.clone(),
                actor_role: Role::System,
                kind: if error.to_string().contains("ADR") {
                    "context.stale_adr".into()
                } else if error.to_string().contains("spec") {
                    "context.stale_spec".into()
                } else {
                    "context.binding_invalid".into()
                },
                requirement_ids: vec![],
                adr_ids: prepared.context_capsule.active_adr_versions.keys().cloned().collect(),
                assumption_ids: vec![],
                features: BTreeMap::from([("context_drift".into(), 1.0), ("severity".into(), 0.95)]),
                provenance: "context-os-v1".into(),
                redacted_summary: "Role output rejected because its Context Capsule became stale during inference.".into(),
                created_at: None,
            })
            .map_err(|record_error| record_error.to_string())?;
        return Err(error.to_string());
    }
    match operation {
        AgentOperation::ProposeArtifact { header, payload } => {
            let role = validate_operation_header(prepared, header)?;
            if header.artifact_type == "patch_proposal" {
                if role != Role::Builder || header.task_id.is_none() {
                    return Err("patch_proposal requires a task-bound Builder run".into());
                }
                let operation_value = payload
                    .get("operation")
                    .cloned()
                    .unwrap_or_else(|| payload.clone());
                let patch_operation: AgentOperation = serde_json::from_value(operation_value)
                    .map_err(|error| {
                        format!(
                            "patch_proposal payload must contain one typed propose_patch operation: {error}"
                        )
                    })?;
                if !matches!(patch_operation, AgentOperation::ProposePatch { .. }) {
                    return Err(
                        "patch_proposal payload must contain one typed propose_patch operation"
                            .into(),
                    );
                }
                validate_builder_patch_task_scope(
                    conn,
                    header.task_id.as_deref().expect("task checked above"),
                    &patch_operation,
                )?;
                let initiative = ledger
                    .get_initiative(&header.initiative_id)
                    .map_err(|error| error.to_string())?;
                let application_workspace: Option<String> = conn.query_row("SELECT workspace_path FROM dream_applications WHERE initiative_id=?1 AND status IN ('building','polishing')", [&header.initiative_id], |row| row.get(0)).optional().map_err(|error| error.to_string())?;
                if let Some(application_workspace) = application_workspace {
                    apply_builder_patch_to_application(
                        conn,
                        prepared,
                        header,
                        payload,
                        &patch_operation,
                        &application_workspace,
                    )?;
                    return Ok(None);
                }
                let initiative_repo = initiative.repo_root.clone();
                let tx = conn
                    .unchecked_transaction()
                    .map_err(|error| error.to_string())?;
                let validation = validate_patch_proposal_with_connection(
                    &tx,
                    PatchProposalRequest {
                        session_id: prepared.context_capsule.session_id.clone(),
                        repo_root: initiative_repo.clone(),
                        operation: patch_operation,
                        agent_profile_id: None,
                        context_bundle_id: prepared.context_capsule.id.clone(),
                    },
                    PathBuf::from(&initiative_repo),
                )?;
                if !validation.ok {
                    return Err(format!(
                        "Builder patch failed governed validation: {}",
                        validation.message
                    ));
                }
                Ledger::new(&tx)
                    .publish_artifact(
                        &ArtifactEnvelope {
                            operation_id: header.operation_id.clone(),
                            initiative_id: header.initiative_id.clone(),
                            task_id: header.task_id.clone(),
                            role,
                            artifact_type: header.artifact_type.clone(),
                            schema_version: header.schema_version,
                            spec_version: header.spec_version,
                            source_context_bundle_id: header.source_context_bundle_id.clone(),
                            reason: header.reason.clone(),
                            expected_outcome: header.expected_outcome.clone(),
                            payload: payload.clone(),
                        },
                        Some(&prepared.run_id),
                        "Builder patch artifact and governed diff validation accepted atomically.",
                    )
                    .map_err(|error| error.to_string())?;
                tx.commit().map_err(|error| error.to_string())?;
                return Ok(None);
            }
            ledger
                .publish_artifact(
                    &ArtifactEnvelope {
                        operation_id: header.operation_id.clone(),
                        initiative_id: header.initiative_id.clone(),
                        task_id: header.task_id.clone(),
                        role,
                        artifact_type: header.artifact_type.clone(),
                        schema_version: header.schema_version,
                        spec_version: header.spec_version,
                        source_context_bundle_id: header.source_context_bundle_id.clone(),
                        reason: header.reason.clone(),
                        expected_outcome: header.expected_outcome.clone(),
                        payload: payload.clone(),
                    },
                    Some(&prepared.run_id),
                    "Configured role artifact accepted.",
                )
                .map_err(|error| error.to_string())?;
            materialize_authoritative_artifact(conn, prepared, role, header, payload)?;
            if role == Role::Dreamer && header.artifact_type == "dream_contract" {
                ledger
                    .create_dream(&header.initiative_id, payload)
                    .map_err(|error| error.to_string())?;
            }
        }
        AgentOperation::PublishBelief { header, payload } => {
            let role = validate_operation_header(prepared, header)?;
            if header.artifact_type != "belief" {
                return Err("publish_belief requires artifactType=belief".into());
            }
            ledger
                .publish_belief(
                    &header.initiative_id,
                    header.task_id.as_deref(),
                    header.spec_version,
                    Some(&prepared.run_id),
                    role,
                    payload,
                )
                .map_err(|error| error.to_string())?;
        }
        AgentOperation::AskAgent {
            header,
            to_role,
            blocking,
            question,
        } => {
            let role = validate_operation_header(prepared, header)?;
            if header.artifact_type != "question" {
                return Err("ask_agent requires artifactType=question".into());
            }
            ledger
                .open_question(&intent_ledger::AlignmentQuestion {
                    id: header.operation_id.clone(),
                    initiative_id: header.initiative_id.clone(),
                    task_id: header.task_id.clone(),
                    from_role: role,
                    to_role: Role::try_from(to_role.as_str()).map_err(|error| error.to_string())?,
                    reason: header.reason.clone(),
                    question: question.clone(),
                    blocking: *blocking,
                    status: "open".into(),
                })
                .map_err(|error| error.to_string())?;
        }
        AgentOperation::AnswerAgent {
            header,
            question_id,
            answer,
            evidence,
        } => {
            let role = validate_operation_header(prepared, header)?;
            ledger
                .answer_question(
                    question_id,
                    role,
                    &json!({"answer":answer,"evidence":evidence,"operationId":header.operation_id}),
                )
                .map_err(|error| error.to_string())?;
        }
        AgentOperation::ReportFinding {
            header,
            severity,
            blocking,
            related_ids,
            summary,
        } => {
            let role = validate_operation_header(prepared, header)?;
            if header.artifact_type != "finding" {
                return Err("report_finding requires artifactType=finding".into());
            }
            ledger
                .publish_artifact(
                    &ArtifactEnvelope {
                        operation_id: header.operation_id.clone(),
                        initiative_id: header.initiative_id.clone(),
                        task_id: header.task_id.clone(),
                        role,
                        artifact_type: "finding".into(),
                        schema_version: 1,
                        spec_version: header.spec_version,
                        source_context_bundle_id: header.source_context_bundle_id.clone(),
                        reason: header.reason.clone(),
                        expected_outcome: header.expected_outcome.clone(),
                        payload: json!({"severity":severity,"blocking":blocking,"relatedIds":related_ids,"summary":summary}),
                    },
                    Some(&prepared.run_id),
                    summary,
                )
                .map_err(|error| error.to_string())?;
        }
        AgentOperation::RequestTransition {
            header,
            target_status,
        } => {
            let role = validate_operation_header(prepared, header)?;
            ledger
                .record_event(OrchestrationEvent {
                    id: header.operation_id.clone(),
                    initiative_id: header.initiative_id.clone(),
                    task_id: header.task_id.clone(),
                    actor_role: role,
                    kind: "transition.requested".into(),
                    requirement_ids: vec![],
                    adr_ids: vec![],
                    assumption_ids: vec![],
                    features: BTreeMap::new(),
                    provenance: prepared.run_id.clone(),
                    redacted_summary: format!(
                        "Role requested transition to {target_status}; no transition was applied."
                    ),
                    created_at: None,
                })
                .map_err(|error| error.to_string())?;
        }
        AgentOperation::RequestContext {
            header,
            categories,
            selectors,
            maximum_tokens,
        } => {
            let role = validate_operation_header(prepared, header)?;
            let request_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM context_requests r
                     JOIN context_capsules c ON c.id=r.source_capsule_id
                     WHERE c.agent_run_id=?1",
                    [&prepared.run_id],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            if request_count >= 4 {
                return Err(
                    "role exceeded four progressive context requests in one run; partition the task or ask the user to narrow scope"
                        .into(),
                );
            }
            let mut retrieval: Vec<RetrievalSelector> = selectors
                .iter()
                .map(|selector| {
                    serde_json::from_value(
                        serde_json::to_value(selector).map_err(|error| error.to_string())?,
                    )
                    .map_err(|error| error.to_string())
                })
                .collect::<Result<_, _>>()?;
            if retrieval.is_empty() {
                retrieval = categories
                    .iter()
                    .map(|category| {
                        let kind = match category.as_str() {
                            "files" | "file_excerpts" => RetrievalKind::FileExcerpt,
                            "symbols" => RetrievalKind::Symbol,
                            "definitions" => RetrievalKind::Definition,
                            "references" => RetrievalKind::Reference,
                            "dependencies" => RetrievalKind::DirectDependency,
                            "tests" => RetrievalKind::Test,
                            "requirements" => RetrievalKind::Requirement,
                            "adrs" => RetrievalKind::Adr,
                            "ux" | "ux_criteria" => RetrievalKind::UxCriterion,
                            "assumptions" => RetrievalKind::Assumption,
                            "evidence" => RetrievalKind::Evidence,
                            "findings" => RetrievalKind::PriorFinding,
                            "task_summaries" => RetrievalKind::TaskSummary,
                            unknown => {
                                return Err(format!("unknown legacy context category: {unknown}"))
                            }
                        };
                        Ok(RetrievalSelector {
                            kind,
                            query: None,
                            relative_path: None,
                            source_id: None,
                            maximum_items: 20,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?;
            }
            let context_request = ContextRequest {
                id: header.operation_id.clone(),
                initiative_id: header.initiative_id.clone(),
                task_id: header.task_id.clone(),
                role,
                source_capsule_id: prepared.context_capsule.id.clone(),
                selectors: retrieval.clone(),
                maximum_additional_tokens: *maximum_tokens,
                reason: header.reason.clone(),
            };
            if let Err(error) = context_request.validate(&projection_policy(role)) {
                record_context_request(conn, &context_request, None, Some(&error.to_string()))
                    .map_err(|record_error| record_error.to_string())?;
                return Err(error.to_string());
            }
            let initiative = ledger
                .get_initiative(&header.initiative_id)
                .map_err(|error| error.to_string())?;
            let protocol_prompt = prepared
                .context_capsule
                .included_artifacts
                .iter()
                .find(|item| item.source_type == "protocol")
                .and_then(|item| item.content.as_str())
                .ok_or_else(|| "source capsule has no P0 protocol".to_string())?;
            let compiled = ContextCompiler::new(conn).compile(CapsuleCompileRequest {
                session_id: &prepared.context_capsule.session_id,
                initiative_id: &header.initiative_id,
                task_id: header.task_id.as_deref(),
                role,
                agent_run_id: &prepared.run_id,
                runtime: &prepared.runtime,
                model: &prepared.model,
                protocol_prompt,
                reserved_output_tokens: Some(prepared.context_capsule.reserved_output_tokens),
                maximum_compiled_input_tokens: Some(
                    prepared
                        .context_capsule
                        .compiled_input_tokens
                        .checked_add(*maximum_tokens)
                        .ok_or_else(|| {
                            "context token request overflowed the numeric limit".to_string()
                        })?,
                ),
                retrieval,
                repo_root: Some(std::path::Path::new(&initiative.repo_root)),
            });
            let resulting_capsule = match compiled {
                Ok(capsule) => capsule,
                Err(error) => {
                    record_context_request(conn, &context_request, None, Some(&error.to_string()))
                        .map_err(|record_error| record_error.to_string())?;
                    return Err(error.to_string());
                }
            };
            record_context_request(conn, &context_request, Some(&resulting_capsule.id), None)
                .map_err(|error| error.to_string())?;
            conn.execute(
                "UPDATE agent_runs SET context_bundle_id=?1, token_estimate=?2 WHERE id=?3",
                params![
                    resulting_capsule.id,
                    resulting_capsule.compiled_input_tokens as i64,
                    prepared.run_id
                ],
            )
            .map_err(|error| error.to_string())?;
            ledger
                .record_event(OrchestrationEvent {
                    id: header.operation_id.clone(),
                    initiative_id: header.initiative_id.clone(),
                    task_id: header.task_id.clone(),
                    actor_role: role,
                    kind: "context.requested".into(),
                    requirement_ids: vec![],
                    adr_ids: vec![],
                    assumption_ids: vec![],
                    features: BTreeMap::from([
                        ("maximumTokens".into(), *maximum_tokens as f64),
                        ("context_pressure".into(), 0.3),
                    ]),
                    provenance: prepared.run_id.clone(),
                    redacted_summary: format!(
                        "Role requested additional categories: {}",
                        if categories.is_empty() {
                            "typed selectors".into()
                        } else {
                            categories.join(", ")
                        }
                    ),
                    created_at: None,
                })
                .map_err(|error| error.to_string())?;
            return Ok(Some(resulting_capsule));
        }
        AgentOperation::StudioFinalReport { header, payload } => {
            let role = validate_operation_header(prepared, header)?;
            ledger
                .publish_artifact(
                    &ArtifactEnvelope {
                        operation_id: header.operation_id.clone(),
                        initiative_id: header.initiative_id.clone(),
                        task_id: header.task_id.clone(),
                        role,
                        artifact_type: header.artifact_type.clone(),
                        schema_version: 1,
                        spec_version: header.spec_version,
                        source_context_bundle_id: header.source_context_bundle_id.clone(),
                        reason: header.reason.clone(),
                        expected_outcome: header.expected_outcome.clone(),
                        payload: payload.clone(),
                    },
                    Some(&prepared.run_id),
                    "Configured role final report accepted.",
                )
                .map_err(|error| error.to_string())?;
        }
        _ => return Err("operation is not permitted in Studio role execution".into()),
    }
    Ok(None)
}

/// Converts role artifacts that advance delivery into ledger records. Generic
/// publication remains the immutable provenance record, but it is never the
/// sole basis for a factory stage transition.
fn materialize_authoritative_artifact(
    conn: &rusqlite::Connection,
    prepared: &orchestration_core::PreparedRoleRun,
    role: Role,
    header: &StudioOperationHeader,
    payload: &Value,
) -> Result<(), String> {
    let ledger = Ledger::new(conn);
    match (role, header.artifact_type.as_str()) {
        (Role::Planner, "task_graph") => {
            let tasks = payload
                .get("tasks")
                .and_then(Value::as_array)
                .ok_or_else(|| "Planner task_graph requires a tasks array".to_string())?;
            if tasks.is_empty() {
                return Err("Planner task_graph requires at least one task".into());
            }
            ledger.freeze_spec(&header.initiative_id,header.spec_version,&json!({"sourceArtifact":header.operation_id,"implementationSpec":payload,"applicationKind":"offline_web_v1"})).map_err(|error|error.to_string())?;
            let default_requirements: Vec<String> = conn
                .prepare("SELECT id FROM requirements WHERE initiative_id=?1 AND spec_version=?2 ORDER BY id")
                .map_err(|error| error.to_string())?
                .query_map(params![header.initiative_id, header.spec_version], |row| row.get(0))
                .map_err(|error| error.to_string())?
                .collect::<std::result::Result<Vec<String>, _>>()
                .map_err(|error| error.to_string())?;
            if default_requirements.is_empty() {
                return Err(
                    "Planner cannot materialize tasks without authoritative requirements".into(),
                );
            }
            let default_adr: Vec<String> = conn
                .prepare("SELECT id FROM architecture_decisions WHERE initiative_id=?1 AND spec_version=?2 AND status='approved' ORDER BY id")
                .map_err(|error| error.to_string())?
                .query_map(params![header.initiative_id, header.spec_version], |row| row.get(0))
                .map_err(|error| error.to_string())?
                .collect::<std::result::Result<Vec<String>, _>>()
                .map_err(|error| error.to_string())?;
            let default_ux: Vec<String> = conn
                .prepare("SELECT id FROM ux_contracts WHERE initiative_id=?1 AND spec_version=?2 AND status IN ('approved','accepted') ORDER BY id")
                .map_err(|error| error.to_string())?
                .query_map(params![header.initiative_id, header.spec_version], |row| row.get(0))
                .map_err(|error| error.to_string())?
                .collect::<std::result::Result<Vec<String>, _>>()
                .map_err(|error| error.to_string())?;
            let mut prior_id: Option<String> = None;
            for (index, item) in tasks.iter().enumerate() {
                let object = item.as_object();
                let title = object
                    .and_then(|value| value.get("title").and_then(Value::as_str))
                    .or_else(|| item.as_str())
                    .unwrap_or("planned task");
                let task_id = object
                    .and_then(|value| value.get("id").and_then(Value::as_str))
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("TASK-{}-{}", header.operation_id, index + 1));
                let allowed_paths = object
                    .and_then(|value| value.get("allowedPaths").and_then(Value::as_array))
                    .cloned()
                    .unwrap_or_else(|| {
                        vec![
                            Value::String("index.html".into()),
                            Value::String("styles.css".into()),
                            Value::String("app.js".into()),
                            Value::String("README.md".into()),
                        ]
                    });
                let expected_files = object
                    .and_then(|value| value.get("expectedFiles").and_then(Value::as_array))
                    .cloned()
                    .unwrap_or_else(|| {
                        vec![Value::String(
                            match index {
                                0 => "app.js",
                                1 => "styles.css",
                                _ => "index.html",
                            }
                            .into(),
                        )]
                    });
                let dependencies: Vec<String> = object
                    .and_then(|value| value.get("dependencies").and_then(Value::as_array))
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_else(|| prior_id.iter().cloned().collect());
                let task = StudioTask {
                    id: task_id.clone(),
                    initiative_id: header.initiative_id.clone(),
                    spec_version: header.spec_version,
                    status: TaskStatus::Ready,
                    assigned_role: Role::Builder,
                    iteration_count: 0,
                    max_iterations: 3,
                    payload: json!({"title":title,"requirementIds":default_requirements,"adrIds":default_adr,"uxAcceptanceIds":default_ux,"dependencies":dependencies,"allowedPaths":allowed_paths,"expectedFiles":expected_files,"forbiddenPaths":[".git/config"],"validationCommands":payload.get("validation").cloned().unwrap_or_else(||json!([["cargo","test","--workspace"]]))}),
                };
                ledger.add_task(&task).map_err(|error| error.to_string())?;
                for dependency in task
                    .payload
                    .get("dependencies")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                {
                    conn.execute("INSERT OR IGNORE INTO dream_task_dependencies (task_id, depends_on_task_id) VALUES (?1,?2)", params![task_id, dependency]).map_err(|error| error.to_string())?;
                }
                prior_id = Some(task_id);
            }
        }
        (Role::Fde, "fde_brief") => {
            if let Some(objective) = payload.get("objective").and_then(Value::as_str) {
                conn.execute("INSERT INTO objectives (id,initiative_id,spec_version,status,payload_json) VALUES (?1,?2,?3,'approved',?4)", params![new_id("OBJ"),header.initiative_id,header.spec_version,json!({"businessObjective":objective,"sourceArtifact":header.operation_id}).to_string()]).map_err(|error| error.to_string())?;
            }
        }
        (Role::UxDesigner, "ux_contract") => {
            let prototype = payload.get("prototype").cloned().unwrap_or_else(|| json!({
                "schemaVersion":1,"title":"Dream candidate prototype","initialState":{},
                "root":{"type":"card","id":"dream-root","title":"Dream candidate","children":[{"type":"text","id":"dream-copy","text":"Review the proposed user journey."}]}
            }));
            let document: PrototypeDocument = serde_json::from_value(prototype.clone())
                .map_err(|error| format!("UX contract prototype is invalid: {error}"))?;
            validate_prototype(&document).map_err(|error| error.to_string())?;
            conn.execute("INSERT INTO ux_contracts (id,initiative_id,spec_version,status,contract_json,prototype_json) VALUES (?1,?2,?3,'approved',?4,?5)", params![new_id("UX"),header.initiative_id,header.spec_version,payload.to_string(),prototype.to_string()]).map_err(|error| error.to_string())?;
        }
        (Role::Architect, "architecture_alternatives" | "adr") => {
            let selected = payload
                .get("selected")
                .and_then(Value::as_str)
                .or_else(|| payload.get("selectedOption").and_then(Value::as_str))
                .or_else(|| payload.get("decision").and_then(Value::as_str))
                .or_else(|| payload.get("recommendation").and_then(Value::as_str))
                .or_else(|| payload.get("preferredOption").and_then(Value::as_str))
                .or_else(|| payload.get("chosenOption").and_then(Value::as_str))
                .or_else(|| payload.get("selectedArchitecture").and_then(Value::as_str))
                .or_else(|| {
                    payload
                        .get("adr")
                        .and_then(|adr| adr.get("decision"))
                        .and_then(Value::as_str)
                })
                .or_else(|| {
                    payload
                        .get("options")
                        .and_then(Value::as_array)
                        .and_then(|options| options.first())
                        .and_then(|option| option.get("id"))
                        .and_then(Value::as_str)
                })
                .ok_or_else(|| {
                    format!("architecture output requires a selected option or decision: {payload}")
                })?;
            conn.execute("INSERT INTO architecture_decisions (id,initiative_id,spec_version,status,payload_json) VALUES (?1,?2,?3,'approved',?4)", params![new_id("ADR"),header.initiative_id,header.spec_version,json!({"selected":selected,"alternatives":payload.get("options").cloned().unwrap_or_else(||json!([])),"sourceArtifact":header.operation_id}).to_string()]).map_err(|error| error.to_string())?;
        }
        _ => {}
    }
    let _ = prepared;
    Ok(())
}

/// The trusted, backend-owned implementation slice used by Dream Factory.  It
/// never touches the user's active checkout: a task can only write beneath its
/// bound application workspace and explicitly approved task path. Fake Runtime
/// deliberately introduces a forbidden remote dependency so deterministic
/// validation produces a concrete repair. Production uses configured role
/// invocations and never relies on fixture text as evidence.
fn factory_build_and_review(
    conn: &rusqlite::Connection,
    factory_id: &str,
    initiative_id: &str,
    task_id: &str,
) -> Result<(), String> {
    let ledger = Ledger::new(conn);
    let task = ledger
        .get_task(task_id)
        .map_err(|error| error.to_string())?;
    if task.initiative_id != initiative_id || task.status != TaskStatus::InProgress {
        return Err("factory Builder requires the controller-selected in-progress task".into());
    }
    let builder_config = factory_role_runtime(conn, initiative_id, Role::Builder)?;
    if builder_config.0 != "fake" {
        return factory_real_build_and_review(
            conn,
            factory_id,
            initiative_id,
            task_id,
            &task,
            builder_config,
        );
    }
    let worktree: String = conn
        .query_row(
            "SELECT workspace_path FROM dream_applications WHERE initiative_id=?1 AND status IN ('building','polishing') UNION ALL SELECT worktree_path FROM governed_worktrees WHERE initiative_id=?1 AND status='active' LIMIT 1",
            [initiative_id], |row| row.get(0),
        )
        .optional().map_err(|error| error.to_string())?
        .ok_or_else(|| "factory task has no active governed worktree".to_string())?;
    let allowed: Vec<String> = task
        .payload
        .get("allowedPaths")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    if allowed.is_empty() {
        return Err("task has no mandate-approved write path".into());
    }
    let expected = task
        .payload
        .get("expectedFiles")
        .and_then(Value::as_array)
        .and_then(|values| values.first())
        .and_then(Value::as_str)
        .ok_or_else(|| "task has no expected output file".to_string())?;
    let normalized = expected.replace('\\', "/");
    if !allowed.iter().any(|allowed| {
        normalized == *allowed
            || normalized.starts_with(&format!("{}/", allowed.trim_end_matches('/')))
    }) || normalized.starts_with('.')
        || normalized.contains("..")
        || normalized.contains(".git")
    {
        return Err("task output path violates mandate-bound worktree policy".into());
    }
    let target = PathBuf::from(&worktree).join(&normalized);
    let before = fs::read_to_string(&target).unwrap_or_default();
    let after = if task.iteration_count == 0 && normalized == "app.js" {
        format!("{before}\n// deterministic fixture: validation must reject remote behavior\nfetch('https://invalid.local');\n")
    } else if task.iteration_count > 0 && normalized == "app.js" {
        before
            .lines()
            .filter(|line| {
                !line.contains("deterministic fixture")
                    && !line.contains("fetch('https://invalid.local')")
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\nlocalStorage.setItem('focus-garden-ready','true');\n"
    } else {
        format!("{before}\n/* {task_id}: bounded application refinement */\n")
    };
    let old_lines = before.lines().count();
    let new_lines = after.lines().count();
    let old_body = before
        .lines()
        .map(|line| format!("-{line}\n"))
        .collect::<String>();
    let new_body = after
        .lines()
        .map(|line| format!("+{line}\n"))
        .collect::<String>();
    let diff = format!(
        "diff --git a/{0} b/{0}\n--- a/{0}\n+++ b/{0}\n@@ -1,{1} +1,{2} @@\n{3}{4}",
        normalized, old_lines, new_lines, old_body, new_body
    );
    let mut digest = Sha256::new();
    digest.update(before.as_bytes());
    let proposal = EnginePatchProposal {
        id: new_id("PATCH"),
        base_commit: None,
        current_commit: None,
        files: vec![EnginePatchFile {
            id: new_id("PATCHFILE"),
            path: normalized.clone(),
            before_sha256: hex::encode(digest.finalize()),
            unified_diff: diff,
        }],
    };
    let applied = apply_patch_proposal_transactional(&PathBuf::from(&worktree), &proposal, None)
        .map_err(|error| format!("mandate-bound worktree patch rejected: {error}"))?;
    ledger
        .record_event(OrchestrationEvent {
            id: new_id("EVENT"),
            initiative_id: initiative_id.into(),
            task_id: Some(task_id.into()),
            actor_role: Role::System,
            kind: "factory.patch_auto_approved".into(),
            requirement_ids: task
                .payload
                .get("requirementIds")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
            adr_ids: vec![],
            assumption_ids: vec![],
            features: BTreeMap::new(),
            provenance: "mandate-bound-worktree-policy".into(),
            redacted_summary: format!(
                "Applied {} only in governed worktree; checkpoint {}.",
                normalized, applied.checkpoint_id
            ),
            created_at: None,
        })
        .map_err(|error| error.to_string())?;
    let passed = ApplicationWorkspaceManager::new(conn)
        .validate_offline_web_app(&PathBuf::from(&worktree))
        .is_ok();
    for requirement_id in task
        .payload
        .get("requirementIds")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        ledger
            .record_evidence(
                initiative_id,
                &intent_ledger::EvidenceInput {
                    requirement_id: requirement_id.into(),
                    task_id: Some(task_id.into()),
                    evidence_type: "governed_file_check".into(),
                    status: if passed { "passed" } else { "failed" }.into(),
                    provenance: "factory-deterministic-validation".into(),
                    output_ref: Some(target.display().to_string()),
                    summary: if passed {
                        "Offline application structural and resource validation passed.".into()
                    } else {
                        "Offline application validation failed; precise repair required.".into()
                    },
                    content_sha256: None,
                },
            )
            .map_err(|error| error.to_string())?;
    }
    conn.execute("UPDATE studio_tasks SET status='reviewing', updated_at=datetime('now') WHERE id=?1 AND status='in_progress'", [task_id]).map_err(|error| error.to_string())?;
    ledger
        .record_event(OrchestrationEvent {
            id: new_id("EVENT"),
            initiative_id: initiative_id.into(),
            task_id: Some(task_id.into()),
            actor_role: Role::Verifier,
            kind: if passed {
                "factory.validation.passed"
            } else {
                "factory.validation.failed"
            }
            .into(),
            requirement_ids: vec![],
            adr_ids: vec![],
            assumption_ids: vec![],
            features: BTreeMap::new(),
            provenance: "factory-deterministic-validation".into(),
            redacted_summary: if passed {
                "Verifier accepted deterministic validation.".into()
            } else {
                "Verifier identified a concrete offline application validation failure.".into()
            },
            created_at: None,
        })
        .map_err(|error| error.to_string())?;
    let verdict = if passed {
        intent_ledger::ReviewerVerdict::Pass
    } else {
        intent_ledger::ReviewerVerdict::Revise
    };
    ledger
        .route_review_verdict(task_id, verdict)
        .map_err(|error| error.to_string())?;
    let controller = DreamFactoryController::new(conn);
    controller
        .set_stage(
            factory_id,
            dream_factory::FactoryStage::TaskBuildPending,
            if passed {
                dream_factory::FactoryStage::TaskSelection
            } else {
                dream_factory::FactoryStage::TaskRevising
            },
            Some(if passed {
                "ready_task"
            } else {
                "targeted_builder_repair"
            }),
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn factory_role_runtime(
    conn: &rusqlite::Connection,
    initiative_id: &str,
    role: Role,
) -> Result<(String, String, Option<String>, i64), String> {
    let session: String = conn
        .query_row(
            "SELECT session_id FROM initiatives WHERE id=?1",
            [initiative_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let configured=conn.query_row("SELECT runtime,model,endpoint_url,timeout_seconds FROM role_runtime_configs WHERE session_id=?1 AND role=?2",params![session,role.as_str()],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).optional().map_err(|error|error.to_string())?;
    if let Some(configured) = configured {
        return Ok(configured);
    }
    Ok(conn.query_row("SELECT runtime,model,endpoint_url,timeout_seconds FROM role_runtime_configs WHERE session_id=?1 AND role='dreamer'",[session],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).optional().map_err(|error|error.to_string())?.unwrap_or_else(||("fake".into(),"studio-fixture-v1".into(),None,300)))
}

fn invoke_factory_task_role(
    conn: &rusqlite::Connection,
    initiative_id: &str,
    task_id: &str,
    role: Role,
    config: (String, String, Option<String>, i64),
) -> Result<Value, String> {
    let prepared = prepare_role_run(
        conn,
        initiative_id,
        Some(task_id),
        role,
        &config.0,
        &config.1,
    )
    .map_err(|error| error.to_string())?;
    let lease = studio_role_scheduler()
        .acquire(&prepared.run_id)
        .map_err(|error| error.to_string())?;
    let result = run_configured_role(
        conn,
        &prepared,
        &config.0,
        &config.1,
        config.2.as_deref(),
        config.3,
    );
    drop(lease);
    match result {
        Ok(value) => {
            complete_role_run(
                conn,
                &prepared,
                "completed",
                "factory_stage_contract_satisfied",
                None,
            )
            .map_err(|error| error.to_string())?;
            Ok(value)
        }
        Err(error) => {
            complete_role_run(
                conn,
                &prepared,
                "failed",
                "factory_role_error",
                Some(&error),
            )
            .map_err(|completion| completion.to_string())?;
            Err(error)
        }
    }
}

fn factory_real_build_and_review(
    conn: &rusqlite::Connection,
    factory_id: &str,
    initiative_id: &str,
    task_id: &str,
    task: &StudioTask,
    builder_config: (String, String, Option<String>, i64),
) -> Result<(), String> {
    invoke_factory_task_role(conn, initiative_id, task_id, Role::Builder, builder_config)?;
    let workspace:String=conn.query_row("SELECT workspace_path FROM dream_applications WHERE initiative_id=?1 AND status IN ('building','polishing')",[initiative_id],|row|row.get(0)).map_err(|error|error.to_string())?;
    let validation =
        ApplicationWorkspaceManager::new(conn).validate_offline_web_app(&PathBuf::from(&workspace));
    let passed_validation = validation.is_ok();
    Ledger::new(conn)
        .record_event(OrchestrationEvent {
            id: new_id("EVENT"),
            initiative_id: initiative_id.into(),
            task_id: Some(task_id.into()),
            actor_role: Role::System,
            kind: if passed_validation {
                "foundry.validation.passed"
            } else {
                "foundry.validation.failed"
            }
            .into(),
            requirement_ids: task
                .payload
                .get("requirementIds")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
            adr_ids: vec![],
            assumption_ids: vec![],
            features: BTreeMap::new(),
            provenance: "offline_web_v1-validator".into(),
            redacted_summary: validation
                .err()
                .map(|error| error.to_string())
                .unwrap_or_else(|| {
                    "Offline scaffold, assets, entry point, and remote-resource policy passed."
                        .into()
                }),
            created_at: None,
        })
        .map_err(|error| error.to_string())?;
    conn.execute("UPDATE studio_tasks SET status='reviewing',updated_at=datetime('now') WHERE id=?1 AND status='in_progress'",[task_id]).map_err(|error|error.to_string())?;
    invoke_factory_task_role(
        conn,
        initiative_id,
        task_id,
        Role::Verifier,
        factory_role_runtime(conn, initiative_id, Role::Verifier)?,
    )?;
    invoke_factory_task_role(
        conn,
        initiative_id,
        task_id,
        Role::Reviewer,
        factory_role_runtime(conn, initiative_id, Role::Reviewer)?,
    )?;
    let reviewer_payload:String=conn.query_row("SELECT payload_json FROM artifacts WHERE initiative_id=?1 AND task_id=?2 AND role='reviewer' AND artifact_type='review_verdict' ORDER BY rowid DESC LIMIT 1",params![initiative_id,task_id],|row|row.get(0)).map_err(|error|error.to_string())?;
    let reviewer: Value =
        serde_json::from_str(&reviewer_payload).map_err(|error| error.to_string())?;
    let verdict = reviewer
        .get("verdict")
        .and_then(Value::as_str)
        .unwrap_or("BLOCKED");
    let routed = if !passed_validation || verdict == "REVISE" {
        intent_ledger::ReviewerVerdict::Revise
    } else if verdict == "PASS" {
        intent_ledger::ReviewerVerdict::Pass
    } else if verdict == "REPLAN" {
        intent_ledger::ReviewerVerdict::Replan
    } else {
        intent_ledger::ReviewerVerdict::Blocked
    };
    Ledger::new(conn)
        .route_review_verdict(task_id, routed)
        .map_err(|error| error.to_string())?;
    let next = match routed {
        intent_ledger::ReviewerVerdict::Pass => dream_factory::FactoryStage::TaskSelection,
        intent_ledger::ReviewerVerdict::Revise => dream_factory::FactoryStage::TaskRevising,
        intent_ledger::ReviewerVerdict::Replan => dream_factory::FactoryStage::InitiativeReplanning,
        intent_ledger::ReviewerVerdict::Blocked => dream_factory::FactoryStage::Blocked,
    };
    DreamFactoryController::new(conn)
        .set_stage(
            factory_id,
            dream_factory::FactoryStage::TaskBuildPending,
            next,
            Some(if routed == intent_ledger::ReviewerVerdict::Pass {
                "ready_task"
            } else {
                "role_findings"
            }),
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn apply_builder_patch_to_application(
    conn: &rusqlite::Connection,
    prepared: &orchestration_core::PreparedRoleRun,
    header: &StudioOperationHeader,
    payload: &Value,
    operation: &AgentOperation,
    workspace: &str,
) -> Result<(), String> {
    let (proposal_id, base_commit, current_commit, files) = match operation {
        AgentOperation::ProposePatch {
            proposal_id,
            base_commit,
            current_commit,
            files,
            ..
        } => (proposal_id, base_commit, current_commit, files),
        _ => return Err("application Builder requires a typed propose_patch operation".into()),
    };
    let proposal = EnginePatchProposal {
        id: proposal_id.clone(),
        base_commit: base_commit.clone(),
        current_commit: current_commit.clone(),
        files: files
            .iter()
            .map(|file| EnginePatchFile {
                id: file.id.clone(),
                path: file.path.clone(),
                before_sha256: file.before_sha256.clone(),
                unified_diff: file.patch.clone(),
            })
            .collect(),
    };
    let applied = apply_patch_proposal_transactional(&PathBuf::from(workspace), &proposal, None)
        .map_err(|error| {
            format!("MandateBoundApplicationWorkspaceApproval rejected patch: {error}")
        })?;
    let ledger = Ledger::new(conn);
    ledger
        .publish_artifact(
            &ArtifactEnvelope {
                operation_id: header.operation_id.clone(),
                initiative_id: header.initiative_id.clone(),
                task_id: header.task_id.clone(),
                role: Role::Builder,
                artifact_type: "patch_proposal".into(),
                schema_version: header.schema_version,
                spec_version: header.spec_version,
                source_context_bundle_id: header.source_context_bundle_id.clone(),
                reason: header.reason.clone(),
                expected_outcome: header.expected_outcome.clone(),
                payload: payload.clone(),
            },
            Some(&prepared.run_id),
            "Configured Builder patch applied to the bound standalone application workspace.",
        )
        .map_err(|error| error.to_string())?;
    ledger
        .record_event(OrchestrationEvent {
            id: new_id("EVENT"),
            initiative_id: header.initiative_id.clone(),
            task_id: header.task_id.clone(),
            actor_role: Role::System,
            kind: "foundry.application_patch.applied".into(),
            requirement_ids: vec![],
            adr_ids: vec![],
            assumption_ids: vec![],
            features: BTreeMap::from([(
                "changed_files".into(),
                applied.applied_files.len() as f64,
            )]),
            provenance: "MandateBoundApplicationWorkspaceApproval".into(),
            redacted_summary: format!(
                "Applied {} Builder file(s) in application workspace; rollback checkpoint {}.",
                applied.applied_files.len(),
                applied.checkpoint_id
            ),
            created_at: None,
        })
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn validate_builder_patch_task_scope(
    conn: &rusqlite::Connection,
    task_id: &str,
    operation: &AgentOperation,
) -> Result<(), String> {
    let payload: String = conn
        .query_row(
            "SELECT payload_json FROM studio_tasks WHERE id=?1",
            [task_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Builder patch task no longer exists".to_string())?;
    let payload: Value = serde_json::from_str(&payload).map_err(|error| error.to_string())?;
    let allowed: Vec<String> = payload
        .get("allowedPaths")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|path| path.replace('\\', "/").trim_end_matches('/').to_string())
        .collect();
    if allowed.is_empty() {
        return Err("Builder task has no backend-approved allowedPaths".into());
    }
    let files = match operation {
        AgentOperation::ProposePatch { files, .. } => files,
        _ => return Err("Builder task scope validation requires propose_patch".into()),
    };
    for file in files {
        let path = file.path.replace('\\', "/");
        if !allowed
            .iter()
            .any(|root| path == *root || path.starts_with(&format!("{root}/")))
        {
            return Err(format!(
                "Builder patch path {} is outside task allowedPaths",
                file.path
            ));
        }
    }
    Ok(())
}

fn parse_fake_scenario(value: &str) -> Result<FakeScenario, String> {
    match value {
        "successful_studio" => Ok(FakeScenario::SuccessfulStudio),
        "reviewer_revision" => Ok(FakeScenario::ReviewerRevision),
        "replan" => Ok(FakeScenario::Replan),
        "blocking_question" => Ok(FakeScenario::BlockingQuestion),
        "dream_rejection" => Ok(FakeScenario::DreamRejection),
        "prototype_promotion" => Ok(FakeScenario::PrototypePromotion),
        "drift_signal" => Ok(FakeScenario::DriftSignal),
        "budget_stop" => Ok(FakeScenario::BudgetStop),
        "malformed_artifact" => Ok(FakeScenario::MalformedArtifact),
        "role_permission_violation" => Ok(FakeScenario::RolePermissionViolation),
        other => Err(format!("unknown Fake Runtime scenario: {other}")),
    }
}

fn canonical(repo_root: &str) -> std::result::Result<String, intent_ledger::LedgerError> {
    PathBuf::from(repo_root)
        .canonicalize()
        .map(|path| path.to_string_lossy().to_string())
        .map_err(|error| intent_ledger::LedgerError::Binding(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_belief() -> String {
        serde_json::json!({
            "type":"publish_belief","operationId":"OP-1","initiativeId":"INIT-1",
            "taskId":null,"role":"verifier","artifactType":"belief","schemaVersion":1,
            "specVersion":1,"reason":"report confidence","expectedOutcome":"alignment",
            "sourceContextBundleId":"CTX-1","payload":{"confidence":0.8}
        })
        .to_string()
    }

    #[test]
    fn configured_role_parser_accepts_only_one_strict_studio_operation() {
        assert!(matches!(
            strict_parse_studio_operation(&valid_belief()).unwrap(),
            AgentOperation::PublishBelief { .. }
        ));
        assert!(strict_parse_studio_operation(&format!("commentary {}", valid_belief())).is_err());
        assert!(strict_parse_studio_operation(
            r#"{"type":"final_report","summary":"x","changedFiles":[],"testsRun":[],"remainingRisks":[]}"#
        )
        .is_err());
        let unknown =
            valid_belief().replace("\"payload\":", "\"unknownAuthority\":true,\"payload\":");
        assert!(strict_parse_studio_operation(&unknown).is_err());
    }

    #[test]
    #[ignore = "requires local Ollama qwen3-coder:latest and can take several minutes"]
    fn qwen_product_council_smoke_invokes_every_role() {
        let root = std::env::temp_dir().join(new_id("qwen-council"));
        std::fs::create_dir_all(&root).unwrap();
        let repo = root.canonicalize().unwrap().to_string_lossy().to_string();
        let session = "qwen-council-smoke";
        let mandate = Mandate {
            id: "MANDATE-QWEN".into(),
            name: "Qwen App Foundry".into(),
            purpose: "Build a bounded standalone offline application".into(),
            allowed_modes: vec![
                InitiativeMode::DreamIdeation,
                InitiativeMode::DreamPrototype,
                InitiativeMode::DreamIncubator,
            ],
            allowed_repo_paths: vec![],
            maximum_candidates_per_cycle: 3,
            maximum_prototypes_per_cycle: 2,
            maximum_builder_iterations: 3,
            maximum_changed_files: 20,
            maximum_elapsed_minutes: 60,
            network_policy: "disabled".into(),
            package_install_policy: "forbidden".into(),
            active_branch_write_policy: "forbidden".into(),
            merge_authority: "human_only".into(),
            enabled: true,
        };
        dream_save_mandate(MandateRequest {
            session_id: session.into(),
            repo_root: repo.clone(),
            mandate,
        })
        .unwrap();
        studio_save_role_runtime(RoleRuntimeConfigRequest {
            session_id: session.into(),
            repo_root: repo.clone(),
            role: "dreamer".into(),
            runtime: "local-server".into(),
            model: "qwen3-coder:latest".into(),
            endpoint_url: Some("http://127.0.0.1:11434/v1".into()),
            timeout_seconds: 600,
            context_window_tokens: 262_144,
            maximum_output_tokens: 4_096,
            token_estimation_method: "conservative_utf8_bytes_div3".into(),
            safety_margin_tokens: 2_048,
            structured_output_behavior: "json_object".into(),
            capability_source: "local-ollama-smoke".into(),
        })
        .unwrap();
        let output = dream_start_cycle(DreamCycleRequest {
            session_id: session.into(),
            repo_root: repo.clone(),
            mandate_id: "MANDATE-QWEN".into(),
            focus: String::new(),
        })
        .unwrap();
        let initiative = output["initiative"]["id"].as_str().unwrap();
        let conn = init_audit(&PathBuf::from(&repo), session).unwrap();
        for role in [
            "dreamer",
            "skeptic",
            "fde",
            "ux_designer",
            "architect",
            "planner",
            "builder",
            "verifier",
            "reviewer",
        ] {
            let count:i64=conn.query_row("SELECT COUNT(*) FROM agent_runs WHERE initiative_id=?1 AND role=?2 AND status='completed'",params![initiative,role],|row|row.get(0)).unwrap();
            assert!(count >= 1, "missing completed {role} invocation");
        }
        let dreamer_runs:i64=conn.query_row("SELECT COUNT(*) FROM agent_runs WHERE initiative_id=?1 AND role='dreamer' AND status='completed'",[initiative],|row|row.get(0)).unwrap();
        assert!(dreamer_runs >= 2, "Dreamer synthesis round did not run");
        drop(conn);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn fake_dream_cycle_seeds_required_context_and_materializes_planner_tasks() {
        let root = std::env::temp_dir().join(format!("synthesize-dream-cycle-{}", new_id("test")));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src").join("lib.rs"), "pub fn seed() {}\n").unwrap();
        let repo = root.canonicalize().unwrap().to_string_lossy().to_string();
        let mandate = Mandate {
            id: "MANDATE-TEST".into(),
            name: "test".into(),
            purpose: "test factory seed".into(),
            allowed_modes: vec![
                InitiativeMode::DreamIdeation,
                InitiativeMode::DreamPrototype,
                InitiativeMode::DreamIncubator,
            ],
            allowed_repo_paths: vec![],
            maximum_candidates_per_cycle: 2,
            maximum_prototypes_per_cycle: 1,
            maximum_builder_iterations: 3,
            maximum_changed_files: 10,
            maximum_elapsed_minutes: 30,
            network_policy: "disabled".into(),
            package_install_policy: "forbidden".into(),
            active_branch_write_policy: "forbidden".into(),
            merge_authority: "human_only".into(),
            enabled: true,
        };
        dream_save_mandate(MandateRequest {
            session_id: "dream-test".into(),
            repo_root: repo.clone(),
            mandate,
        })
        .unwrap();
        let output = dream_start_cycle(DreamCycleRequest {
            session_id: "dream-test".into(),
            repo_root: repo.clone(),
            mandate_id: "MANDATE-TEST".into(),
            focus: String::new(),
        })
        .unwrap();
        let initiative = output["initiative"]["id"].as_str().unwrap();
        let conn = init_audit(&PathBuf::from(&repo), "dream-test").unwrap();
        let task_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM studio_tasks WHERE initiative_id=?1",
                [initiative],
                |row| row.get(0),
            )
            .unwrap();
        assert!(task_count >= 2);
        let fde_runs: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_runs WHERE initiative_id=?1 AND role='fde'",
                [initiative],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(fde_runs, 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn dream_factory_completes_a_repaired_two_task_concept_then_starts_the_next() {
        use std::process::Command;
        let root = std::env::temp_dir().join(format!("synthesize-factory-e2e-{}", new_id("test")));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub fn seed() {}\n").unwrap();
        std::fs::write(root.join(".gitignore"), ".synthesize/\n").unwrap();
        assert!(Command::new("git")
            .arg("init")
            .current_dir(&root)
            .status()
            .unwrap()
            .success());
        assert!(Command::new("git")
            .args(["add", "."])
            .current_dir(&root)
            .status()
            .unwrap()
            .success());
        assert!(Command::new("git")
            .args([
                "-c",
                "user.email=factory@test.local",
                "-c",
                "user.name=Factory Test",
                "commit",
                "-m",
                "seed"
            ])
            .current_dir(&root)
            .status()
            .unwrap()
            .success());
        let repo = root.canonicalize().unwrap().to_string_lossy().to_string();
        let mandate = Mandate {
            id: "MANDATE-E2E".into(),
            name: "e2e".into(),
            purpose: "bounded autonomous prototype".into(),
            allowed_modes: vec![
                InitiativeMode::DreamIdeation,
                InitiativeMode::DreamPrototype,
                InitiativeMode::DreamIncubator,
            ],
            allowed_repo_paths: vec![],
            maximum_candidates_per_cycle: 3,
            maximum_prototypes_per_cycle: 2,
            maximum_builder_iterations: 3,
            maximum_changed_files: 10,
            maximum_elapsed_minutes: 30,
            network_policy: "disabled".into(),
            package_install_policy: "forbidden".into(),
            active_branch_write_policy: "forbidden".into(),
            merge_authority: "human_only".into(),
            enabled: true,
        };
        dream_save_mandate(MandateRequest {
            session_id: "factory-e2e".into(),
            repo_root: repo.clone(),
            mandate,
        })
        .unwrap();
        let output_root = std::env::temp_dir().join(new_id("dream-output"));
        let output_root_id = ApplicationWorkspaceManager::new(
            &init_audit(&PathBuf::from(&repo), "factory-e2e").unwrap(),
        )
        .approve_output_root("factory-e2e", &output_root)
        .unwrap();
        dream_factory_start(DreamFactoryRequest {
            session_id: "factory-e2e".into(),
            repo_root: repo.clone(),
            mandate_id: "MANDATE-E2E".into(),
            output_root_id,
        })
        .unwrap();
        let request = DreamFactoryTickRequest {
            session_id: "factory-e2e".into(),
            repo_root: repo.clone(),
        };
        let original_head = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&root)
            .output()
            .unwrap()
            .stdout;
        let mut saw_repair = false;
        let mut saw_second = false;
        for _ in 0..32 {
            let state = dream_factory_tick(DreamFactoryTickRequest {
                session_id: request.session_id.clone(),
                repo_root: request.repo_root.clone(),
            })
            .unwrap();
            if state["stage"] == "task_revising" {
                saw_repair = true;
            }
            if state["completedDreamCount"].as_i64().unwrap_or(0) >= 1
                && !state["currentInitiativeId"].is_null()
            {
                saw_second = true;
                break;
            }
        }
        assert!(
            saw_repair,
            "first task must enter a verifier/reviewer repair loop"
        );
        assert!(
            saw_second,
            "continuous factory must start DREAM-B only after DREAM-A terminal completion"
        );
        let conn = init_audit(&PathBuf::from(&repo), "factory-e2e").unwrap();
        let passed: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM studio_tasks WHERE status='passed'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            passed >= 2,
            "DREAM-A must have two dependency-linked passed tasks"
        );
        let application_path: String = conn
            .query_row(
                "SELECT workspace_path FROM dream_applications ORDER BY rowid LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(PathBuf::from(&application_path)
            .join("index.html")
            .is_file());
        assert!(PathBuf::from(&application_path).join("app.js").is_file());
        ApplicationWorkspaceManager::new(&conn)
            .validate_offline_web_app(&PathBuf::from(&application_path))
            .unwrap();
        let current_head = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&root)
            .output()
            .unwrap()
            .stdout;
        assert_eq!(
            original_head, current_head,
            "active checkout was mutated or merged"
        );
        drop(conn);
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(output_root);
    }

    #[test]
    fn builder_artifact_enters_existing_governed_diff_queue() {
        let repo = std::env::temp_dir().join(format!("studio-builder-{}", new_id("test")));
        std::fs::create_dir_all(repo.join("src/studio")).unwrap();
        std::fs::write(repo.join("src/studio/mod.rs"), "before\n").unwrap();
        let repo_string = repo.to_string_lossy().to_string();
        let conn = init_audit(&repo, "studio-patch-session").unwrap();
        let created = bootstrap_studio(
            &conn,
            "studio-patch-session",
            &repo_string,
            "Change the bounded fixture value",
        )
        .unwrap();
        let initiative_id = created["initiative"]["id"].as_str().unwrap();
        approve_studio_scope(&conn, initiative_id).unwrap();
        let task_id: String = conn
            .query_row(
                "SELECT id FROM studio_tasks WHERE initiative_id=?1 ORDER BY id LIMIT 1",
                [initiative_id],
                |row| row.get(0),
            )
            .unwrap();
        let prepared = prepare_role_run(
            &conn,
            initiative_id,
            Some(&task_id),
            Role::Builder,
            "fake",
            "fixture",
        )
        .unwrap();
        let operation: AgentOperation = serde_json::from_value(json!({
            "type":"propose_artifact",
            "operationId":"STUDIO-PATCH-ARTIFACT",
            "initiativeId":initiative_id,
            "taskId":task_id,
            "role":"builder",
            "artifactType":"patch_proposal",
            "schemaVersion":1,
            "specVersion":1,
            "reason":"Implement the bounded task",
            "expectedOutcome":"The governed diff queue contains a validated candidate",
            "sourceContextBundleId":prepared.context_capsule.id,
            "payload":{
                "type":"propose_patch",
                "proposalId":"STUDIO-PATCH-1",
                "summary":"Change fixture value",
                "baseCommit":null,
                "currentCommit":null,
                "files":[{
                    "id":"STUDIO-PATCH-FILE-1",
                    "path":"src/studio/mod.rs",
                    "beforeSha256":crate::sha256_str("before\n"),
                    "patch":"diff --git a/src/studio/mod.rs b/src/studio/mod.rs\n--- a/src/studio/mod.rs\n+++ b/src/studio/mod.rs\n@@ -1 +1 @@\n-before\n+after\n"
                }],
                "riskNotes":[],
                "suggestedCommands":[]
            }
        }))
        .unwrap();
        assert!(apply_studio_operation(&conn, &prepared, &operation)
            .unwrap()
            .is_none());
        let status: String = conn
            .query_row(
                "SELECT status FROM patch_proposals WHERE proposal_id='STUDIO-PATCH-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "validated");
        let artifact_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM operation_links WHERE operation_id='STUDIO-PATCH-ARTIFACT'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(artifact_count, 1);
        let (requirements, adrs, context_id): (String, String, String) = conn
            .query_row(
                "SELECT requirement_ids_json, adr_ids_json, context_bundle_id
                 FROM operation_links WHERE operation_id='STUDIO-PATCH-ARTIFACT'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_ne!(requirements, "[]");
        assert_ne!(adrs, "[]");
        assert_eq!(context_id, prepared.context_capsule.id);
        drop(conn);
        std::fs::remove_dir_all(repo).unwrap();
    }
}

fn with_ledger<T>(
    session_id: &str,
    repo_root: &str,
    operation: impl FnOnce(&Ledger<'_>) -> std::result::Result<T, intent_ledger::LedgerError>,
) -> Result<T, String> {
    let canonical_repo = canonical(repo_root).map_err(|error| error.to_string())?;
    let conn = init_audit(&PathBuf::from(canonical_repo), session_id)
        .map_err(|error| error.to_string())?;
    operation(&Ledger::new(&conn)).map_err(|error| error.to_string())
}

fn bound_connection(req: &StudioInitiativeRequest) -> Result<rusqlite::Connection, String> {
    let canonical_repo = canonical(&req.repo_root).map_err(|error| error.to_string())?;
    let conn = init_audit(&PathBuf::from(&canonical_repo), &req.session_id)
        .map_err(|error| error.to_string())?;
    let initiative = Ledger::new(&conn)
        .get_initiative(&req.initiative_id)
        .map_err(|error| error.to_string())?;
    if initiative.session_id != req.session_id || initiative.repo_root != canonical_repo {
        return Err("initiative session/repository binding rejected".into());
    }
    Ok(conn)
}

fn with_bound_ledger<T>(
    req: &StudioInitiativeRequest,
    operation: impl FnOnce(&Ledger<'_>) -> std::result::Result<T, intent_ledger::LedgerError>,
) -> Result<T, String> {
    let conn = bound_connection(req)?;
    operation(&Ledger::new(&conn)).map_err(|error| error.to_string())
}
