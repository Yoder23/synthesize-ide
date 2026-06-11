export type ContextItemBase = { id: string; tokenEstimate: number; source: string; trust: 'untrusted-data' | 'user-provided' | 'system' };
export type FileContext = ContextItemBase & { kind: 'file'; path: string; startLine: number; endLine: number; content: string; sha256: string };
export type SelectionContext = ContextItemBase & { kind: 'selection'; path: string; startLine: number; endLine: number; content: string };
export type SearchResultContext = ContextItemBase & { kind: 'search_result'; query: string; path: string; line: number; snippet: string };
export type SymbolContext = ContextItemBase & { kind: 'symbol'; path: string; symbolName: string; symbolKind: string; signature?: string };
export type GitDiffContext = ContextItemBase & { kind: 'git_diff'; diff: string };
export type PackageScriptContext = ContextItemBase & { kind: 'package_script'; path: string; scripts: Record<string, string> };
export type TestContext = ContextItemBase & { kind: 'test'; path: string; content: string; sha256: string };

export type ContextItem = FileContext | SelectionContext | SearchResultContext | SymbolContext | GitDiffContext | PackageScriptContext | TestContext;

export type ContextBundle = {
  id: string;
  repoRoot: string;
  userTask: string;
  included: ContextItem[];
  excluded: Array<{ path: string; reason: string }>;
  tokenEstimate: number;
  createdAt: string;
};
