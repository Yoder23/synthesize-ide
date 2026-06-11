use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentOperation {
    #[serde(rename = "read_file")]
    ReadFile { path: String, reason: String },
    #[serde(rename = "search_repo")]
    SearchRepo { query: String, glob: Option<String>, reason: String },
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
    AskUser { question: String, options: Option<Vec<String>> },
    #[serde(rename = "final_report")]
    FinalReport { summary: String, #[serde(rename = "changedFiles")] changed_files: Vec<String>, #[serde(rename = "testsRun")] tests_run: Vec<String>, #[serde(rename = "remainingRisks")] remaining_risks: Vec<String> },
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
