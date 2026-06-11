# Synthesize v13.1 Release-Candidate Patch

V13.1 is a targeted release-candidate patch that fixes the remaining Agent Profile policy-binding issue identified in source review.

## What changed

- Patch proposals are now associated with the persisted context bundle that produced them.
- `ContextBundleView` now records `agent_profile_id`.
- `validate_patch_proposal` accepts `context_bundle_id` and enforces Agent Profile policy from the persisted context bundle, not from the currently selected UI profile.
- The UI records `sourceAgentProfileId` and `contextBundleId` per proposal ID.
- Switching from Local Reviewer/Planner to Local Patcher after a patch appears no longer makes the original report-only proposal patch-validatable.
- `patch_proposals` now stores optional `source_context_bundle_id` and `source_agent_profile_id` for auditability.
- Diff Queue now displays the source agent/context for each patch proposal.
- Context Visibility now displays the source Agent Profile used to build the exact persisted context bundle.
- Added backend tests for persisted-context source profile enforcement.

## Remaining release gate

V13.1 still requires real local verification before any production-ready claim:

```bash
pnpm install
git add pnpm-lock.yaml
./scripts/release-check.sh
```

Then run the manual smoke tests in `RELEASE_CHECKLIST.md`.

## Honest status

Synthesize v13.1 is a production-shaped release candidate. It should not be called a verified production build until the lockfile is committed, CI/release checks pass, and at least one real local-model patch workflow succeeds on a throwaway repo or clean branch.
