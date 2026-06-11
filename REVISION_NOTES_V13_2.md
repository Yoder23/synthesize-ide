# Synthesize v13.2 Release-Candidate Patch

V13.2 is a narrow production-readiness patch. It removes the remaining Agent Profile policy fallback identified in v13.1 source review.

## What changed

- `validate_patch_proposal` now requires `context_bundle_id`.
- Backend patch validation now always loads the persisted context bundle that produced the model response.
- Agent Profile policy is enforced from the persisted context bundle, not from the current UI selection or a frontend-provided fallback.
- The previous fallback to `req.agent_profile_id.unwrap_or("local-patcher")` was removed from the production validation path.
- Diff Queue now disables validation for proposals without persisted context binding.
- Frontend validation now blocks early with a clear error if a proposal is not bound to a context bundle.
- Added backend test coverage for missing `context_bundle_id` rejection.
- Updated production-readiness docs and release checklist.

## Why this matters

A patch generated while using Local Reviewer or Local Planner must not become validatable simply because the user later switches the UI to Local Patcher. The backend now requires the source context bundle and enforces the Agent Profile stored there.

The invariant is now:

```text
context bundle source profile
  -> model response
  -> parsed proposal
  -> validation request with context_bundle_id
  -> backend loads context bundle
  -> backend enforces source Agent Profile
```

## Still required before production-ready claim

V13.2 still requires real release verification:

```bash
pnpm install
git add pnpm-lock.yaml
./scripts/release-check.sh
```

Then run manual smoke tests for fake runtime, managed llama.cpp, manual local model server, and one real coding-model patch on a throwaway repo or clean Git branch.

## Honest status

Synthesize v13.2 is a production-shaped release candidate. It should not be called a verified production build until the lockfile is committed, release checks pass, and a real local-model patch workflow succeeds.
