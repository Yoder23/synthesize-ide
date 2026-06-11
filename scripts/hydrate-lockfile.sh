#!/usr/bin/env bash
set -euo pipefail

if ! command -v pnpm >/dev/null 2>&1; then
  echo "pnpm is required. Try: corepack enable && corepack prepare pnpm@9.15.0 --activate" >&2
  exit 127
fi

pnpm install --lockfile-only
pnpm install --frozen-lockfile
