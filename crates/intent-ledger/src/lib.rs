use audit_log::new_id;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const MAX_ARTIFACT_BYTES: usize = 512 * 1024;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("record not found: {0}")]
    NotFound(String),
    #[error("invalid transition: {0}")]
    InvalidTransition(String),
    #[error("binding rejected: {0}")]
    Binding(String),
    #[error("permission rejected: {0}")]
    Permission(String),
    #[error("validation rejected: {0}")]
    Validation(String),
    #[error("mandatory evidence is incomplete: {0}")]
    MissingEvidence(String),
    #[error("immutable record cannot be changed: {0}")]
    Immutable(String),
    #[error("budget rejected: {0}")]
    Budget(String),
}

pub type Result<T> = std::result::Result<T, LedgerError>;

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $value),+ }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = LedgerError;
            fn try_from(value: &str) -> Result<Self> {
                match value { $($value => Ok(Self::$variant),)+ other => Err(LedgerError::Validation(format!("unknown {}: {other}", stringify!($name)))) }
            }
        }
    };
}

string_enum!(InitiativeMode {
    Assist => "assist",
    Studio => "studio",
    DreamIdeation => "dream_ideation",
    DreamPrototype => "dream_prototype",
    DreamIncubator => "dream_incubator"
});

string_enum!(InitiativeStatus {
    Created => "created",
    Discovery => "discovery",
    Concepting => "concepting",
    Challenging => "challenging",
    UxDesign => "ux_design",
    Architecture => "architecture",
    Planning => "planning",
    AwaitingScopeApproval => "awaiting_scope_approval",
    Implementing => "implementing",
    Verifying => "verifying",
    Reviewing => "reviewing",
    AwaitingMergeReview => "awaiting_merge_review",
    Completed => "completed",
    Blocked => "blocked",
    Paused => "paused",
    Abandoned => "abandoned",
    Failed => "failed"
});

string_enum!(RequirementStatus {
    Proposed => "proposed",
    Approved => "approved",
    ImplementationStarted => "implementation_started",
    Implemented => "implemented",
    VerificationPending => "verification_pending",
    Verified => "verified",
    OutcomePending => "outcome_pending",
    OutcomeConfirmed => "outcome_confirmed",
    OutcomeDisproven => "outcome_disproven",
    Rejected => "rejected",
    Blocked => "blocked",
    Failed => "failed",
    Repairing => "repairing",
    Superseded => "superseded"
});

string_enum!(TaskStatus {
    Ready => "ready",
    InProgress => "in_progress",
    AwaitingOperationApproval => "awaiting_operation_approval",
    Applied => "applied",
    Verifying => "verifying",
    Reviewing => "reviewing",
    Revising => "revising",
    Replanning => "replanning",
    Passed => "passed",
    Blocked => "blocked",
    BlockedContextOverflow => "blocked_context_overflow",
    Failed => "failed",
    Cancelled => "cancelled"
});

string_enum!(DreamStatus {
    Proposed => "proposed",
    Deduplicated => "deduplicated",
    Challenged => "challenged",
    Rejected => "rejected",
    Shortlisted => "shortlisted",
    PrototypeApproved => "prototype_approved",
    Prototyping => "prototyping",
    Validated => "validated",
    PromotedToGoal => "promoted_to_goal",
    Archived => "archived"
});

string_enum!(Role {
    Dreamer => "dreamer",
    Fde => "fde",
    UxDesigner => "ux_designer",
    Skeptic => "skeptic",
    Architect => "architect",
    Planner => "planner",
    Builder => "builder",
    Verifier => "verifier",
    Reviewer => "reviewer",
    Human => "human",
    System => "system"
});

string_enum!(ReviewerVerdict {
    Pass => "pass",
    Revise => "revise",
    Replan => "replan",
    Blocked => "blocked"
});

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Initiative {
    pub id: String,
    pub session_id: String,
    pub repo_root: String,
    pub mode: InitiativeMode,
    pub title: String,
    pub source: String,
    pub status: InitiativeStatus,
    pub resume_status: Option<InitiativeStatus>,
    pub standing_mandate_id: Option<String>,
    pub active_spec_version: i64,
    pub active_worktree_id: Option<String>,
    pub autonomy_level: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    pub id: String,
    pub initiative_id: String,
    pub spec_version: i64,
    pub status: RequirementStatus,
    pub required_evidence: Vec<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StudioTask {
    pub id: String,
    pub initiative_id: String,
    pub spec_version: i64,
    pub status: TaskStatus,
    pub assigned_role: Role,
    pub iteration_count: i64,
    pub max_iterations: i64,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceInput {
    pub requirement_id: String,
    pub task_id: Option<String>,
    pub evidence_type: String,
    pub status: String,
    pub provenance: String,
    pub output_ref: Option<String>,
    pub summary: String,
    pub content_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactEnvelope {
    pub operation_id: String,
    pub initiative_id: String,
    pub task_id: Option<String>,
    pub role: Role,
    pub artifact_type: String,
    pub schema_version: i64,
    pub spec_version: i64,
    pub source_context_bundle_id: Option<String>,
    pub reason: String,
    pub expected_outcome: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Mandate {
    pub id: String,
    pub name: String,
    pub purpose: String,
    pub allowed_modes: Vec<InitiativeMode>,
    pub allowed_repo_paths: Vec<String>,
    pub maximum_candidates_per_cycle: u32,
    pub maximum_prototypes_per_cycle: u32,
    pub maximum_builder_iterations: u32,
    pub maximum_changed_files: u32,
    pub maximum_elapsed_minutes: u32,
    pub network_policy: String,
    pub package_install_policy: String,
    pub active_branch_write_policy: String,
    pub merge_authority: String,
    pub enabled: bool,
}

impl Mandate {
    pub fn validate(&self) -> Result<()> {
        if self.allowed_modes.is_empty() {
            return Err(LedgerError::Validation(
                "mandate has no allowed modes".into(),
            ));
        }
        if self.maximum_candidates_per_cycle == 0
            || self.maximum_builder_iterations == 0
            || self.maximum_elapsed_minutes == 0
        {
            return Err(LedgerError::Budget(
                "mandate limits must be positive and bounded".into(),
            ));
        }
        if self.merge_authority != "human_only" {
            return Err(LedgerError::Permission(
                "merge authority must remain human_only".into(),
            ));
        }
        if self.active_branch_write_policy != "forbidden" {
            return Err(LedgerError::Permission(
                "Dream active branch writes must remain forbidden".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationEvent {
    pub id: String,
    pub initiative_id: String,
    pub task_id: Option<String>,
    pub actor_role: Role,
    pub kind: String,
    pub requirement_ids: Vec<String>,
    pub adr_ids: Vec<String>,
    pub assumption_ids: Vec<String>,
    pub features: BTreeMap<String, f64>,
    pub provenance: String,
    pub redacted_summary: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AlignmentQuestion {
    pub id: String,
    pub initiative_id: String,
    pub task_id: Option<String>,
    pub from_role: Role,
    pub to_role: Role,
    pub reason: String,
    pub question: String,
    pub blocking: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProofReport {
    pub id: String,
    pub initiative_id: String,
    pub spec_version: i64,
    pub objective: Vec<Value>,
    pub assumptions: Vec<Value>,
    pub constraints: Vec<Value>,
    pub requirements: Vec<Value>,
    pub architecture_decisions: Vec<Value>,
    pub ux_contracts: Vec<Value>,
    pub tasks: Vec<Value>,
    pub evidence: Vec<Value>,
    pub operations: Vec<Value>,
    pub complete: Vec<String>,
    pub incomplete: Vec<String>,
    pub blocked: Vec<String>,
    pub unverified: Vec<String>,
    pub outcome_pending: Vec<String>,
    pub remaining_risks: Vec<String>,
    pub excludes_sensitive_context: bool,
}

pub struct Ledger<'a> {
    conn: &'a Connection,
}

impl<'a> Ledger<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn create_initiative(
        &self,
        session_id: &str,
        repo_root: &str,
        title: &str,
        mode: InitiativeMode,
        source: &str,
        mandate_id: Option<&str>,
    ) -> Result<Initiative> {
        if title.trim().is_empty() || title.chars().count() > 240 {
            return Err(LedgerError::Validation(
                "initiative title must contain 1-240 characters".into(),
            ));
        }
        if matches!(
            mode,
            InitiativeMode::DreamIdeation
                | InitiativeMode::DreamPrototype
                | InitiativeMode::DreamIncubator
        ) {
            let mandate_id = mandate_id.ok_or_else(|| {
                LedgerError::Permission("Dream initiative requires a standing mandate".into())
            })?;
            self.require_enabled_mandate(session_id, repo_root, mandate_id, mode)?;
        }
        let id = new_id("INIT");
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO initiatives (id, session_id, repo_root, mode, title, source, status, standing_mandate_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'created', ?7)",
            params![id, session_id, repo_root, mode.as_str(), title.trim(), source, mandate_id],
        )?;
        tx.execute(
            "INSERT INTO spec_versions (initiative_id, version, status, change_reason, payload_json)
             VALUES (?1, 1, 'draft', 'initial initiative specification', '{}')",
            [&id],
        )?;
        tx.execute(
            "INSERT INTO autonomy_usage (initiative_id) VALUES (?1)",
            [&id],
        )?;
        tx.commit()?;
        self.record_event(OrchestrationEvent {
            id: new_id("EVENT"),
            initiative_id: id.clone(),
            task_id: None,
            actor_role: Role::Human,
            kind: "initiative.created".into(),
            requirement_ids: vec![],
            adr_ids: vec![],
            assumption_ids: vec![],
            features: BTreeMap::new(),
            provenance: source.into(),
            redacted_summary: format!("Created {mode:?} initiative"),
            created_at: None,
        })?;
        self.get_initiative(&id)
    }

    pub fn get_initiative(&self, id: &str) -> Result<Initiative> {
        self.conn
            .query_row(
                "SELECT id, session_id, repo_root, mode, title, source, status, resume_status,
                        standing_mandate_id, active_spec_version, active_worktree_id,
                        autonomy_level, created_at, updated_at
                 FROM initiatives WHERE id = ?1",
                [id],
                |row| {
                    let mode: String = row.get(3)?;
                    let status: String = row.get(6)?;
                    let resume: Option<String> = row.get(7)?;
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        mode,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        status,
                        resume,
                        row.get::<_, Option<String>>(8)?,
                        row.get::<_, i64>(9)?,
                        row.get::<_, Option<String>>(10)?,
                        row.get::<_, i64>(11)?,
                        row.get::<_, String>(12)?,
                        row.get::<_, String>(13)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| LedgerError::NotFound(format!("initiative {id}")))
            .and_then(|r| {
                Ok(Initiative {
                    id: r.0,
                    session_id: r.1,
                    repo_root: r.2,
                    mode: InitiativeMode::try_from(r.3.as_str())?,
                    title: r.4,
                    source: r.5,
                    status: InitiativeStatus::try_from(r.6.as_str())?,
                    resume_status: r.7.as_deref().map(InitiativeStatus::try_from).transpose()?,
                    standing_mandate_id: r.8,
                    active_spec_version: r.9,
                    active_worktree_id: r.10,
                    autonomy_level: r.11,
                    created_at: r.12,
                    updated_at: r.13,
                })
            })
    }

    pub fn list_initiatives(&self, session_id: &str, repo_root: &str) -> Result<Vec<Initiative>> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM initiatives WHERE session_id = ?1 AND repo_root = ?2
             ORDER BY updated_at DESC, rowid DESC",
        )?;
        let ids: Vec<String> = stmt
            .query_map(params![session_id, repo_root], |row| row.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        ids.iter().map(|id| self.get_initiative(id)).collect()
    }

    pub fn transition_initiative(
        &self,
        id: &str,
        target: InitiativeStatus,
        actor: Role,
        reason: &str,
    ) -> Result<Initiative> {
        let initiative = self.get_initiative(id)?;
        if actor != Role::Human && !matches!(actor, Role::System) {
            return Err(LedgerError::Permission(
                "agent roles may request but not perform initiative transitions".into(),
            ));
        }
        if !initiative_transition_allowed(initiative.status, target) {
            return Err(LedgerError::InvalidTransition(format!(
                "initiative {} cannot move from {} to {}",
                id,
                initiative.status.as_str(),
                target.as_str()
            )));
        }
        if target == InitiativeStatus::Completed {
            self.require_completion_evidence(id, initiative.active_spec_version)?;
        }
        let resume_status = if target == InitiativeStatus::Paused {
            Some(initiative.status.as_str())
        } else {
            None
        };
        self.conn.execute(
            "UPDATE initiatives SET status = ?2, resume_status = CASE WHEN ?2 = 'paused' THEN ?3 ELSE NULL END,
                    updated_at = datetime('now') WHERE id = ?1 AND status = ?4",
            params![id, target.as_str(), resume_status, initiative.status.as_str()],
        )?;
        self.record_transition_event(id, actor, initiative.status, target, reason)?;
        self.get_initiative(id)
    }

    pub fn resume_initiative(&self, id: &str, reason: &str) -> Result<Initiative> {
        let initiative = self.get_initiative(id)?;
        if initiative.status != InitiativeStatus::Paused {
            return Err(LedgerError::InvalidTransition(
                "only a paused initiative can resume".into(),
            ));
        }
        let target = initiative
            .resume_status
            .unwrap_or(InitiativeStatus::Discovery);
        self.conn.execute(
            "UPDATE initiatives SET status = ?2, resume_status = NULL, updated_at = datetime('now')
             WHERE id = ?1 AND status = 'paused'",
            params![id, target.as_str()],
        )?;
        self.record_transition_event(id, Role::Human, InitiativeStatus::Paused, target, reason)?;
        self.get_initiative(id)
    }

    pub fn freeze_spec(&self, initiative_id: &str, version: i64, payload: &Value) -> Result<()> {
        self.require_active_spec(initiative_id, version)?;
        let changed = self.conn.execute(
            "UPDATE spec_versions SET status = 'frozen', payload_json = ?3, frozen_at = datetime('now')
             WHERE initiative_id = ?1 AND version = ?2 AND status = 'draft'",
            params![initiative_id, version, serde_json::to_string(payload)?],
        )?;
        if changed != 1 {
            return Err(LedgerError::Immutable(format!(
                "spec {initiative_id} v{version} is already frozen or missing"
            )));
        }
        Ok(())
    }

    pub fn create_spec_version(
        &self,
        initiative_id: &str,
        change_reason: &str,
        payload: &Value,
    ) -> Result<i64> {
        let initiative = self.get_initiative(initiative_id)?;
        let prior_status: String = self.conn.query_row(
            "SELECT status FROM spec_versions WHERE initiative_id = ?1 AND version = ?2",
            params![initiative_id, initiative.active_spec_version],
            |row| row.get(0),
        )?;
        if prior_status != "frozen" {
            return Err(LedgerError::Immutable(
                "a new spec version requires the prior active version to be frozen".into(),
            ));
        }
        let next = initiative.active_spec_version + 1;
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO spec_versions (initiative_id, version, status, change_reason, payload_json)
             VALUES (?1, ?2, 'draft', ?3, ?4)",
            params![initiative_id, next, change_reason, serde_json::to_string(payload)?],
        )?;
        tx.execute(
            "UPDATE initiatives SET active_spec_version = ?2, updated_at = datetime('now') WHERE id = ?1",
            params![initiative_id, next],
        )?;
        tx.commit()?;
        Ok(next)
    }

    pub fn add_requirement(&self, requirement: &Requirement) -> Result<()> {
        self.require_mutable_spec(&requirement.initiative_id, requirement.spec_version)?;
        if requirement.required_evidence.is_empty() {
            return Err(LedgerError::Validation(format!(
                "requirement {} must declare mandatory evidence",
                requirement.id
            )));
        }
        self.conn.execute(
            "INSERT INTO requirements (id, initiative_id, spec_version, status, required_evidence_json, payload_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                requirement.id,
                requirement.initiative_id,
                requirement.spec_version,
                requirement.status.as_str(),
                serde_json::to_string(&requirement.required_evidence)?,
                serde_json::to_string(&requirement.payload)?
            ],
        )?;
        Ok(())
    }

    pub fn get_requirement(&self, id: &str) -> Result<Requirement> {
        let row = self
            .conn
            .query_row(
                "SELECT id, initiative_id, spec_version, status, required_evidence_json, payload_json
                 FROM requirements WHERE id = ?1",
                [id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| LedgerError::NotFound(format!("requirement {id}")))?;
        Ok(Requirement {
            id: row.0,
            initiative_id: row.1,
            spec_version: row.2,
            status: RequirementStatus::try_from(row.3.as_str())?,
            required_evidence: serde_json::from_str(&row.4)?,
            payload: serde_json::from_str(&row.5)?,
        })
    }

    pub fn transition_requirement(
        &self,
        id: &str,
        target: RequirementStatus,
    ) -> Result<Requirement> {
        let requirement = self.get_requirement(id)?;
        if !requirement_transition_allowed(requirement.status, target) {
            return Err(LedgerError::InvalidTransition(format!(
                "requirement {id} cannot move from {} to {}",
                requirement.status.as_str(),
                target.as_str()
            )));
        }
        if target == RequirementStatus::Verified {
            self.require_evidence(&requirement)?;
        }
        self.conn.execute(
            "UPDATE requirements SET status = ?2, updated_at = datetime('now')
             WHERE id = ?1 AND status = ?3",
            params![id, target.as_str(), requirement.status.as_str()],
        )?;
        self.get_requirement(id)
    }

    pub fn record_evidence(&self, initiative_id: &str, input: &EvidenceInput) -> Result<String> {
        let requirement = self.get_requirement(&input.requirement_id)?;
        if requirement.initiative_id != initiative_id {
            return Err(LedgerError::Binding(
                "evidence requirement belongs to another initiative".into(),
            ));
        }
        if !matches!(input.status.as_str(), "passed" | "failed" | "pending") {
            return Err(LedgerError::Validation(
                "evidence status must be passed, failed, or pending".into(),
            ));
        }
        if input.provenance.trim().is_empty() || input.summary.trim().is_empty() {
            return Err(LedgerError::Validation(
                "evidence requires provenance and a concise summary".into(),
            ));
        }
        let id = new_id("EVID");
        let hash = input
            .content_sha256
            .clone()
            .unwrap_or_else(|| sha256(input.summary.as_bytes()));
        self.conn.execute(
            "INSERT INTO verification_evidence
             (id, initiative_id, requirement_id, task_id, evidence_type, status, provenance, output_ref, content_sha256, summary)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                initiative_id,
                input.requirement_id,
                input.task_id,
                input.evidence_type,
                input.status,
                input.provenance,
                input.output_ref,
                hash,
                input.summary
            ],
        )?;
        Ok(id)
    }

    pub fn add_task(&self, task: &StudioTask) -> Result<()> {
        self.require_active_spec(&task.initiative_id, task.spec_version)?;
        if task.max_iterations < 1 || task.max_iterations > 50 {
            return Err(LedgerError::Budget(
                "task max_iterations must be within 1..=50".into(),
            ));
        }
        validate_task_scope(&task.payload)?;
        self.conn.execute(
            "INSERT INTO studio_tasks
             (id, initiative_id, spec_version, status, assigned_role, iteration_count, max_iterations, payload_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                task.id,
                task.initiative_id,
                task.spec_version,
                task.status.as_str(),
                task.assigned_role.as_str(),
                task.iteration_count,
                task.max_iterations,
                serde_json::to_string(&task.payload)?
            ],
        )?;
        Ok(())
    }

    pub fn get_task(&self, id: &str) -> Result<StudioTask> {
        let row = self
            .conn
            .query_row(
                "SELECT id, initiative_id, spec_version, status, assigned_role, iteration_count,
                        max_iterations, payload_json FROM studio_tasks WHERE id = ?1",
                [id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, String>(7)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| LedgerError::NotFound(format!("task {id}")))?;
        Ok(StudioTask {
            id: row.0,
            initiative_id: row.1,
            spec_version: row.2,
            status: TaskStatus::try_from(row.3.as_str())?,
            assigned_role: Role::try_from(row.4.as_str())?,
            iteration_count: row.5,
            max_iterations: row.6,
            payload: serde_json::from_str(&row.7)?,
        })
    }

    pub fn route_review_verdict(
        &self,
        task_id: &str,
        verdict: ReviewerVerdict,
    ) -> Result<StudioTask> {
        let task = self.get_task(task_id)?;
        if task.status != TaskStatus::Reviewing {
            return Err(LedgerError::InvalidTransition(
                "review verdict requires a task in reviewing state".into(),
            ));
        }
        let target = match verdict {
            ReviewerVerdict::Pass => TaskStatus::Passed,
            ReviewerVerdict::Revise => TaskStatus::Revising,
            ReviewerVerdict::Replan => TaskStatus::Replanning,
            ReviewerVerdict::Blocked => TaskStatus::Blocked,
        };
        let next_iterations = task.iteration_count + i64::from(verdict == ReviewerVerdict::Revise);
        if next_iterations > task.max_iterations {
            self.conn.execute(
                "UPDATE studio_tasks SET status = 'blocked', updated_at = datetime('now') WHERE id = ?1",
                [task_id],
            )?;
            return Err(LedgerError::Budget(format!(
                "task {task_id} exhausted {} iterations and was blocked",
                task.max_iterations
            )));
        }
        self.conn.execute(
            "UPDATE studio_tasks SET status = ?2, iteration_count = ?3, updated_at = datetime('now')
             WHERE id = ?1 AND status = 'reviewing'",
            params![task_id, target.as_str(), next_iterations],
        )?;
        self.get_task(task_id)
    }

    pub fn publish_artifact(
        &self,
        envelope: &ArtifactEnvelope,
        agent_run_id: Option<&str>,
        redacted_summary: &str,
    ) -> Result<String> {
        self.require_active_spec(&envelope.initiative_id, envelope.spec_version)?;
        if envelope.operation_id.trim().is_empty()
            || envelope.reason.trim().is_empty()
            || envelope.expected_outcome.trim().is_empty()
        {
            return Err(LedgerError::Validation(
                "artifact envelope requires operation ID, reason, and expected outcome".into(),
            ));
        }
        validate_role_artifact_permission(envelope.role, &envelope.artifact_type)?;
        let existing_operation: Option<String> = self
            .conn
            .query_row(
                "SELECT operation_id FROM operation_links WHERE operation_id=?1",
                [&envelope.operation_id],
                |row| row.get(0),
            )
            .optional()?;
        if existing_operation.is_some() {
            return Err(LedgerError::Binding(format!(
                "operation ID {} has already been used",
                envelope.operation_id
            )));
        }
        let payload = serde_json::to_vec(&envelope.payload)?;
        if payload.len() > MAX_ARTIFACT_BYTES {
            return Err(LedgerError::Validation(format!(
                "artifact exceeds {MAX_ARTIFACT_BYTES} byte limit"
            )));
        }
        if let Some(task_id) = &envelope.task_id {
            let task = self.get_task(task_id)?;
            if task.initiative_id != envelope.initiative_id
                || task.spec_version != envelope.spec_version
            {
                return Err(LedgerError::Binding(
                    "artifact task binding does not match initiative/spec".into(),
                ));
            }
        }
        let id = new_id("ART");
        self.conn.execute(
            "INSERT INTO artifacts
             (id, initiative_id, task_id, spec_version, agent_run_id, role, artifact_type,
              schema_version, content_sha256, payload_json, redacted_summary)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                envelope.initiative_id,
                envelope.task_id,
                envelope.spec_version,
                agent_run_id,
                envelope.role.as_str(),
                envelope.artifact_type,
                envelope.schema_version,
                sha256(&payload),
                String::from_utf8_lossy(&payload),
                truncate(redacted_summary, 800)
            ],
        )?;
        let operation_bytes = serde_json::to_vec(envelope)?;
        let string_ids = |key: &str| -> Vec<String> {
            envelope
                .payload
                .get(key)
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default()
        };
        let mut requirement_ids = string_ids("requirementIds");
        let mut adr_ids = string_ids("adrIds");
        if let Some(task_id) = envelope.task_id.as_deref() {
            let task = self.get_task(task_id)?;
            let task_ids = |key: &str| -> Vec<String> {
                task.payload
                    .get(key)
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            };
            if requirement_ids.is_empty() {
                requirement_ids = task_ids("requirementIds");
            }
            if adr_ids.is_empty() {
                adr_ids = task_ids("adrIds");
            }
        }
        self.conn.execute(
            "INSERT INTO operation_links
             (operation_id, initiative_id, task_id, spec_version, requirement_ids_json,
              adr_ids_json, context_bundle_id, operation_sha256)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                envelope.operation_id,
                envelope.initiative_id,
                envelope.task_id,
                envelope.spec_version,
                serde_json::to_string(&requirement_ids)?,
                serde_json::to_string(&adr_ids)?,
                envelope.source_context_bundle_id,
                sha256(&operation_bytes)
            ],
        )?;
        Ok(id)
    }

    pub fn publish_belief(
        &self,
        initiative_id: &str,
        task_id: Option<&str>,
        spec_version: i64,
        agent_run_id: Option<&str>,
        role: Role,
        payload: &Value,
    ) -> Result<String> {
        self.require_active_spec(initiative_id, spec_version)?;
        if serde_json::to_vec(payload)?.len() > MAX_ARTIFACT_BYTES {
            return Err(LedgerError::Validation(
                "belief payload exceeds artifact limit".into(),
            ));
        }
        if let Some(task_id) = task_id {
            let task = self.get_task(task_id)?;
            if task.initiative_id != initiative_id || task.spec_version != spec_version {
                return Err(LedgerError::Binding(
                    "belief task binding is invalid".into(),
                ));
            }
        }
        let id = new_id("BELIEF");
        self.conn.execute(
            "INSERT INTO agent_beliefs
             (id, initiative_id, task_id, spec_version, agent_run_id, role, payload_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                initiative_id,
                task_id,
                spec_version,
                agent_run_id,
                role.as_str(),
                serde_json::to_string(payload)?
            ],
        )?;
        Ok(id)
    }

    pub fn open_question(&self, question: &AlignmentQuestion) -> Result<String> {
        let initiative = self.get_initiative(&question.initiative_id)?;
        if let Some(task_id) = &question.task_id {
            let task = self.get_task(task_id)?;
            if task.initiative_id != question.initiative_id
                || task.spec_version != initiative.active_spec_version
            {
                return Err(LedgerError::Binding(
                    "question task binding is invalid".into(),
                ));
            }
        }
        if question.question.trim().is_empty() || question.reason.trim().is_empty() {
            return Err(LedgerError::Validation(
                "question and reason are required".into(),
            ));
        }
        self.conn.execute(
            "INSERT INTO alignment_questions
             (id, initiative_id, task_id, from_role, to_role, blocking, status, payload_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'open', ?7)",
            params![
                question.id,
                question.initiative_id,
                question.task_id,
                question.from_role.as_str(),
                question.to_role.as_str(),
                i64::from(question.blocking),
                serde_json::to_string(question)?
            ],
        )?;
        Ok(question.id.clone())
    }

    pub fn answer_question(
        &self,
        question_id: &str,
        answering_role: Role,
        answer: &Value,
    ) -> Result<()> {
        let to_role: String = self
            .conn
            .query_row(
                "SELECT to_role FROM alignment_questions WHERE id=?1 AND status='open'",
                [question_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| LedgerError::NotFound(format!("open question {question_id}")))?;
        if answering_role != Role::Human && answering_role.as_str() != to_role {
            return Err(LedgerError::Permission(
                "only the addressed role or local human may answer a question".into(),
            ));
        }
        self.conn.execute(
            "UPDATE alignment_questions SET status='answered', answer_json=?2, answered_at=datetime('now')
             WHERE id=?1 AND status='open'",
            params![question_id, serde_json::to_string(answer)?],
        )?;
        Ok(())
    }

    pub fn invalidate_assumption(
        &self,
        assumption_id: &str,
        evidence_summary: &str,
    ) -> Result<bool> {
        let row: (String, String, String) = self
            .conn
            .query_row(
                "SELECT initiative_id, status, impact_if_false FROM assumptions WHERE id=?1",
                [assumption_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
            .ok_or_else(|| LedgerError::NotFound(format!("assumption {assumption_id}")))?;
        if matches!(row.1.as_str(), "invalidated" | "superseded") {
            return Err(LedgerError::Immutable(
                "assumption was already invalidated or superseded".into(),
            ));
        }
        self.conn.execute(
            "UPDATE assumptions SET status='invalidated', updated_at=datetime('now') WHERE id=?1",
            [assumption_id],
        )?;
        let high_impact = row.2 == "high";
        self.record_event(OrchestrationEvent {
            id: new_id("EVENT"),
            initiative_id: row.0,
            task_id: None,
            actor_role: Role::Verifier,
            kind: "assumption.invalidated".into(),
            requirement_ids: vec![],
            adr_ids: vec![],
            assumption_ids: vec![assumption_id.into()],
            features: BTreeMap::from([("severity".into(), if high_impact { 0.9 } else { 0.5 })]),
            provenance: "verification-evidence".into(),
            redacted_summary: truncate(evidence_summary, 800),
            created_at: None,
        })?;
        Ok(high_impact)
    }

    pub fn upsert_mandate(
        &self,
        session_id: &str,
        repo_root: &str,
        mandate: &Mandate,
        approved_by: &str,
    ) -> Result<()> {
        mandate.validate()?;
        if approved_by != "local-user" {
            return Err(LedgerError::Permission(
                "standing mandates require local-user approval".into(),
            ));
        }
        self.conn.execute(
            "INSERT INTO standing_mandates
             (id, session_id, repo_root, enabled, payload_json, approved_by_source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET enabled=excluded.enabled, payload_json=excluded.payload_json,
                 approved_by_source=excluded.approved_by_source, updated_at=datetime('now')",
            params![
                mandate.id,
                session_id,
                repo_root,
                i64::from(mandate.enabled),
                serde_json::to_string(mandate)?,
                approved_by
            ],
        )?;
        Ok(())
    }

    pub fn create_dream(&self, initiative_id: &str, payload: &Value) -> Result<String> {
        let initiative = self.get_initiative(initiative_id)?;
        if !matches!(
            initiative.mode,
            InitiativeMode::DreamIdeation
                | InitiativeMode::DreamPrototype
                | InitiativeMode::DreamIncubator
                | InitiativeMode::Studio
        ) {
            return Err(LedgerError::Permission(
                "Dream Contracts are not valid in Assist Mode".into(),
            ));
        }
        validate_dream_contract(payload)?;
        let fingerprint = dream_fingerprint(payload);
        let duplicate: Option<String> = self
            .conn
            .query_row(
                "SELECT dream_id FROM dream_dedup_index WHERE session_id=?1 AND fingerprint=?2",
                params![initiative.session_id, fingerprint],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(existing_id) = duplicate {
            self.record_event(OrchestrationEvent {
                id: new_id("EVENT"),
                initiative_id: initiative_id.into(),
                task_id: None,
                actor_role: Role::System,
                kind: "dream.candidate_deduplicated".into(),
                requirement_ids: vec![],
                adr_ids: vec![],
                assumption_ids: vec![],
                features: BTreeMap::from([("deduplicated".into(), 1.0)]),
                provenance: "semantic-fingerprint-v1".into(),
                redacted_summary: "Duplicate Dream candidate reused an existing inbox item.".into(),
                created_at: None,
            })?;
            return Ok(existing_id);
        }
        if let Some(mandate_id) = &initiative.standing_mandate_id {
            let mandate_json: String = self.conn.query_row(
                "SELECT payload_json FROM standing_mandates WHERE id=?1 AND enabled=1",
                [mandate_id],
                |row| row.get(0),
            )?;
            let mandate: Mandate = serde_json::from_str(&mandate_json)?;
            let used: i64 = self.conn.query_row(
                "SELECT candidates_created FROM autonomy_usage WHERE initiative_id=?1",
                [initiative_id],
                |row| row.get(0),
            )?;
            if used >= i64::from(mandate.maximum_candidates_per_cycle) {
                return Err(LedgerError::Budget(format!(
                    "Dream candidate budget exhausted ({used}/{})",
                    mandate.maximum_candidates_per_cycle
                )));
            }
        }
        let id = payload
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| new_id("DREAM"));
        let horizon = payload
            .get("horizon")
            .and_then(Value::as_str)
            .unwrap_or("adjacent");
        self.conn.execute(
            "INSERT INTO dream_contracts (id, initiative_id, horizon, status, payload_json)
             VALUES (?1, ?2, ?3, 'proposed', ?4)",
            params![id, initiative_id, horizon, serde_json::to_string(payload)?],
        )?;
        self.conn.execute(
            "INSERT INTO dream_dedup_index (dream_id, session_id, fingerprint) VALUES (?1, ?2, ?3)",
            params![id, initiative.session_id, fingerprint],
        )?;
        self.conn.execute(
            "UPDATE autonomy_usage SET candidates_created=candidates_created+1, updated_at=datetime('now')
             WHERE initiative_id=?1",
            [initiative_id],
        )?;
        Ok(id)
    }

    pub fn transition_dream(&self, dream_id: &str, target: DreamStatus, actor: Role) -> Result<()> {
        if actor != Role::Human && actor != Role::System {
            return Err(LedgerError::Permission(
                "agents may recommend but not authorize Dream lifecycle transitions".into(),
            ));
        }
        let current: String = self
            .conn
            .query_row(
                "SELECT status FROM dream_contracts WHERE id = ?1",
                [dream_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| LedgerError::NotFound(format!("dream {dream_id}")))?;
        let current = DreamStatus::try_from(current.as_str())?;
        if !dream_transition_allowed(current, target) {
            return Err(LedgerError::InvalidTransition(format!(
                "dream {dream_id} cannot move from {} to {}",
                current.as_str(),
                target.as_str()
            )));
        }
        self.conn.execute(
            "UPDATE dream_contracts SET status = ?2, updated_at = datetime('now') WHERE id = ?1",
            params![dream_id, target.as_str()],
        )?;
        Ok(())
    }

    pub fn set_dream_mode(
        &self,
        initiative_id: &str,
        target: InitiativeMode,
        actor: Role,
    ) -> Result<Initiative> {
        if actor != Role::Human {
            return Err(LedgerError::Permission(
                "only a human may raise Dream autonomy".into(),
            ));
        }
        let initiative = self.get_initiative(initiative_id)?;
        if !matches!(
            target,
            InitiativeMode::DreamPrototype | InitiativeMode::DreamIncubator
        ) {
            return Err(LedgerError::Validation(
                "Dream mode may only be raised to prototype or incubator".into(),
            ));
        }
        if !matches!(
            initiative.mode,
            InitiativeMode::DreamIdeation
                | InitiativeMode::DreamPrototype
                | InitiativeMode::DreamIncubator
        ) {
            return Err(LedgerError::Permission(
                "initiative is not governed by a Dream mandate".into(),
            ));
        }
        let mandate_id = initiative.standing_mandate_id.as_deref().ok_or_else(|| {
            LedgerError::Binding("Dream initiative has no mandate binding".into())
        })?;
        self.require_enabled_mandate(
            &initiative.session_id,
            &initiative.repo_root,
            mandate_id,
            target,
        )?;
        let autonomy_level = if target == InitiativeMode::DreamPrototype {
            1
        } else {
            2
        };
        self.conn.execute(
            "UPDATE initiatives SET mode=?2, autonomy_level=?3, updated_at=datetime('now') WHERE id=?1",
            params![initiative_id, target.as_str(), autonomy_level],
        )?;
        self.record_event(OrchestrationEvent {
            id: new_id("EVENT"),
            initiative_id: initiative_id.into(),
            task_id: None,
            actor_role: actor,
            kind: "dream.autonomy_raised".into(),
            requirement_ids: vec![],
            adr_ids: vec![],
            assumption_ids: vec![],
            features: BTreeMap::from([
                ("autonomyLevel".into(), autonomy_level as f64),
                ("humanApproved".into(), 1.0),
            ]),
            provenance: "local-user-control".into(),
            redacted_summary: format!("Human approved Dream mode {}", target.as_str()),
            created_at: None,
        })?;
        self.get_initiative(initiative_id)
    }

    pub fn record_event(&self, event: OrchestrationEvent) -> Result<String> {
        if event.redacted_summary.chars().count() > 1000 {
            return Err(LedgerError::Validation(
                "event summary exceeds 1000 characters".into(),
            ));
        }
        self.get_initiative(&event.initiative_id)?;
        self.conn.execute(
            "INSERT INTO orchestration_events
             (id, initiative_id, task_id, actor_role, kind, requirement_ids_json, adr_ids_json,
              assumption_ids_json, features_json, provenance, redacted_summary)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                event.id,
                event.initiative_id,
                event.task_id,
                event.actor_role.as_str(),
                event.kind,
                serde_json::to_string(&event.requirement_ids)?,
                serde_json::to_string(&event.adr_ids)?,
                serde_json::to_string(&event.assumption_ids)?,
                serde_json::to_string(&event.features)?,
                event.provenance,
                event.redacted_summary
            ],
        )?;
        Ok(event.id)
    }

    pub fn list_events(
        &self,
        initiative_id: &str,
        limit: usize,
    ) -> Result<Vec<OrchestrationEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, initiative_id, task_id, actor_role, kind, requirement_ids_json,
                    adr_ids_json, assumption_ids_json, features_json, provenance,
                    redacted_summary, created_at
             FROM orchestration_events WHERE initiative_id = ?1
             ORDER BY created_at DESC, rowid DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![initiative_id, limit.min(1000) as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
            ))
        })?;
        rows.map(|row| {
            let row = row?;
            Ok(OrchestrationEvent {
                id: row.0,
                initiative_id: row.1,
                task_id: row.2,
                actor_role: Role::try_from(row.3.as_str())?,
                kind: row.4,
                requirement_ids: serde_json::from_str(&row.5)?,
                adr_ids: serde_json::from_str(&row.6)?,
                assumption_ids: serde_json::from_str(&row.7)?,
                features: serde_json::from_str(&row.8)?,
                provenance: row.9,
                redacted_summary: row.10,
                created_at: Some(row.11),
            })
        })
        .collect()
    }

    pub fn generate_proof_report(&self, initiative_id: &str) -> Result<ProofReport> {
        let initiative = self.get_initiative(initiative_id)?;
        let objectives = self.json_rows(
            "SELECT payload_json FROM objectives WHERE initiative_id = ?1 AND spec_version = ?2",
            initiative_id,
            initiative.active_spec_version,
        )?;
        let assumptions = self.json_rows(
            "SELECT payload_json FROM assumptions WHERE initiative_id = ?1 AND spec_version = ?2",
            initiative_id,
            initiative.active_spec_version,
        )?;
        let constraints = self.json_rows(
            "SELECT payload_json FROM constraints WHERE initiative_id = ?1 AND spec_version = ?2",
            initiative_id,
            initiative.active_spec_version,
        )?;
        let requirements = self.json_rows(
            "SELECT json_object('id', id, 'status', status, 'payload', json(payload_json))
             FROM requirements WHERE initiative_id = ?1 AND spec_version = ?2",
            initiative_id,
            initiative.active_spec_version,
        )?;
        let adrs = self.json_rows(
            "SELECT payload_json FROM architecture_decisions WHERE initiative_id = ?1 AND spec_version = ?2",
            initiative_id,
            initiative.active_spec_version,
        )?;
        let ux = self.json_rows(
            "SELECT contract_json FROM ux_contracts WHERE initiative_id = ?1 AND spec_version = ?2",
            initiative_id,
            initiative.active_spec_version,
        )?;
        let tasks = self.json_rows(
            "SELECT json_object('id', id, 'status', status, 'payload', json(payload_json))
             FROM studio_tasks WHERE initiative_id = ?1 AND spec_version = ?2",
            initiative_id,
            initiative.active_spec_version,
        )?;
        let evidence = self.json_rows_any_version(
            "SELECT json_object('id', id, 'requirementId', requirement_id, 'type', evidence_type,
                                'status', status, 'provenance', provenance, 'summary', summary)
             FROM verification_evidence WHERE initiative_id = ?1",
            initiative_id,
        )?;
        let operations = self.json_rows_any_version(
            "SELECT json_object('operationId', operation_id, 'taskId', task_id,
                                'specVersion', spec_version,
                                'requirementIds', json(requirement_ids_json),
                                'adrIds', json(adr_ids_json),
                                'contextBundleId', context_bundle_id,
                                'sha256', operation_sha256, 'createdAt', created_at)
             FROM operation_links WHERE initiative_id = ?1 ORDER BY created_at, rowid",
            initiative_id,
        )?;

        let mut complete = vec![];
        let mut incomplete = vec![];
        let mut blocked = vec![];
        let mut unverified = vec![];
        let mut outcome_pending = vec![];
        let mut stmt = self.conn.prepare(
            "SELECT id, status FROM requirements WHERE initiative_id = ?1 AND spec_version = ?2",
        )?;
        for row in stmt.query_map(
            params![initiative_id, initiative.active_spec_version],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )? {
            let (id, status) = row?;
            match status.as_str() {
                "verified" | "outcome_pending" | "outcome_confirmed" => complete.push(id.clone()),
                "blocked" | "failed" => blocked.push(id.clone()),
                "proposed" | "approved" | "implementation_started" | "implemented" => {
                    incomplete.push(id.clone())
                }
                "verification_pending" | "repairing" => unverified.push(id.clone()),
                _ => {}
            }
            if status == "outcome_pending" {
                outcome_pending.push(id);
            }
        }
        let remaining_risks = self
            .json_rows_any_version(
                "SELECT payload_json FROM artifacts WHERE initiative_id = ?1 AND artifact_type IN ('finding','review_verdict')",
                initiative_id,
            )?
            .into_iter()
            .filter_map(|v| v.get("summary").and_then(Value::as_str).map(str::to_string))
            .collect();
        Ok(ProofReport {
            id: new_id("PROOF"),
            initiative_id: initiative_id.into(),
            spec_version: initiative.active_spec_version,
            objective: objectives,
            assumptions,
            constraints,
            requirements,
            architecture_decisions: adrs,
            ux_contracts: ux,
            tasks,
            evidence,
            operations,
            complete,
            incomplete,
            blocked,
            unverified,
            outcome_pending,
            remaining_risks,
            excludes_sensitive_context: true,
        })
    }

    pub fn persist_proof_report(&self, report: &ProofReport) -> Result<String> {
        let bytes = serde_json::to_vec(report)?;
        self.conn.execute(
            "INSERT INTO proof_reports (id, initiative_id, spec_version, report_json, content_sha256)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                report.id,
                report.initiative_id,
                report.spec_version,
                String::from_utf8_lossy(&bytes),
                sha256(&bytes)
            ],
        )?;
        Ok(report.id.clone())
    }

    pub fn workspace_snapshot(&self, initiative_id: &str) -> Result<Value> {
        let initiative = self.get_initiative(initiative_id)?;
        let report = self.generate_proof_report(initiative_id)?;
        let dreams = self.json_rows_any_version(
            "SELECT json_object('id', id, 'horizon', horizon, 'status', status, 'payload', json(payload_json))
             FROM dream_contracts WHERE initiative_id = ?1 ORDER BY created_at DESC",
            initiative_id,
        )?;
        let artifacts = self.json_rows_any_version(
            "SELECT json_object('id', id, 'role', role, 'artifactType', artifact_type,
                                'summary', redacted_summary, 'createdAt', created_at)
             FROM artifacts WHERE initiative_id = ?1 ORDER BY created_at DESC LIMIT 200",
            initiative_id,
        )?;
        let ux_documents = self.json_rows(
            "SELECT json_object('id', id, 'status', status, 'contract', json(contract_json),
                                'prototype', json(prototype_json))
             FROM ux_contracts WHERE initiative_id = ?1 AND spec_version = ?2 ORDER BY created_at",
            initiative_id,
            initiative.active_spec_version,
        )?;
        let architecture = self.json_rows(
            "SELECT json_object('id', id, 'status', status, 'decision', json(payload_json))
             FROM architecture_decisions WHERE initiative_id = ?1 AND spec_version = ?2 ORDER BY created_at",
            initiative_id,
            initiative.active_spec_version,
        )?;
        let questions = self.json_rows_any_version(
            "SELECT json_object('id', id, 'fromRole', from_role, 'toRole', to_role,
                                'blocking', blocking, 'status', status, 'payload', json(payload_json))
             FROM alignment_questions WHERE initiative_id = ?1 ORDER BY created_at DESC",
            initiative_id,
        )?;
        let agent_runs = self.json_rows_any_version(
            "SELECT json_object('id', id, 'taskId', task_id, 'specVersion', spec_version,
                                'role', role, 'runtime', runtime, 'model', model,
                                'profileVersion', profile_version,
                                'contextBundleId', context_bundle_id, 'status', status,
                                'parseResult', parse_result, 'errorSummary', error_summary,
                                'startedAt', started_at, 'endedAt', ended_at)
             FROM agent_runs WHERE initiative_id = ?1 ORDER BY started_at DESC, rowid DESC LIMIT 300",
            initiative_id,
        )?;
        let context_capsules = self.json_rows_any_version(
            "SELECT payload_json FROM context_capsules
             WHERE initiative_id = ?1 ORDER BY created_at DESC, rowid DESC LIMIT 100",
            initiative_id,
        )?;
        let worktree: Option<Value> = self
            .conn
            .query_row(
                "SELECT json_object('id', id, 'path', worktree_path, 'branch', branch_name,
                                    'baseCommit', base_commit, 'status', status)
                 FROM governed_worktrees WHERE initiative_id = ?1",
                [initiative_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .map(|s| serde_json::from_str(&s))
            .transpose()?;
        Ok(json!({
            "initiative": initiative,
            "proof": report,
            "dreams": dreams,
            "artifacts": artifacts,
            "uxDocuments": ux_documents,
            "architecture": architecture,
            "questions": questions,
            "agentRuns": agent_runs,
            "contextCapsules": context_capsules,
            "timeline": self.list_events(initiative_id, 300)?,
            "worktree": worktree
        }))
    }

    fn require_enabled_mandate(
        &self,
        session_id: &str,
        repo_root: &str,
        mandate_id: &str,
        mode: InitiativeMode,
    ) -> Result<Mandate> {
        let payload: String = self
            .conn
            .query_row(
                "SELECT payload_json FROM standing_mandates
                 WHERE id = ?1 AND session_id = ?2 AND repo_root = ?3 AND enabled = 1
                       AND approved_by_source = 'local-user'",
                params![mandate_id, session_id, repo_root],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| {
                LedgerError::Permission("enabled repository-bound mandate not found".into())
            })?;
        let mandate: Mandate = serde_json::from_str(&payload)?;
        mandate.validate()?;
        if !mandate.allowed_modes.contains(&mode) {
            return Err(LedgerError::Permission(format!(
                "mandate does not allow mode {}",
                mode.as_str()
            )));
        }
        Ok(mandate)
    }

    fn require_active_spec(&self, initiative_id: &str, version: i64) -> Result<()> {
        let initiative = self.get_initiative(initiative_id)?;
        if initiative.active_spec_version != version {
            return Err(LedgerError::Binding(format!(
                "stale spec v{version}; active version is v{}",
                initiative.active_spec_version
            )));
        }
        Ok(())
    }

    fn require_mutable_spec(&self, initiative_id: &str, version: i64) -> Result<()> {
        self.require_active_spec(initiative_id, version)?;
        let status: String = self.conn.query_row(
            "SELECT status FROM spec_versions WHERE initiative_id = ?1 AND version = ?2",
            params![initiative_id, version],
            |row| row.get(0),
        )?;
        if status != "draft" {
            return Err(LedgerError::Immutable(format!(
                "spec {initiative_id} v{version} is frozen"
            )));
        }
        Ok(())
    }

    fn require_evidence(&self, requirement: &Requirement) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "SELECT evidence_type FROM verification_evidence
             WHERE requirement_id = ?1 AND initiative_id = ?2 AND status = 'passed'",
        )?;
        let present: BTreeSet<String> = stmt
            .query_map(params![requirement.id, requirement.initiative_id], |row| {
                row.get(0)
            })?
            .collect::<std::result::Result<_, _>>()?;
        let missing: Vec<String> = requirement
            .required_evidence
            .iter()
            .filter(|kind| !present.contains(*kind))
            .cloned()
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(LedgerError::MissingEvidence(missing.join(", ")))
        }
    }

    fn require_completion_evidence(&self, initiative_id: &str, spec_version: i64) -> Result<()> {
        let not_complete: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM requirements WHERE initiative_id = ?1 AND spec_version = ?2
             AND status NOT IN ('verified', 'outcome_pending', 'outcome_confirmed')",
            params![initiative_id, spec_version],
            |row| row.get(0),
        )?;
        let task_failures: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM studio_tasks WHERE initiative_id = ?1 AND spec_version = ?2
             AND status != 'passed'",
            params![initiative_id, spec_version],
            |row| row.get(0),
        )?;
        let proof_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM proof_reports WHERE initiative_id = ?1 AND spec_version = ?2",
            params![initiative_id, spec_version],
            |row| row.get(0),
        )?;
        if not_complete == 0 && task_failures == 0 && proof_count > 0 {
            Ok(())
        } else {
            Err(LedgerError::MissingEvidence(format!(
                "{not_complete} requirement(s) incomplete, {task_failures} task(s) not passed, proof report present={}",
                proof_count > 0
            )))
        }
    }

    fn record_transition_event(
        &self,
        initiative_id: &str,
        actor: Role,
        from: InitiativeStatus,
        to: InitiativeStatus,
        reason: &str,
    ) -> Result<()> {
        self.record_event(OrchestrationEvent {
            id: new_id("EVENT"),
            initiative_id: initiative_id.into(),
            task_id: None,
            actor_role: actor,
            kind: "initiative.transitioned".into(),
            requirement_ids: vec![],
            adr_ids: vec![],
            assumption_ids: vec![],
            features: BTreeMap::new(),
            provenance: "trusted-backend".into(),
            redacted_summary: format!(
                "{} → {}: {}",
                from.as_str(),
                to.as_str(),
                truncate(reason, 500)
            ),
            created_at: None,
        })?;
        Ok(())
    }

    fn json_rows(&self, sql: &str, id: &str, version: i64) -> Result<Vec<Value>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params![id, version], |row| row.get::<_, String>(0))?;
        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }

    fn json_rows_any_version(&self, sql: &str, id: &str) -> Result<Vec<Value>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([id], |row| row.get::<_, String>(0))?;
        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }
}

pub fn initiative_transition_allowed(from: InitiativeStatus, to: InitiativeStatus) -> bool {
    use InitiativeStatus::*;
    if to == Paused && !matches!(from, Completed | Abandoned | Failed | Paused) {
        return true;
    }
    if matches!(to, Abandoned | Failed) && !matches!(from, Completed | Abandoned | Failed) {
        return true;
    }
    matches!(
        (from, to),
        (Created, Discovery)
            | (Discovery, Concepting)
            | (Discovery, Blocked)
            | (Concepting, Challenging)
            | (Concepting, AwaitingScopeApproval)
            | (Challenging, UxDesign)
            | (Challenging, Blocked)
            | (UxDesign, Architecture)
            | (Architecture, Planning)
            | (Planning, AwaitingScopeApproval)
            | (AwaitingScopeApproval, Implementing)
            | (AwaitingScopeApproval, Concepting)
            | (Implementing, Verifying)
            | (Implementing, Blocked)
            | (Verifying, Reviewing)
            | (Verifying, Implementing)
            | (Reviewing, Implementing)
            | (Reviewing, Planning)
            | (Reviewing, AwaitingMergeReview)
            | (Reviewing, Blocked)
            | (AwaitingMergeReview, Completed)
            | (AwaitingMergeReview, Implementing)
            | (Blocked, Planning)
            | (Blocked, Implementing)
            | (Blocked, Verifying)
    )
}

pub fn requirement_transition_allowed(from: RequirementStatus, to: RequirementStatus) -> bool {
    use RequirementStatus::*;
    matches!(
        (from, to),
        (Proposed, Approved)
            | (Proposed, Rejected)
            | (Approved, ImplementationStarted)
            | (Approved, Superseded)
            | (ImplementationStarted, Implemented)
            | (ImplementationStarted, Blocked)
            | (ImplementationStarted, Failed)
            | (Implemented, VerificationPending)
            | (VerificationPending, Verified)
            | (VerificationPending, Repairing)
            | (VerificationPending, Blocked)
            | (Repairing, ImplementationStarted)
            | (Verified, OutcomePending)
            | (OutcomePending, OutcomeConfirmed)
            | (OutcomePending, OutcomeDisproven)
            | (Blocked, ImplementationStarted)
            | (Failed, Repairing)
    )
}

pub fn dream_transition_allowed(from: DreamStatus, to: DreamStatus) -> bool {
    use DreamStatus::*;
    matches!(
        (from, to),
        (Proposed, Deduplicated)
            | (Proposed, Challenged)
            | (Proposed, Rejected)
            | (Proposed, Archived)
            | (Proposed, PrototypeApproved)
            | (Proposed, PromotedToGoal)
            | (Deduplicated, Challenged)
            | (Deduplicated, Archived)
            | (Challenged, Rejected)
            | (Challenged, Shortlisted)
            | (Challenged, Archived)
            | (Challenged, PrototypeApproved)
            | (Challenged, PromotedToGoal)
            | (Shortlisted, PrototypeApproved)
            | (Shortlisted, Rejected)
            | (Shortlisted, PromotedToGoal)
            | (PrototypeApproved, Prototyping)
            | (Prototyping, Validated)
            | (Prototyping, Rejected)
            | (Validated, PromotedToGoal)
            | (Validated, Archived)
    )
}

pub fn validate_role_artifact_permission(role: Role, artifact_type: &str) -> Result<()> {
    let allowed: &[&str] = match role {
        Role::Dreamer => &["dream_contract", "belief", "question", "finding"],
        Role::Fde => &[
            "fde_brief",
            "objective",
            "outcome_hypothesis",
            "assumption_register",
            "constraint_set",
            "non_goal_set",
            "opportunity_decision",
            "belief",
            "question",
            "finding",
        ],
        Role::UxDesigner => &[
            "ux_contract",
            "declarative_prototype",
            "ux_conformance",
            "belief",
            "question",
            "finding",
        ],
        Role::Skeptic => &[
            "finding",
            "disconfirmation_experiment",
            "review_verdict",
            "belief",
            "question",
        ],
        Role::Architect => &[
            "architecture_alternatives",
            "adr",
            "architecture_conformance",
            "belief",
            "question",
            "finding",
        ],
        Role::Planner => &[
            "implementation_spec",
            "requirement_set",
            "task_graph",
            "validation_plan",
            "belief",
            "question",
            "finding",
        ],
        Role::Builder => &["patch_proposal", "belief", "question", "finding"],
        Role::Verifier => &[
            "verification_evidence",
            "verification_verdict",
            "belief",
            "question",
            "finding",
        ],
        Role::Reviewer => &["review_verdict", "belief", "question", "finding"],
        Role::Human | Role::System => &["*"],
    };
    if allowed.contains(&"*") || allowed.contains(&artifact_type) {
        Ok(())
    } else {
        Err(LedgerError::Permission(format!(
            "role {} cannot publish artifact type {artifact_type}",
            role.as_str()
        )))
    }
}

pub fn validate_task_scope(payload: &Value) -> Result<()> {
    for field in ["allowedPaths", "expectedFiles", "forbiddenPaths"] {
        let Some(paths) = payload.get(field).and_then(Value::as_array) else {
            return Err(LedgerError::Validation(format!(
                "task payload requires {field} array"
            )));
        };
        for path in paths {
            let path = path.as_str().ok_or_else(|| {
                LedgerError::Validation(format!("{field} entries must be strings"))
            })?;
            let valid = if field == "forbiddenPaths" {
                safe_forbidden_path(path)
            } else {
                safe_relative_path(path)
            };
            if !valid {
                return Err(LedgerError::Validation(format!(
                    "unsafe task path in {field}: {path}"
                )));
            }
        }
    }
    Ok(())
}

fn safe_forbidden_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    !normalized.is_empty()
        && !normalized.starts_with('/')
        && !(normalized.len() > 1 && normalized.as_bytes()[1] == b':')
        && !normalized
            .split('/')
            .any(|part| part == ".." || part.is_empty())
}

pub fn safe_relative_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    !normalized.is_empty()
        && !normalized.starts_with('/')
        && !(normalized.len() > 1 && normalized.as_bytes()[1] == b':')
        && !normalized
            .split('/')
            .any(|part| part == ".." || part.is_empty())
        && !normalized.starts_with(".git/")
        && normalized != ".git"
        && !normalized.starts_with(".synthesize/")
        && normalized != ".synthesize"
}

pub fn validate_dream_contract(payload: &Value) -> Result<()> {
    for field in [
        "title",
        "horizon",
        "problemObserved",
        "proposedFuture",
        "smallestExperiment",
        "estimatedCost",
        "reversibility",
        "noveltyRationale",
        "confidence",
    ] {
        if payload.get(field).is_none() {
            return Err(LedgerError::Validation(format!(
                "Dream Contract missing {field}"
            )));
        }
    }
    let horizon = payload
        .get("horizon")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !matches!(horizon, "adjacent" | "strategic" | "moonshot") {
        return Err(LedgerError::Validation("invalid Dream horizon".into()));
    }
    let confidence = payload
        .get("confidence")
        .and_then(Value::as_f64)
        .unwrap_or(-1.0);
    if !(0.0..=1.0).contains(&confidence) {
        return Err(LedgerError::Validation(
            "Dream confidence must be within 0..=1".into(),
        ));
    }
    for array in [
        "supportingEvidence",
        "expectedValue",
        "counterarguments",
        "assumptions",
    ] {
        if !payload.get(array).is_some_and(Value::is_array) {
            return Err(LedgerError::Validation(format!(
                "Dream Contract {array} must be an array"
            )));
        }
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn dream_fingerprint(payload: &Value) -> String {
    let normalized = ["title", "problemObserved", "proposedFuture"]
        .iter()
        .filter_map(|field| payload.get(*field).and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("|")
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    sha256(normalized.as_bytes())
}

fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use audit_log::init_schema;

    fn database() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO sessions (id, repo_root) VALUES ('session', '/repo')",
            [],
        )
        .unwrap();
        conn
    }

    fn studio(conn: &Connection) -> Initiative {
        Ledger::new(conn)
            .create_initiative(
                "session",
                "/repo",
                "Build safe studio",
                InitiativeMode::Studio,
                "user_prompt",
                None,
            )
            .unwrap()
    }

    #[test]
    fn initiative_transitions_are_backend_validated() {
        let conn = database();
        let initiative = studio(&conn);
        let ledger = Ledger::new(&conn);
        assert!(ledger
            .transition_initiative(
                &initiative.id,
                InitiativeStatus::Implementing,
                Role::Human,
                "skip ahead"
            )
            .is_err());
        assert!(ledger
            .transition_initiative(
                &initiative.id,
                InitiativeStatus::Discovery,
                Role::Builder,
                "model asks"
            )
            .is_err());
        let discovery = ledger
            .transition_initiative(
                &initiative.id,
                InitiativeStatus::Discovery,
                Role::Human,
                "start",
            )
            .unwrap();
        assert_eq!(discovery.status, InitiativeStatus::Discovery);
    }

    #[test]
    fn pause_and_resume_preserve_state() {
        let conn = database();
        let initiative = studio(&conn);
        let ledger = Ledger::new(&conn);
        ledger
            .transition_initiative(
                &initiative.id,
                InitiativeStatus::Discovery,
                Role::Human,
                "start",
            )
            .unwrap();
        let paused = ledger
            .transition_initiative(
                &initiative.id,
                InitiativeStatus::Paused,
                Role::Human,
                "break",
            )
            .unwrap();
        assert_eq!(paused.resume_status, Some(InitiativeStatus::Discovery));
        assert_eq!(
            ledger
                .resume_initiative(&initiative.id, "continue")
                .unwrap()
                .status,
            InitiativeStatus::Discovery
        );
    }

    #[test]
    fn frozen_specs_are_immutable_and_replans_create_versions() {
        let conn = database();
        let initiative = studio(&conn);
        let ledger = Ledger::new(&conn);
        ledger
            .freeze_spec(&initiative.id, 1, &json!({"goal": "v1"}))
            .unwrap();
        assert!(ledger
            .freeze_spec(&initiative.id, 1, &json!({"goal": "changed"}))
            .is_err());
        let version = ledger
            .create_spec_version(
                &initiative.id,
                "assumption invalidated",
                &json!({"goal": "v2"}),
            )
            .unwrap();
        assert_eq!(version, 2);
    }

    #[test]
    fn requirement_cannot_verify_without_all_evidence() {
        let conn = database();
        let initiative = studio(&conn);
        let ledger = Ledger::new(&conn);
        let req = Requirement {
            id: "REQ-1".into(),
            initiative_id: initiative.id.clone(),
            spec_version: 1,
            status: RequirementStatus::Proposed,
            required_evidence: vec!["unit_test".into(), "security_review".into()],
            payload: json!({"description": "enforce evidence"}),
        };
        ledger.add_requirement(&req).unwrap();
        for state in [
            RequirementStatus::Approved,
            RequirementStatus::ImplementationStarted,
            RequirementStatus::Implemented,
            RequirementStatus::VerificationPending,
        ] {
            ledger.transition_requirement("REQ-1", state).unwrap();
        }
        assert!(ledger
            .transition_requirement("REQ-1", RequirementStatus::Verified)
            .is_err());
        for evidence_type in ["unit_test", "security_review"] {
            ledger
                .record_evidence(
                    &initiative.id,
                    &EvidenceInput {
                        requirement_id: "REQ-1".into(),
                        task_id: None,
                        evidence_type: evidence_type.into(),
                        status: "passed".into(),
                        provenance: "cargo test".into(),
                        output_ref: None,
                        summary: "passed".into(),
                        content_sha256: None,
                    },
                )
                .unwrap();
        }
        assert_eq!(
            ledger
                .transition_requirement("REQ-1", RequirementStatus::Verified)
                .unwrap()
                .status,
            RequirementStatus::Verified
        );
    }

    #[test]
    fn task_verdicts_route_and_budget_revisions() {
        let conn = database();
        let initiative = studio(&conn);
        let ledger = Ledger::new(&conn);
        let task = StudioTask {
            id: "TASK-1".into(),
            initiative_id: initiative.id,
            spec_version: 1,
            status: TaskStatus::Reviewing,
            assigned_role: Role::Builder,
            iteration_count: 0,
            max_iterations: 1,
            payload: json!({"allowedPaths":["src/a.rs"],"expectedFiles":["src/a.rs"],"forbiddenPaths":["secrets.txt"]}),
        };
        ledger.add_task(&task).unwrap();
        let revised = ledger
            .route_review_verdict("TASK-1", ReviewerVerdict::Revise)
            .unwrap();
        assert_eq!(revised.status, TaskStatus::Revising);
        conn.execute(
            "UPDATE studio_tasks SET status='reviewing' WHERE id='TASK-1'",
            [],
        )
        .unwrap();
        assert!(ledger
            .route_review_verdict("TASK-1", ReviewerVerdict::Revise)
            .is_err());
        assert_eq!(
            ledger.get_task("TASK-1").unwrap().status,
            TaskStatus::Blocked
        );
    }

    #[test]
    fn role_permissions_and_bindings_reject_forgery() {
        let conn = database();
        let initiative = studio(&conn);
        let ledger = Ledger::new(&conn);
        let envelope = ArtifactEnvelope {
            operation_id: "OP-1".into(),
            initiative_id: initiative.id,
            task_id: None,
            role: Role::Builder,
            artifact_type: "adr".into(),
            schema_version: 1,
            spec_version: 1,
            source_context_bundle_id: None,
            reason: "choose architecture".into(),
            expected_outcome: "ADR".into(),
            payload: json!({}),
        };
        assert!(ledger
            .publish_artifact(&envelope, None, "forged role")
            .is_err());
    }

    #[test]
    fn dream_requires_human_approved_enabled_mandate() {
        let conn = database();
        let ledger = Ledger::new(&conn);
        assert!(ledger
            .create_initiative(
                "session",
                "/repo",
                "Dream",
                InitiativeMode::DreamIdeation,
                "dream_cycle",
                Some("MANDATE-1")
            )
            .is_err());
        let mandate = Mandate {
            id: "MANDATE-1".into(),
            name: "Local exploration".into(),
            purpose: "reversible ideas".into(),
            allowed_modes: vec![InitiativeMode::DreamIdeation],
            allowed_repo_paths: vec![],
            maximum_candidates_per_cycle: 10,
            maximum_prototypes_per_cycle: 2,
            maximum_builder_iterations: 8,
            maximum_changed_files: 20,
            maximum_elapsed_minutes: 240,
            network_policy: "disabled".into(),
            package_install_policy: "forbidden".into(),
            active_branch_write_policy: "forbidden".into(),
            merge_authority: "human_only".into(),
            enabled: true,
        };
        ledger
            .upsert_mandate("session", "/repo", &mandate, "local-user")
            .unwrap();
        assert!(ledger
            .create_initiative(
                "session",
                "/repo",
                "Dream",
                InitiativeMode::DreamIdeation,
                "dream_cycle",
                Some("MANDATE-1")
            )
            .is_ok());
    }

    #[test]
    fn unsafe_task_paths_are_rejected() {
        assert!(!safe_relative_path("../escape"));
        assert!(!safe_relative_path("C:\\escape"));
        assert!(!safe_relative_path(".git/config"));
        assert!(safe_relative_path("src/studio/mod.rs"));
    }

    #[test]
    fn dream_contract_validation_requires_counterarguments() {
        let contract = json!({
            "title":"Atlas","horizon":"strategic","problemObserved":"rediscovery",
            "proposedFuture":"evidence graph","supportingEvidence":[],"expectedValue":[],
            "counterarguments":[],"assumptions":[],"smallestExperiment":"render graph",
            "estimatedCost":"medium","reversibility":"high","noveltyRationale":"linked proof",
            "confidence":0.58
        });
        assert!(validate_dream_contract(&contract).is_ok());
    }

    #[test]
    fn artifacts_are_bound_to_unique_hashed_operations() {
        let conn = database();
        let initiative = studio(&conn);
        let ledger = Ledger::new(&conn);
        let envelope = ArtifactEnvelope {
            operation_id: "OP-BOUND-1".into(),
            initiative_id: initiative.id.clone(),
            task_id: None,
            role: Role::Architect,
            artifact_type: "adr".into(),
            schema_version: 1,
            spec_version: 1,
            source_context_bundle_id: None,
            reason: "record the approved boundary".into(),
            expected_outcome: "traceable decision".into(),
            payload: json!({"requirementIds":["REQ-1"],"adrIds":["ADR-1"]}),
        };
        ledger
            .publish_artifact(&envelope, None, "ADR published")
            .unwrap();
        assert!(ledger.publish_artifact(&envelope, None, "replay").is_err());
        let proof = ledger.generate_proof_report(&initiative.id).unwrap();
        assert_eq!(proof.operations.len(), 1);
        assert_eq!(proof.operations[0]["operationId"], "OP-BOUND-1");
        assert_eq!(proof.operations[0]["requirementIds"][0], "REQ-1");
        assert_eq!(proof.operations[0]["adrIds"][0], "ADR-1");
        assert_eq!(proof.operations[0]["sha256"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn dream_dedup_budget_and_autonomy_are_backend_enforced() {
        let conn = database();
        let ledger = Ledger::new(&conn);
        let mandate = Mandate {
            id: "MANDATE-GOVERNED".into(),
            name: "Governed exploration".into(),
            purpose: "bounded prototypes".into(),
            allowed_modes: vec![
                InitiativeMode::DreamIdeation,
                InitiativeMode::DreamPrototype,
                InitiativeMode::DreamIncubator,
            ],
            allowed_repo_paths: vec!["src/experiments".into()],
            maximum_candidates_per_cycle: 1,
            maximum_prototypes_per_cycle: 1,
            maximum_builder_iterations: 1,
            maximum_changed_files: 2,
            maximum_elapsed_minutes: 30,
            network_policy: "disabled".into(),
            package_install_policy: "forbidden".into(),
            active_branch_write_policy: "forbidden".into(),
            merge_authority: "human_only".into(),
            enabled: true,
        };
        ledger
            .upsert_mandate("session", "/repo", &mandate, "local-user")
            .unwrap();
        let initiative = ledger
            .create_initiative(
                "session",
                "/repo",
                "Dream",
                InitiativeMode::DreamIdeation,
                "dream_cycle",
                Some(&mandate.id),
            )
            .unwrap();
        let contract = json!({
            "title":"Atlas","horizon":"strategic","problemObserved":"rediscovery",
            "proposedFuture":"evidence graph","supportingEvidence":[],"expectedValue":[],
            "counterarguments":[],"assumptions":[],"smallestExperiment":"render graph",
            "estimatedCost":"medium","reversibility":"high","noveltyRationale":"linked proof",
            "confidence":0.58
        });
        let dream_id = ledger.create_dream(&initiative.id, &contract).unwrap();
        assert_eq!(
            ledger.create_dream(&initiative.id, &contract).unwrap(),
            dream_id
        );
        let mut second = contract.clone();
        second["title"] = json!("Different candidate");
        assert!(ledger.create_dream(&initiative.id, &second).is_err());
        assert!(ledger
            .set_dream_mode(
                &initiative.id,
                InitiativeMode::DreamPrototype,
                Role::Dreamer
            )
            .is_err());
        let raised = ledger
            .set_dream_mode(&initiative.id, InitiativeMode::DreamPrototype, Role::Human)
            .unwrap();
        assert_eq!(raised.mode, InitiativeMode::DreamPrototype);
        assert_eq!(raised.autonomy_level, 1);
    }

    #[test]
    fn beliefs_and_questions_remain_explicit_and_answerable() {
        let conn = database();
        let initiative = studio(&conn);
        let ledger = Ledger::new(&conn);
        let belief_id = ledger
            .publish_belief(
                &initiative.id,
                None,
                1,
                None,
                Role::Verifier,
                &json!({"requirementComplete":{"REQ-1":false},"confidence":0.8}),
            )
            .unwrap();
        let question = AlignmentQuestion {
            id: "QUESTION-1".into(),
            initiative_id: initiative.id.clone(),
            task_id: None,
            from_role: Role::Builder,
            to_role: Role::Architect,
            reason: "ADR boundary is ambiguous".into(),
            question: "Which process owns retries?".into(),
            blocking: true,
            status: "open".into(),
        };
        ledger.open_question(&question).unwrap();
        ledger
            .answer_question(
                &question.id,
                Role::Architect,
                &json!({"answer":"The orchestration process","evidence":[]}),
            )
            .unwrap();
        let proof = ledger.workspace_snapshot(&initiative.id).unwrap();
        assert!(belief_id.starts_with("BELIEF-"));
        assert_eq!(proof["questions"][0]["status"], "answered");
    }
}
