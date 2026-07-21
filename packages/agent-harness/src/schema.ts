import { z } from 'zod';

export const CommandRequestOperationSchema = z.object({
  type: z.literal('run_command'),
  argv: z.array(z.string()).min(1),
  cwd: z.string(),
  reason: z.string(),
  expectedOutcome: z.string(),
  requiresNetwork: z.boolean(),
  mayModifyFiles: z.boolean()
});

const RelativeRepoPathSchema = z.string().min(1).refine((path) => {
  if (path.startsWith('/') || path.startsWith('~')) return false;
  if (/^[A-Za-z]:/.test(path)) return false;
  return !path.split(/[\\/]+/).some((part) => part === '..' || part === '');
}, 'path must be a relative repo path without traversal');

const PatchFileSchema = z.object({
  id: z.string().min(1).optional(),
  fileId: z.string().min(1).optional(),
  path: RelativeRepoPathSchema,
  beforeSha256: z.string(),
  patch: z.string().optional(),
  unifiedDiff: z.string().optional(),
  risk: z.string().optional()
}).transform((value, ctx) => {
  const id = value.id ?? value.fileId;
  const patch = value.patch ?? value.unifiedDiff;
  if (!id) {
    ctx.addIssue({ code: z.ZodIssueCode.custom, message: 'patch file requires id or fileId' });
    return z.NEVER;
  }
  if (!patch) {
    ctx.addIssue({ code: z.ZodIssueCode.custom, message: 'patch file requires patch or unifiedDiff' });
    return z.NEVER;
  }
  return { id, path: value.path, beforeSha256: value.beforeSha256, patch };
});

const StudioRoleSchema = z.enum([
  'dreamer', 'fde', 'ux_designer', 'skeptic', 'architect', 'planner', 'builder', 'verifier', 'reviewer'
]);

const StudioHeaderShape = {
  operationId: z.string().min(1).max(160),
  initiativeId: z.string().min(1).max(160),
  taskId: z.string().min(1).max(160).optional(),
  role: StudioRoleSchema,
  artifactType: z.string().min(1).max(100),
  schemaVersion: z.number().int().positive(),
  specVersion: z.number().int().positive(),
  reason: z.string().min(1).max(2000),
  expectedOutcome: z.string().min(1).max(2000),
  sourceContextBundleId: z.string().min(1).max(160).optional()
};

export const OperationSchema = z.discriminatedUnion('type', [
  z.object({ type: z.literal('read_file'), path: RelativeRepoPathSchema, reason: z.string() }),
  z.object({ type: z.literal('search_repo'), query: z.string(), glob: z.string().optional(), reason: z.string() }),
  z.object({
    type: z.literal('propose_patch'),
    proposalId: z.string().min(1),
    summary: z.string(),
    baseCommit: z.string().optional(),
    currentCommit: z.string().optional(),
    files: z.array(PatchFileSchema).min(1),
    riskNotes: z.array(z.string()).default([]),
    suggestedCommands: z.array(CommandRequestOperationSchema).default([])
  }),
  CommandRequestOperationSchema,
  z.object({ type: z.literal('ask_user'), question: z.string(), options: z.array(z.string()).optional() }),
  z.object({ type: z.literal('report'), summary: z.string(), details: z.string().optional() }),
  z.object({ type: z.literal('final_report'), summary: z.string(), changedFiles: z.array(z.string()), testsRun: z.array(z.string()), remainingRisks: z.array(z.string()) }),
  z.object({
    type: z.literal('hand_off'),
    toSkill: z.string().min(1),
    contextSummary: z.string(),
    reason: z.string(),
    preserveFileHistory: z.boolean().default(true)
  }),
  z.object({ type: z.literal('propose_artifact'), ...StudioHeaderShape, payload: z.unknown() }).strict(),
  z.object({ type: z.literal('publish_belief'), ...StudioHeaderShape, payload: z.unknown() }).strict(),
  z.object({ type: z.literal('ask_agent'), ...StudioHeaderShape, toRole: StudioRoleSchema, blocking: z.boolean(), question: z.string().min(1).max(8000) }).strict(),
  z.object({ type: z.literal('answer_agent'), ...StudioHeaderShape, questionId: z.string().min(1), answer: z.string().min(1).max(16000), evidence: z.array(z.string()).max(100) }).strict(),
  z.object({ type: z.literal('report_finding'), ...StudioHeaderShape, severity: z.enum(['info', 'warning', 'error', 'critical']), blocking: z.boolean(), relatedIds: z.array(z.string()).max(200), summary: z.string().min(1).max(8000) }).strict(),
  z.object({ type: z.literal('request_transition'), ...StudioHeaderShape, targetStatus: z.string().min(1).max(100) }).strict(),
  z.object({
    type: z.literal('request_context'), ...StudioHeaderShape,
    categories: z.array(z.string()).max(100).optional(),
    selectors: z.array(z.object({
      kind: z.enum(['repository_map', 'file_excerpt', 'symbol', 'definition', 'reference', 'direct_dependency', 'test', 'requirement', 'adr', 'ux_criterion', 'assumption', 'evidence', 'prior_finding', 'task_summary']),
      query: z.string().max(500).nullable().optional(), relativePath: z.string().max(1000).nullable().optional(),
      sourceId: z.string().max(500).nullable().optional(), maximumItems: z.number().int().min(1).max(200)
    }).strict()).min(1).max(20),
    maximumTokens: z.number().int().min(1).max(500_000)
  }).strict(),
  z.object({ type: z.literal('studio_final_report'), ...StudioHeaderShape, payload: z.unknown() }).strict()
]);

export const OperationListSchema = z.object({ operations: z.array(OperationSchema) });
