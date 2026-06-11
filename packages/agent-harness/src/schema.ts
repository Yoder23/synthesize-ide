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
  z.object({ type: z.literal('final_report'), summary: z.string(), changedFiles: z.array(z.string()), testsRun: z.array(z.string()), remainingRisks: z.array(z.string()) })
]);

export const OperationListSchema = z.object({ operations: z.array(OperationSchema) });
