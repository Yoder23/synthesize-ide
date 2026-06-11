#!/usr/bin/env bash
set -euo pipefail

missing=0
for tool in cargo pnpm; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "missing required release tool: $tool" >&2
    missing=1
  fi
done
if [ "$missing" -ne 0 ]; then
  echo "Install Rust/Cargo and pnpm before running the release gate." >&2
  exit 127
fi

if [ ! -f pnpm-lock.yaml ] || grep -q "Synthesize IDE lockfile note" pnpm-lock.yaml; then
  echo "pnpm-lock.yaml is missing or placeholder-only; generating it with 'pnpm install --lockfile-only' before the release gate." >&2
  pnpm install --lockfile-only
fi

cargo check --workspace
cargo test --workspace
pnpm install --frozen-lockfile
pnpm typecheck
pnpm build
pnpm test
