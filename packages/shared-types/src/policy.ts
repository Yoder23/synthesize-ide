export type FilePolicy = {
  repoRoot: string;
  allowHiddenFiles: boolean;
  deniedGlobs: string[];
  deniedBasenames: string[];
  requireExplicitApprovalGlobs: string[];
};

export type CommandPattern = {
  program: string;
  argsPrefix?: string[];
};

export type CommandPolicy = {
  workingDirectoryMustBeInsideRepo: boolean;
  networkDefault: 'disabled' | 'ask' | 'allowed';
  allow: CommandPattern[];
  deny: CommandPattern[];
  requireApprovalFor: CommandPattern[];
  maxRuntimeSeconds: number;
  maxOutputBytes: number;
  envPolicy: {
    inheritUserEnv: boolean;
    allowlist: string[];
    denylist: string[];
  };
};

export type ContextPolicy = {
  maxTokens: number;
  includeCurrentFile: boolean;
  includeSelection: boolean;
  includeGitDiff: boolean;
  includePackageScripts: boolean;
  includeTests: boolean;
  deniedGlobs: string[];
};
