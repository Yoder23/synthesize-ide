# Synthesize v13 Production-Build Candidate

V13 is a production-readiness tightening pass on the v12.1 release candidate. It does not add new major product surface. It focuses on reducing release risk and aligning UI behavior with backend policy.

## Changes

- Tightened local agent prompts so Planner and Reviewer profiles are clearly report-only and do not ask models to emit patch proposals.
- Kept backend Agent Profile enforcement as the source of truth: Planner/Reviewer cannot validate patch proposals.
- Updated Diff Queue UX so patch validation is disabled for report-only Agent Profiles and the user is directed to switch to Local Patcher.
- Adjusted local model server request bodies so Synthesize does not send `response_format: null` to strict OpenAI-compatible local HTTP servers.
- Added milestone-neutral production-build-candidate language in current docs.
- Added `docs/production-readiness.md` with exact release requirements and acceptance criteria.
- Updated `scripts/release-check.sh` to fail fast with a clear message when `pnpm-lock.yaml` is missing.

## Status

V13 is the production-build candidate that should be validated in a real Rust + pnpm environment. It is not honestly production-ready until:

- `pnpm-lock.yaml` is generated and committed,
- `./scripts/release-check.sh` passes,
- fake runtime QA passes,
- managed llama.cpp QA passes,
- manual local server QA passes,
- at least one real local-model patch workflow passes on a throwaway repo or clean branch.

## Target claim after successful verification

Synthesize is production-shaped daily-use ready for personal/local repos on clean Git branches with self-hosted open-source coding models.
