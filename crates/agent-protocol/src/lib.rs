use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum AgentOperation {
    #[serde(rename = "read_file")]
    ReadFile { path: String, reason: String },
    #[serde(rename = "search_repo")]
    SearchRepo {
        query: String,
        glob: Option<String>,
        reason: String,
    },
    #[serde(rename = "propose_patch")]
    ProposePatch {
        #[serde(rename = "proposalId")]
        proposal_id: String,
        summary: String,
        #[serde(rename = "baseCommit")]
        base_commit: Option<String>,
        #[serde(rename = "currentCommit")]
        current_commit: Option<String>,
        files: Vec<PatchFile>,
        #[serde(rename = "riskNotes")]
        risk_notes: Vec<String>,
        #[serde(rename = "suggestedCommands")]
        suggested_commands: Vec<CommandRequest>,
    },
    #[serde(rename = "run_command")]
    RunCommand(CommandRequest),
    #[serde(rename = "ask_user")]
    AskUser {
        question: String,
        options: Option<Vec<String>>,
    },
    #[serde(rename = "final_report")]
    FinalReport {
        summary: String,
        #[serde(rename = "changedFiles")]
        changed_files: Vec<String>,
        #[serde(rename = "testsRun")]
        tests_run: Vec<String>,
        #[serde(rename = "remainingRisks")]
        remaining_risks: Vec<String>,
    },
    #[serde(rename = "hand_off")]
    HandOff {
        #[serde(rename = "toSkill")]
        to_skill: String,
        #[serde(rename = "contextSummary")]
        context_summary: String,
        reason: String,
        #[serde(rename = "preserveFileHistory")]
        preserve_file_history: bool,
    },
    #[serde(rename = "propose_artifact")]
    ProposeArtifact {
        #[serde(flatten)]
        header: StudioOperationHeader,
        payload: serde_json::Value,
    },
    #[serde(rename = "publish_belief")]
    PublishBelief {
        #[serde(flatten)]
        header: StudioOperationHeader,
        payload: serde_json::Value,
    },
    #[serde(rename = "ask_agent")]
    AskAgent {
        #[serde(flatten)]
        header: StudioOperationHeader,
        #[serde(rename = "toRole")]
        to_role: String,
        blocking: bool,
        question: String,
    },
    #[serde(rename = "answer_agent")]
    AnswerAgent {
        #[serde(flatten)]
        header: StudioOperationHeader,
        #[serde(rename = "questionId")]
        question_id: String,
        answer: String,
        evidence: Vec<String>,
    },
    #[serde(rename = "report_finding")]
    ReportFinding {
        #[serde(flatten)]
        header: StudioOperationHeader,
        severity: String,
        blocking: bool,
        #[serde(rename = "relatedIds")]
        related_ids: Vec<String>,
        summary: String,
    },
    #[serde(rename = "request_transition")]
    RequestTransition {
        #[serde(flatten)]
        header: StudioOperationHeader,
        #[serde(rename = "targetStatus")]
        target_status: String,
    },
    #[serde(rename = "request_context")]
    RequestContext {
        #[serde(flatten)]
        header: StudioOperationHeader,
        #[serde(default)]
        categories: Vec<String>,
        #[serde(default)]
        selectors: Vec<ContextSelector>,
        #[serde(rename = "maximumTokens", alias = "maximumChars")]
        maximum_tokens: usize,
    },
    #[serde(rename = "studio_final_report")]
    StudioFinalReport {
        #[serde(flatten)]
        header: StudioOperationHeader,
        payload: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextSelector {
    pub kind: String,
    pub query: Option<String>,
    pub relative_path: Option<String>,
    pub source_id: Option<String>,
    pub maximum_items: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StudioOperationHeader {
    #[serde(rename = "operationId")]
    pub operation_id: String,
    #[serde(rename = "initiativeId")]
    pub initiative_id: String,
    #[serde(rename = "taskId")]
    pub task_id: Option<String>,
    pub role: String,
    #[serde(rename = "artifactType")]
    pub artifact_type: String,
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(rename = "specVersion")]
    pub spec_version: i64,
    pub reason: String,
    #[serde(rename = "expectedOutcome")]
    pub expected_outcome: String,
    #[serde(rename = "sourceContextBundleId")]
    pub source_context_bundle_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchFile {
    pub id: String,
    pub path: String,
    #[serde(rename = "beforeSha256")]
    pub before_sha256: String,
    pub patch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequest {
    pub argv: Vec<String>,
    pub cwd: String,
    pub reason: String,
    #[serde(rename = "expectedOutcome")]
    pub expected_outcome: String,
    #[serde(rename = "requiresNetwork")]
    pub requires_network: bool,
    #[serde(rename = "mayModifyFiles")]
    pub may_modify_files: bool,
}

#[cfg(test)]
mod studio_tests {
    use super::*;

    #[test]
    fn parses_backward_compatible_studio_artifact_envelope() {
        let operation: AgentOperation = serde_json::from_value(serde_json::json!({
            "type":"propose_artifact","operationId":"OP-1","initiativeId":"INIT-1",
            "taskId":null,"role":"architect","artifactType":"adr","schemaVersion":1,
            "specVersion":2,"reason":"compare options","expectedOutcome":"approved ADR",
            "sourceContextBundleId":"CTX-1","payload":{"decision":"A"}
        }))
        .unwrap();
        match operation {
            AgentOperation::ProposeArtifact { header, payload } => {
                assert_eq!(header.spec_version, 2);
                assert_eq!(payload["decision"], "A");
            }
            _ => panic!("wrong operation variant"),
        }
    }

    #[test]
    fn existing_patch_operation_still_parses() {
        let operation: AgentOperation = serde_json::from_value(serde_json::json!({
            "type":"propose_patch","proposalId":"P","summary":"s","files":[],
            "riskNotes":[],"suggestedCommands":[]
        }))
        .unwrap();
        assert!(matches!(operation, AgentOperation::ProposePatch { .. }));
    }

    #[test]
    fn operations_reject_unknown_fields() {
        let operation = serde_json::from_value::<AgentOperation>(serde_json::json!({
            "type":"publish_belief","operationId":"OP-1","initiativeId":"INIT-1",
            "taskId":null,"role":"verifier","artifactType":"belief","schemaVersion":1,
            "specVersion":1,"reason":"state view","expectedOutcome":"alignment",
            "sourceContextBundleId":"CTX-1","payload":{},"untrustedAuthority":true
        }));
        assert!(operation.is_err());
    }
}
