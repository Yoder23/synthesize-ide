import type { CommandPolicy, ContextPolicy, FilePolicy } from './policy';

export type AgentRole = 'planner' | 'patcher' | 'reviewer' | 'tester' | 'refactorer' | 'autonomous';

export type AgentProfile = {
  id: string;
  name: string;
  modelProfileId: string;
  role: AgentRole;
  permissions: {
    readFiles: boolean;
    searchRepo: boolean;
    proposePatches: boolean;
    applyPatches: 'never' | 'afterApproval';
    runCommands: 'never' | 'allowlistedAfterApproval' | 'explicitApproval';
    network: 'never' | 'explicitApproval';
  };
  contextPolicy: ContextPolicy;
  filePolicy: FilePolicy;
  commandPolicy: CommandPolicy;
  autonomy: {
    maxIterations: number;
    requireApprovalBeforePatch: boolean;
    requireApprovalBeforeCommand: boolean;
    stopOnTestFailure: boolean;
  };
};

export type PatchApprovalState = 'proposed' | 'under_review' | 'approved' | 'partially_approved' | 'rejected' | 'applied' | 'rolled_back';

export type PatchFileProposal = {
  id: string;
  path: string;
  beforeSha256: string;
  patch: string;
  approvalState: PatchApprovalState;
};

export type PatchProposal = {
  id: string;
  summary: string;
  baseCommit?: string;
  currentCommit?: string;
  approvalState: PatchApprovalState;
  files: PatchFileProposal[];
  riskNotes: string[];
  suggestedCommands: CommandRequestOperation[];
  createdAt: string;
};

export type ReadFileOperation = { type: 'read_file'; path: string; reason: string };
export type SearchRepoOperation = { type: 'search_repo'; query: string; glob?: string; reason: string };
export type CommandRequestOperation = {
  type: 'run_command';
  argv: string[];
  cwd: string;
  reason: string;
  expectedOutcome: string;
  requiresNetwork: boolean;
  mayModifyFiles: boolean;
};
export type ProposePatchOperation = {
  type: 'propose_patch';
  proposalId: string;
  summary: string;
  baseCommit?: string;
  currentCommit?: string;
  files: Array<{ id: string; path: string; beforeSha256: string; patch: string }>;
  riskNotes: string[];
  suggestedCommands: CommandRequestOperation[];
};
export type AskUserOperation = { type: 'ask_user'; question: string; options?: string[] };
export type ReportOperation = { type: 'report'; summary: string; details?: string };
export type FinalReportOperation = { type: 'final_report'; summary: string; changedFiles: string[]; testsRun: string[]; remainingRisks: string[] };

/**
 * HandOffOperation lets a skill agent delegate to another named skill.
 * The backend serialises `contextSummary` into the next agent's system prompt,
 * then acquires the GPU serial lock before spawning the successor. Only one
 * Qwen3 instance runs at a time; no simultaneous GPU use.
 */
export type HandOffOperation = {
  type: 'hand_off';
  toSkill: string;
  contextSummary: string;
  reason: string;
  preserveFileHistory: boolean;
};

export type StudioRole =
  | 'dreamer'
  | 'fde'
  | 'ux_designer'
  | 'skeptic'
  | 'architect'
  | 'planner'
  | 'builder'
  | 'verifier'
  | 'reviewer';

export type StudioOperationHeader = {
  operationId: string;
  initiativeId: string;
  taskId?: string;
  role: StudioRole;
  artifactType: string;
  schemaVersion: number;
  specVersion: number;
  reason: string;
  expectedOutcome: string;
  sourceContextBundleId?: string;
};

export type ProposeArtifactOperation = StudioOperationHeader & { type: 'propose_artifact'; payload: unknown };
export type PublishBeliefOperation = StudioOperationHeader & { type: 'publish_belief'; payload: unknown };
export type AskAgentOperation = StudioOperationHeader & { type: 'ask_agent'; toRole: StudioRole; blocking: boolean; question: string };
export type AnswerAgentOperation = StudioOperationHeader & { type: 'answer_agent'; questionId: string; answer: string; evidence: string[] };
export type ReportFindingOperation = StudioOperationHeader & { type: 'report_finding'; severity: 'info' | 'warning' | 'error' | 'critical'; blocking: boolean; relatedIds: string[]; summary: string };
export type RequestTransitionOperation = StudioOperationHeader & { type: 'request_transition'; targetStatus: string };
export type ContextSelector = {
  kind: 'repository_map' | 'file_excerpt' | 'symbol' | 'definition' | 'reference' | 'direct_dependency' | 'test' |
    'requirement' | 'adr' | 'ux_criterion' | 'assumption' | 'evidence' | 'prior_finding' | 'task_summary';
  query?: string | null; relativePath?: string | null; sourceId?: string | null; maximumItems: number;
};
export type RequestContextOperation = StudioOperationHeader & { type: 'request_context'; categories?: string[]; selectors: ContextSelector[]; maximumTokens: number };
export type StudioFinalReportOperation = StudioOperationHeader & { type: 'studio_final_report'; payload: unknown };

export type AgentOperation =
  | ReadFileOperation
  | SearchRepoOperation
  | ProposePatchOperation
  | CommandRequestOperation
  | AskUserOperation
  | ReportOperation
  | FinalReportOperation
  | HandOffOperation
  | ProposeArtifactOperation
  | PublishBeliefOperation
  | AskAgentOperation
  | AnswerAgentOperation
  | ReportFindingOperation
  | RequestTransitionOperation
  | RequestContextOperation
  | StudioFinalReportOperation;
