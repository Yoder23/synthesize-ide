use audit_log::new_id;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FactoryError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("factory is not active")]
    Inactive,
    #[error("invalid factory transition: {0}")]
    Transition(String),
}

pub type Result<T> = std::result::Result<T, FactoryError>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FactoryStatus {
    Running,
    Paused,
    StopAfterCurrent,
    Stopped,
    Waiting,
    Failed,
}

impl FactoryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Paused => "paused",
            Self::StopAfterCurrent => "stop_after_current",
            Self::Stopped => "stopped",
            Self::Waiting => "waiting",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FactoryStage {
    DreamPending,
    Dreaming,
    DreamChallengePending,
    DreamChallenging,
    FdePending,
    FdeFraming,
    UxPending,
    UxDesigning,
    ArchitectPending,
    Architecting,
    PlanPending,
    Planning,
    ScopeGate,
    WorktreePending,
    WorktreeCreating,
    TaskSelection,
    TaskBuildPending,
    TaskBuilding,
    PatchPolicyReview,
    TaskValidating,
    TaskVerifyPending,
    TaskVerifying,
    TaskReviewPending,
    TaskReviewing,
    TaskRevising,
    InitiativeReplanning,
    FinalUxReview,
    FinalFdeReview,
    CandidateComplete,
    Rejected,
    Blocked,
    BudgetExhausted,
    Paused,
    Abandoned,
    Failed,
}
impl FactoryStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DreamPending => "dream_pending",
            Self::Dreaming => "dreaming",
            Self::DreamChallengePending => "dream_challenge_pending",
            Self::DreamChallenging => "dream_challenging",
            Self::FdePending => "fde_pending",
            Self::FdeFraming => "fde_framing",
            Self::UxPending => "ux_pending",
            Self::UxDesigning => "ux_designing",
            Self::ArchitectPending => "architect_pending",
            Self::Architecting => "architecting",
            Self::PlanPending => "plan_pending",
            Self::Planning => "planning",
            Self::ScopeGate => "scope_gate",
            Self::WorktreePending => "worktree_pending",
            Self::WorktreeCreating => "worktree_creating",
            Self::TaskSelection => "task_selection",
            Self::TaskBuildPending => "task_build_pending",
            Self::TaskBuilding => "task_building",
            Self::PatchPolicyReview => "patch_policy_review",
            Self::TaskValidating => "task_validating",
            Self::TaskVerifyPending => "task_verify_pending",
            Self::TaskVerifying => "task_verifying",
            Self::TaskReviewPending => "task_review_pending",
            Self::TaskReviewing => "task_reviewing",
            Self::TaskRevising => "task_revising",
            Self::InitiativeReplanning => "initiative_replanning",
            Self::FinalUxReview => "final_ux_review",
            Self::FinalFdeReview => "final_fde_review",
            Self::CandidateComplete => "candidate_complete",
            Self::Rejected => "rejected",
            Self::Blocked => "blocked",
            Self::BudgetExhausted => "budget_exhausted",
            Self::Paused => "paused",
            Self::Abandoned => "abandoned",
            Self::Failed => "failed",
        }
    }
    pub fn terminal(self) -> bool {
        matches!(
            self,
            Self::CandidateComplete
                | Self::Rejected
                | Self::Blocked
                | Self::BudgetExhausted
                | Self::Abandoned
                | Self::Failed
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactoryState {
    pub id: String,
    pub session_id: String,
    pub repo_root: String,
    pub mandate_id: String,
    pub status: FactoryStatus,
    pub current_initiative_id: Option<String>,
    pub completed_dream_count: i64,
    pub stop_after_current: bool,
    pub stage: Option<String>,
    pub active_task_id: Option<String>,
    pub attempt_count: i64,
    pub expected_artifact: Option<String>,
    pub waiting_reason: Option<String>,
}

pub struct DreamFactoryController<'a> {
    conn: &'a Connection,
}
impl<'a> DreamFactoryController<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
    pub fn start_factory(&self, session: &str, repo: &str, mandate: &str) -> Result<FactoryState> {
        let id = new_id("FACTORY");
        self.conn.execute("INSERT INTO dream_factories (id,session_id,repo_root,mandate_id,status) VALUES (?1,?2,?3,?4,'running') ON CONFLICT(session_id,repo_root) DO UPDATE SET mandate_id=excluded.mandate_id,status='running',stop_after_current=0,waiting_reason=NULL,updated_at=datetime('now')",params![id,session,repo,mandate])?;
        self.load(session, repo)
    }
    pub fn pause_factory(&self, session: &str, repo: &str) -> Result<FactoryState> {
        self.set_status(session, repo, FactoryStatus::Paused, false)?;
        self.load(session, repo)
    }
    pub fn resume_factory(&self, session: &str, repo: &str) -> Result<FactoryState> {
        self.set_status(session, repo, FactoryStatus::Running, false)?;
        self.load(session, repo)
    }
    pub fn stop_factory(
        &self,
        session: &str,
        repo: &str,
        after_current: bool,
    ) -> Result<FactoryState> {
        self.set_status(
            session,
            repo,
            if after_current {
                FactoryStatus::StopAfterCurrent
            } else {
                FactoryStatus::Stopped
            },
            after_current,
        )?;
        self.load(session, repo)
    }
    fn set_status(
        &self,
        session: &str,
        repo: &str,
        status: FactoryStatus,
        after: bool,
    ) -> Result<()> {
        self.conn.execute("UPDATE dream_factories SET status=?3,stop_after_current=?4,updated_at=datetime('now') WHERE session_id=?1 AND repo_root=?2",params![session,repo,status.as_str(),i64::from(after)])?;
        Ok(())
    }
    pub fn begin_concept(&self, factory_id: &str, initiative_id: &str) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        let key = format!("{initiative_id}:dream_pending");
        tx.execute("INSERT INTO dream_factory_runs (id,factory_id,initiative_id,stage,expected_artifact,idempotency_key,status) VALUES (?1,?2,?3,'dream_pending','dream_contract',?4,'active') ON CONFLICT(factory_id,idempotency_key) DO NOTHING",params![new_id("FACTORY_RUN"),factory_id,initiative_id,key])?;
        tx.execute("UPDATE dream_factories SET current_initiative_id=?2,status='running',updated_at=datetime('now') WHERE id=?1",params![factory_id,initiative_id])?;
        tx.commit()?;
        Ok(())
    }
    pub fn active_factory_id(&self, session: &str, repo: &str) -> Result<Option<String>> {
        self.conn.query_row("SELECT id FROM dream_factories WHERE session_id=?1 AND repo_root=?2 AND status IN ('running','stop_after_current')", params![session, repo], |row| row.get(0)).optional().map_err(Into::into)
    }
    pub fn set_stage(
        &self,
        factory_id: &str,
        from: FactoryStage,
        to: FactoryStage,
        expected_artifact: Option<&str>,
    ) -> Result<()> {
        self.transition(factory_id, from, to, expected_artifact)
    }
    pub fn select_next_ready_task(
        &self,
        factory_id: &str,
        initiative_id: &str,
    ) -> Result<Option<String>> {
        let task: Option<String> = self.conn.query_row(
            "SELECT t.id FROM studio_tasks t
             WHERE t.initiative_id=?1 AND t.status='ready'
             AND NOT EXISTS (SELECT 1 FROM dream_task_dependencies d JOIN studio_tasks p ON p.id=d.depends_on_task_id WHERE d.task_id=t.id AND p.status!='passed')
             ORDER BY t.id LIMIT 1", [initiative_id], |row| row.get(0)
        ).optional()?;
        if let Some(task_id) = &task {
            let claimed = self.conn.execute("UPDATE studio_tasks SET status='in_progress', updated_at=datetime('now') WHERE id=?1 AND status='ready'", [task_id])?;
            if claimed != 1 {
                return Err(FactoryError::Transition(
                    "ready task could not be claimed".into(),
                ));
            }
            let changed = self.conn.execute("UPDATE dream_factory_runs SET active_task_id=?2,stage='task_build_pending',expected_artifact='patch_proposal',updated_at=datetime('now') WHERE factory_id=?1 AND status='active' AND stage='task_selection'", params![factory_id, task_id])?;
            if changed != 1 {
                return Err(FactoryError::Transition(
                    "task selection was not active".into(),
                ));
            }
        }
        Ok(task)
    }
    pub fn complete_concept(&self, factory_id: &str, terminal: FactoryStage) -> Result<()> {
        if !terminal.terminal() {
            return Err(FactoryError::Transition(
                "completion requires a terminal stage".into(),
            ));
        }
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("UPDATE dream_factory_runs SET stage=?2,status='terminal',completed_at=datetime('now'),updated_at=datetime('now') WHERE factory_id=?1 AND status='active'", params![factory_id, terminal.as_str()])?;
        tx.execute("UPDATE dream_factories SET completed_dream_count=completed_dream_count+1,current_initiative_id=NULL,status=CASE WHEN stop_after_current=1 THEN 'stopped' ELSE 'waiting' END,updated_at=datetime('now') WHERE id=?1", [factory_id])?;
        tx.commit()?;
        Ok(())
    }
    pub fn wait_with_reason(&self, factory_id: &str, reason: &str) -> Result<()> {
        self.conn.execute("UPDATE dream_factories SET status='waiting',waiting_reason=?2,updated_at=datetime('now') WHERE id=?1", params![factory_id, reason])?;
        self.conn.execute("UPDATE dream_factory_runs SET waiting_reason=?2,updated_at=datetime('now') WHERE factory_id=?1 AND status='active'", params![factory_id, reason])?;
        Ok(())
    }
    pub fn next_required_step(&self, session: &str, repo: &str) -> Result<Option<FactoryStage>> {
        let state = self.load(session, repo)?;
        if state.status != FactoryStatus::Running {
            return Ok(None);
        };
        let Some(stage) = state.stage else {
            return Ok(Some(FactoryStage::DreamPending));
        };
        Ok(parse_stage(&stage))
    }
    pub fn transition(
        &self,
        factory_id: &str,
        from: FactoryStage,
        to: FactoryStage,
        expected_artifact: Option<&str>,
    ) -> Result<()> {
        let changed=self.conn.execute("UPDATE dream_factory_runs SET stage=?3,expected_artifact=?4,attempt_count=CASE WHEN ?3='task_revising' THEN attempt_count+1 ELSE attempt_count END,updated_at=datetime('now') WHERE factory_id=?1 AND status='active' AND stage=?2",params![factory_id,from.as_str(),to.as_str(),expected_artifact])?;
        if changed != 1 {
            return Err(FactoryError::Transition(format!(
                "expected active stage {}",
                from.as_str()
            )));
        };
        Ok(())
    }
    pub fn recover_active_runs(&self, session: &str, repo: &str) -> Result<FactoryState> {
        self.conn.execute("UPDATE dream_factory_runs SET stage=CASE stage WHEN 'dreaming' THEN 'dream_pending' WHEN 'dream_challenging' THEN 'dream_challenge_pending' WHEN 'fde_framing' THEN 'fde_pending' WHEN 'ux_designing' THEN 'ux_pending' WHEN 'architecting' THEN 'architect_pending' WHEN 'planning' THEN 'plan_pending' WHEN 'task_building' THEN 'task_build_pending' WHEN 'task_verifying' THEN 'task_verify_pending' WHEN 'task_reviewing' THEN 'task_review_pending' ELSE stage END,waiting_reason='recovered after restart',updated_at=datetime('now') WHERE factory_id IN (SELECT id FROM dream_factories WHERE session_id=?1 AND repo_root=?2) AND status='active'",params![session,repo])?;
        self.load(session, repo)
    }
    pub fn load(&self, session: &str, repo: &str) -> Result<FactoryState> {
        let raw = self.conn.query_row("SELECT f.id,f.session_id,f.repo_root,f.mandate_id,f.status,f.current_initiative_id,f.completed_dream_count,f.stop_after_current,r.stage,r.active_task_id,r.attempt_count,r.expected_artifact,r.waiting_reason FROM dream_factories f LEFT JOIN dream_factory_runs r ON r.factory_id=f.id AND r.status='active' WHERE f.session_id=?1 AND f.repo_root=?2",params![session,repo],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get(5)?,r.get(6)?,r.get::<_,i64>(7)?,r.get(8)?,r.get(9)?,r.get::<_,Option<i64>>(10)?,r.get(11)?,r.get(12)?)))?;
        Ok(FactoryState {
            id: raw.0,
            session_id: raw.1,
            repo_root: raw.2,
            mandate_id: raw.3,
            status: parse_status(&raw.4)?,
            current_initiative_id: raw.5,
            completed_dream_count: raw.6,
            stop_after_current: raw.7 != 0,
            stage: raw.8,
            active_task_id: raw.9,
            attempt_count: raw.10.unwrap_or(0),
            expected_artifact: raw.11,
            waiting_reason: raw.12,
        })
    }
}
fn parse_status(v: &str) -> Result<FactoryStatus> {
    match v {
        "running" => Ok(FactoryStatus::Running),
        "paused" => Ok(FactoryStatus::Paused),
        "stop_after_current" => Ok(FactoryStatus::StopAfterCurrent),
        "stopped" => Ok(FactoryStatus::Stopped),
        "waiting" => Ok(FactoryStatus::Waiting),
        "failed" => Ok(FactoryStatus::Failed),
        _ => Err(FactoryError::Transition(format!(
            "unknown factory status {v}"
        ))),
    }
}
fn parse_stage(v: &str) -> Option<FactoryStage> {
    Some(match v {
        "dream_pending" => FactoryStage::DreamPending,
        "dream_challenge_pending" => FactoryStage::DreamChallengePending,
        "fde_pending" => FactoryStage::FdePending,
        "ux_pending" => FactoryStage::UxPending,
        "architect_pending" => FactoryStage::ArchitectPending,
        "plan_pending" => FactoryStage::PlanPending,
        "scope_gate" => FactoryStage::ScopeGate,
        "worktree_pending" => FactoryStage::WorktreePending,
        "task_selection" => FactoryStage::TaskSelection,
        "task_build_pending" => FactoryStage::TaskBuildPending,
        "task_revising" => FactoryStage::TaskRevising,
        "task_verify_pending" => FactoryStage::TaskVerifyPending,
        "task_review_pending" => FactoryStage::TaskReviewPending,
        "final_ux_review" => FactoryStage::FinalUxReview,
        "final_fde_review" => FactoryStage::FinalFdeReview,
        "candidate_complete" => FactoryStage::CandidateComplete,
        "rejected" => FactoryStage::Rejected,
        "blocked" => FactoryStage::Blocked,
        "budget_exhausted" => FactoryStage::BudgetExhausted,
        "abandoned" => FactoryStage::Abandoned,
        "failed" => FactoryStage::Failed,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use audit_log::init_schema;

    #[test]
    fn controller_persists_cas_transitions_and_terminal_disposition() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
        conn.execute(
            "INSERT INTO sessions (id, repo_root) VALUES ('s','/repo')",
            [],
        )
        .unwrap();
        let controller = DreamFactoryController::new(&conn);
        let state = controller.start_factory("s", "/repo", "mandate").unwrap();
        controller.begin_concept(&state.id, "INIT-1").unwrap();
        assert_eq!(
            controller.next_required_step("s", "/repo").unwrap(),
            Some(FactoryStage::DreamPending)
        );
        controller
            .set_stage(
                &state.id,
                FactoryStage::DreamPending,
                FactoryStage::FdePending,
                Some("fde_brief"),
            )
            .unwrap();
        assert!(controller
            .set_stage(
                &state.id,
                FactoryStage::DreamPending,
                FactoryStage::UxPending,
                Some("ux_contract")
            )
            .is_err());
        controller
            .complete_concept(&state.id, FactoryStage::CandidateComplete)
            .unwrap();
        let terminal = controller.load("s", "/repo").unwrap();
        assert_eq!(terminal.completed_dream_count, 1);
        assert!(terminal.current_initiative_id.is_none());
        assert_eq!(terminal.status, FactoryStatus::Waiting);
    }
}
