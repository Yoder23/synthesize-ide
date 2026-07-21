# Security model

Models, repository content, generated artifacts, prototype documents, and command/test output are untrusted. Backend repositories bind operations to initiative, task, spec version, role run, source context, repository, worktree, and hashes. Roles can propose transitions but cannot perform them.

Controls include explicit role permissions, schema and size limits, immutable versions, evidence provenance, replay-resistant human approvals, worktree/base bindings, mandate and budget enforcement, RepoGuard path validation, governed argv execution, no model-accessible merge command, and complete transition/intervention audit events.

Context OS additionally validates runtime capabilities, applies role-specific projections, isolates role summaries, byte-bounds repository reads, caps progressive disclosure, and verifies the token equation and exact-message hash before inference. Stale spec, ADR, capability, initiative, or task bindings fail closed; mandatory context overflow never falls through to runtime truncation.

Declarative prototypes are data interpreted by an allowlisted renderer. They cannot contain script, URLs, Tauri commands, storage access, filesystem access, network access, or process access. Prototype interactions only update namespaced component-local values.

Pulse deterministic findings override neural optimism. The liquid observer is experimental and shadow-only unless compatible calibrated weights are explicitly loaded; even calibrated output remains a proposal source with no authority.

Synthesize still does not claim OS-level sandboxing, container isolation, or complete network isolation. Worktrees isolate Git state, not processes. Clean branches and backups remain recommended.
