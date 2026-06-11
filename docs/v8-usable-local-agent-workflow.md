# V8 Usable Local Agent Workflow

Goal: open a repo, ask a local or fake agent for a patch, review it, and land or roll it back through the governed backend lifecycle.

```txt
Open repo
  ↓
Build visible context bundle
  ↓
Fake runtime or OpenAI-compatible endpoint returns typed operations
  ↓
Operation parser validates strict JSON schema
  ↓
Diff queue shows proposal summary and changed files
  ↓
Backend validates and snapshots proposal
  ↓
Backend records approval
  ↓
Backend applies persisted snapshot with checkpoint/restore transaction shape
  ↓
Backend-owned rollback by proposal id if needed
  ↓
Session log shows audit events
```

## Safety warning

Synthesize shows the user:

> Use a clean Git branch or throwaway repo. Synthesize checkpoints changes, but this is not a substitute for version control.

The checkpoint system is a safety aid, not a replacement for source control.
