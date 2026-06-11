import type { ContextBundle, ContextItem } from '@synthesize/shared-types';

export type BuildContextRequest = {
  repoRoot: string;
  userTask: string;
  currentFile?: string;
  selectedText?: { path: string; startLine: number; endLine: number; content: string };
  maxTokens: number;
};

export function estimateTokens(text: string): number {
  return Math.ceil(text.length / 4);
}

export function createEmptyContextBundle(req: BuildContextRequest): ContextBundle {
  return {
    id: crypto.randomUUID(),
    repoRoot: req.repoRoot,
    userTask: req.userTask,
    included: [] as ContextItem[],
    excluded: [],
    tokenEstimate: estimateTokens(req.userTask),
    createdAt: new Date().toISOString()
  };
}
