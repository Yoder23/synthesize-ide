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

export type AgentOperation =
  | ReadFileOperation
  | SearchRepoOperation
  | ProposePatchOperation
  | CommandRequestOperation
  | AskUserOperation
  | ReportOperation
  | FinalReportOperation;
