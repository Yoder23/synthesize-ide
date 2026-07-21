import { z } from 'zod';
import type { SkillTier } from '@synthesize/shared-types';

// ---------------------------------------------------------------------------
// Skill Definition
// ---------------------------------------------------------------------------

/**
 * A SkillDefinition configures one Qwen3 (or cloud) subagent that can be
 * spawned on demand and can receive hand-offs from other skills.
 *
 * Design invariants:
 *  - Only ONE skill agent runs at a time (GPU serial lock in Rust backend).
 *  - Skills may hand off to another skill via `hand_off` operation.
 *  - Cloud skills (tier: cloud-*) require explicit endpoint approval.
 *  - All patches/commands produced by any skill still pass through the full
 *    Synthesize validation → approval → apply → audit lifecycle.
 */
export const SkillDefinitionSchema = z.object({
  /** Machine-readable unique ID, used in `hand_off.toSkill` */
  id: z.string().min(1).regex(/^[a-z0-9-]+$/, 'skill ID must be lowercase alphanumeric with hyphens'),

  /** Human-readable display name */
  name: z.string().min(1),

  /** Short description shown in the Skill Agent panel */
  description: z.string(),

  /**
   * Which model registry entry backs this skill.
   * Must match a `registryId` in models/registry.json or the runtime-adapters.
   */
  modelRegistryId: z.string().min(1),

  /**
   * The tier determines GPU resource strategy:
   * - fast / balanced / powerful / frontier-local → local Qwen3 (GPU serial lock)
   * - cloud-heavy / cloud-reasoning → cloud API (no GPU, but costs money)
   */
  tier: z.enum(['fast', 'balanced', 'powerful', 'frontier-local', 'cloud-heavy', 'cloud-reasoning'] satisfies [SkillTier, ...SkillTier[]]),

  /**
   * Additional system prompt text injected after the standard Synthesize
   * system prompt. Use this to specialise the agent for a specific domain.
   */
  systemPromptAddon: z.string().default(''),

  /**
   * Operations this skill is permitted to emit. Backend still validates all
   * operations; this is a UI hint / schema-level allowlist.
   */
  allowedOperations: z.array(
    z.enum(['read_file', 'search_repo', 'propose_patch', 'run_command', 'ask_user', 'report', 'final_report', 'hand_off'])
  ).default(['read_file', 'search_repo', 'propose_patch', 'ask_user', 'report', 'final_report', 'hand_off']),

  /**
   * Optional: list of skill IDs this skill is allowed to hand off to.
   * Empty means any skill is allowed (backend enforces by looking up skill registry).
   */
  allowedHandOffTargets: z.array(z.string()).default([]),

  /**
   * Max iterations before the backend forces a FinalReport.
   * Prevents runaway loops.
   */
  maxIterations: z.number().int().min(1).max(50).default(10),

  /**
   * Whether this skill is enabled and will appear in the spawn UI.
   */
  enabled: z.boolean().default(true),

  /**
   * Optional tags for filtering in the UI.
   */
  tags: z.array(z.string()).default([])
});

export type SkillDefinition = z.infer<typeof SkillDefinitionSchema>;

// ---------------------------------------------------------------------------
// Built-in Default Skills
// ---------------------------------------------------------------------------

export const DEFAULT_SKILLS: SkillDefinition[] = [
  {
    id: 'code-writer',
    name: 'Code Writer',
    description: 'Low-lift code generation: adds new functions, modules, and boilerplate. Stays small and reviewable.',
    modelRegistryId: 'qwen3-coder-1.7b-instruct-q4-k-m',
    tier: 'balanced',
    systemPromptAddon: 'You are a focused code-writing agent. Produce small, reviewable patches only. Prefer propose_patch operations. If the task is large, use hand_off to delegate to the "planner" skill first.',
    allowedOperations: ['read_file', 'search_repo', 'propose_patch', 'ask_user', 'report', 'final_report', 'hand_off'],
    allowedHandOffTargets: ['code-reviewer', 'test-writer', 'planner'],
    maxIterations: 10,
    enabled: true,
    tags: ['writing', 'patch']
  },
  {
    id: 'code-reviewer',
    name: 'Code Reviewer',
    description: 'Reviews diffs and proposed patches for bugs, security issues, and style. Produces a report or targeted fix patch.',
    modelRegistryId: 'qwen3-coder-1.7b-instruct-q4-k-m',
    tier: 'balanced',
    systemPromptAddon: 'You are a code review agent. Your primary output is a detailed report operation. Only propose_patch when you find a concrete, specific bug to fix. Do not suggest cosmetic changes.',
    allowedOperations: ['read_file', 'search_repo', 'propose_patch', 'ask_user', 'report', 'final_report', 'hand_off'],
    allowedHandOffTargets: ['code-writer', 'test-writer'],
    maxIterations: 8,
    enabled: true,
    tags: ['review', 'quality']
  },
  {
    id: 'test-writer',
    name: 'Test Writer',
    description: 'Writes unit and integration tests for existing code. Focuses on edge cases and regression coverage.',
    modelRegistryId: 'qwen3-coder-1.7b-instruct-q4-k-m',
    tier: 'balanced',
    systemPromptAddon: 'You are a test-writing agent. Emit propose_patch operations that add tests to the codebase. Keep each test file patch small and self-contained. Prefer one test file per patch proposal.',
    allowedOperations: ['read_file', 'search_repo', 'propose_patch', 'ask_user', 'report', 'final_report', 'hand_off'],
    allowedHandOffTargets: ['code-reviewer'],
    maxIterations: 10,
    enabled: true,
    tags: ['testing', 'quality']
  },
  {
    id: 'debugger',
    name: 'Debugger',
    description: 'Investigates failing tests and error traces. Proposes minimal targeted patches to fix root causes.',
    modelRegistryId: 'qwen3-coder-8b-instruct-q4-k-m',
    tier: 'powerful',
    systemPromptAddon: 'You are a debugging agent. Read error output carefully. Search the repo for relevant code paths. Propose the smallest patch that fixes the root cause. Always include a riskNote explaining what you changed and why.',
    allowedOperations: ['read_file', 'search_repo', 'propose_patch', 'run_command', 'ask_user', 'report', 'final_report', 'hand_off'],
    allowedHandOffTargets: ['code-reviewer', 'test-writer'],
    maxIterations: 12,
    enabled: true,
    tags: ['debugging', 'fix']
  },
  {
    id: 'planner',
    name: 'Planner',
    description: 'Breaks large tasks into a sequenced plan and hands off subtasks to specialist skills.',
    modelRegistryId: 'qwen3-coder-8b-instruct-q4-k-m',
    tier: 'powerful',
    systemPromptAddon: 'You are a planning agent. Your job is to understand the full scope of a task, break it into concrete subtasks, and emit hand_off operations to the correct specialist skill for each subtask. Do not write code yourself. Only plan, report, and hand off.',
    allowedOperations: ['read_file', 'search_repo', 'ask_user', 'report', 'final_report', 'hand_off'],
    allowedHandOffTargets: ['code-writer', 'code-reviewer', 'test-writer', 'debugger', 'docs-writer', 'cloud-architect'],
    maxIterations: 6,
    enabled: true,
    tags: ['planning', 'orchestration']
  },
  {
    id: 'docs-writer',
    name: 'Docs Writer',
    description: 'Writes or updates documentation: README files, JSDoc/TSDoc comments, API docs, and guides.',
    modelRegistryId: 'qwen3-coder-1.7b-instruct-q4-k-m',
    tier: 'balanced',
    systemPromptAddon: 'You are a documentation agent. Write clear, accurate documentation. Emit propose_patch operations that add or update docs files or inline comments. Never modify logic; only add or update documentation.',
    allowedOperations: ['read_file', 'search_repo', 'propose_patch', 'ask_user', 'report', 'final_report'],
    allowedHandOffTargets: [],
    maxIterations: 8,
    enabled: true,
    tags: ['docs', 'writing']
  },
  {
    id: 'cloud-architect',
    name: 'Cloud Architect (GPT-4o)',
    description: 'Heavy-lift: complex architecture decisions, large codebase analysis, security review. Uses cloud frontier model.',
    modelRegistryId: 'cloud-openai-gpt-4o',
    tier: 'cloud-heavy',
    systemPromptAddon: 'You are an expert software architect. Provide a thorough architectural analysis and concrete recommendations. Emit report operations with detailed findings, and propose_patch only for targeted, critical fixes. Always surface remaining risks in final_report.',
    allowedOperations: ['read_file', 'search_repo', 'propose_patch', 'ask_user', 'report', 'final_report', 'hand_off'],
    allowedHandOffTargets: ['code-writer', 'code-reviewer'],
    maxIterations: 15,
    enabled: true,
    tags: ['architecture', 'cloud', 'heavy-lift']
  },
  {
    id: 'cloud-reasoner',
    name: 'Cloud Reasoner (o3)',
    description: 'Hard problems only: algorithmic correctness, formal reasoning, security proofs. Uses OpenAI o3.',
    modelRegistryId: 'cloud-openai-o3',
    tier: 'cloud-reasoning',
    systemPromptAddon: 'You are a formal reasoning agent. Think step by step. For algorithmic problems, show your reasoning before proposing any patch. Prioritise correctness over brevity. Surface all edge cases in remainingRisks.',
    allowedOperations: ['read_file', 'search_repo', 'propose_patch', 'ask_user', 'report', 'final_report'],
    allowedHandOffTargets: [],
    maxIterations: 20,
    enabled: true,
    tags: ['reasoning', 'cloud', 'correctness']
  }
];

// ---------------------------------------------------------------------------
// Skill Registry
// ---------------------------------------------------------------------------

export class SkillRegistry {
  private skills: Map<string, SkillDefinition>;

  constructor(initial: SkillDefinition[] = DEFAULT_SKILLS) {
    this.skills = new Map(initial.map((s) => [s.id, s]));
  }

  list(): SkillDefinition[] {
    return Array.from(this.skills.values()).filter((s) => s.enabled);
  }

  listAll(): SkillDefinition[] {
    return Array.from(this.skills.values());
  }

  get(id: string): SkillDefinition | undefined {
    return this.skills.get(id);
  }

  upsert(skill: SkillDefinition): void {
    const parsed = SkillDefinitionSchema.parse(skill);
    this.skills.set(parsed.id, parsed);
  }

  remove(id: string): boolean {
    return this.skills.delete(id);
  }

  /**
   * Validates that a hand-off target is allowed by the originating skill.
   * Returns true if allowed, false if the skill is not found or not permitted.
   */
  isHandOffAllowed(fromSkillId: string, toSkillId: string): { allowed: boolean; reason: string } {
    const from = this.skills.get(fromSkillId);
    if (!from) return { allowed: false, reason: `originating skill '${fromSkillId}' not found in registry` };
    const to = this.skills.get(toSkillId);
    if (!to) return { allowed: false, reason: `target skill '${toSkillId}' not found in registry` };
    if (!to.enabled) return { allowed: false, reason: `target skill '${toSkillId}' is disabled` };
    if (from.allowedHandOffTargets.length > 0 && !from.allowedHandOffTargets.includes(toSkillId)) {
      return { allowed: false, reason: `skill '${fromSkillId}' is not permitted to hand off to '${toSkillId}'` };
    }
    return { allowed: true, reason: 'allowed' };
  }

  /**
   * Serialises the registry to a plain JSON array for persistence.
   */
  toJSON(): SkillDefinition[] {
    return this.listAll();
  }

  /**
   * Restores the registry from a persisted JSON array.
   * Invalid entries are skipped and logged.
   */
  static fromJSON(data: unknown[]): SkillRegistry {
    const valid: SkillDefinition[] = [];
    for (const item of data) {
      const result = SkillDefinitionSchema.safeParse(item);
      if (result.success) valid.push(result.data);
      else console.warn('[SkillRegistry] skipped invalid skill entry:', result.error.message);
    }
    return new SkillRegistry(valid.length > 0 ? valid : DEFAULT_SKILLS);
  }
}

// ---------------------------------------------------------------------------
// Skill Queue State (shared type used by UI + backend bridge)
// ---------------------------------------------------------------------------

export type SkillQueueEntry = {
  id: string;
  skillId: string;
  skillName: string;
  contextSummary: string;
  status: 'queued' | 'running' | 'completed' | 'failed' | 'cancelled';
  startedAt?: string;
  completedAt?: string;
  errorMessage?: string;
  iterations: number;
  handOffFrom?: string;
};

export type SkillQueueState = {
  currentEntry: SkillQueueEntry | null;
  queue: SkillQueueEntry[];
  history: SkillQueueEntry[];
  gpuLockHeld: boolean;
};
