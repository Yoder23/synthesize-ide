# Agent Operation Protocol

The model emits structured operations. The harness parses and validates them. Invalid operations are rejected or returned to the model for repair.

The protocol boundary is sacred:

> The model never acts. The model proposes typed operations. The harness validates, displays, and executes only approved operations.

## Operations

```ts
type AgentOperation =
  | ReadFileOperation
  | SearchRepoOperation
  | ProposePatchOperation
  | RequestCommandOperation
  | AskUserOperation
  | FinalReportOperation
  | ProposeArtifactOperation
  | PublishBeliefOperation
  | AskAgentOperation
  | AnswerAgentOperation
  | ReportFindingOperation
  | RequestTransitionOperation
  | RequestContextOperation
  | StudioFinalReportOperation;
```

Studio operations share a required header: `operationId`, `initiativeId`, optional `taskId`, `role`, `artifactType`, `schemaVersion`, `specVersion`, optional `sourceContextBundleId`, `reason`, and `expectedOutcome`. Unknown fields, stale spec versions, mismatched tasks/contexts, duplicate operation IDs, oversized payloads, and role-forbidden artifact types are rejected by the backend.

Structured communication is explicit: roles publish artifacts and beliefs or ask an addressed role a question. A question records its reason, blocking status, and eventual answer. Agents can request a lifecycle transition; they cannot perform it. Studio final reports are proof artifacts, not permission to merge.

Builder patch proposals remain proposals. Their proof link records task/spec/requirement/ADR/context bindings and a canonical SHA-256 operation hash before any governed patch lifecycle can proceed.

## Command operations use argv

Prefer:

```json
{
  "type": "run_command",
  "argv": ["pnpm", "test", "auth"],
  "cwd": ".",
  "reason": "Verify auth tests",
  "expectedOutcome": "Tests pass",
  "requiresNetwork": false,
  "mayModifyFiles": false
}
```

Avoid raw shell strings. Shell mode is an elevated explicit approval feature.

## Patch operations

Patch operations must include:

- `proposalId`
- per-file `id`
- per-file `beforeSha256`
- optional `baseCommit`
- optional `currentCommit`
- unified diff text for each file

Example:

```json
{
  "type": "propose_patch",
  "proposalId": "patch-001",
  "summary": "Fix auth refresh behavior and add a regression test.",
  "baseCommit": "abc123",
  "currentCommit": "abc123",
  "files": [
    {
      "id": "patch-001-file-001",
      "path": "src/auth/refresh.ts",
      "beforeSha256": "...",
      "patch": "diff --git a/src/auth/refresh.ts b/src/auth/refresh.ts\n..."
    }
  ],
  "riskNotes": [],
  "suggestedCommands": []
}
```

The patch is rejected if the current file hash differs from `beforeSha256`. If `currentCommit` is supplied and the repo has moved, the patch must be revalidated or regenerated.

## JSON extraction policy

The harness accepts only:

1. a strict JSON object as the entire response, or
2. a fenced `json` block containing a strict JSON object.

It intentionally does not slice from the first `{` to the last `}` because that accepts ambiguous model chatter and code snippets.

## Operation lifecycle

```txt
Model text
  ↓
Strict JSON extraction
  ↓
Schema validation
  ↓
Operation normalization
  ↓
Policy decision
  ↓
UI review
  ↓
Execution
  ↓
Audit record
```
