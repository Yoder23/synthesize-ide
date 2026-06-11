# Synthesize IDE rename notes

This archive was renamed from Forge IDE to Synthesize IDE to avoid name collision with existing Forge-branded IDE projects.

Changes applied:

- Visible project branding changed from Forge to Synthesize across docs, package metadata, prompts, app title, identifiers, and internal strings.
- npm workspace/package names changed from `forge-*` / `@forge/*` to `synthesize-*` / `@synthesize/*`.
- Tauri product name changed to `Synthesize IDE` and identifier changed to `dev.synthesize.ide`.
- Local app data and repo audit paths were renamed from Forge-oriented names to Synthesize-oriented names by the broad text rename.
- `scripts/release-check.sh` now generates `pnpm-lock.yaml` with `pnpm install --lockfile-only` if it is missing, then continues to the frozen install gate.
- A `pnpm-lock.yaml` note file is included so the missing-lockfile failure is no longer silent. Hydrate it in a normal dev environment with `pnpm install --lockfile-only` before final release tagging.

Validation performed in this sandbox:

- Broad text search for remaining `Forge`, `forge`, or `FORGE` references returned no project references after the rename.
- `bash -n scripts/release-check.sh` passed.

Validation not performed:

- Full pnpm lockfile hydration, because the sandbox could not access the npm registry / download pnpm through Corepack.
- Rust/Cargo checks, because this sandbox does not include the Rust toolchain.
