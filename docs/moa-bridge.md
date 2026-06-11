# Bundled MoA Bridge

Synthesize IDE now vendors the local MoA repository under `integrations/moa` and exposes a small bridge process at `integrations/moa/synthesize_bridge.py`.

The bridge is deliberately narrow:

- Synthesize remains the actor.
- The model still emits typed operations only.
- The bridge evaluates those operations with MoA's safety gate and returns structured approval data.
- Nothing in the bridge applies patches or executes commands directly.

## Commands

Send one JSON object per line over stdin.

### Health

```json
{"command":"health"}
```

### Verify bundled MoA

```json
{"command":"verify"}
```

This runs `verify_moa.py` with the current Python interpreter and returns stdout/stderr and the exit code.

### Evaluate one Synthesize operation

```json
{
  "command": "evaluate_operation",
  "operation": {
    "type": "propose_patch",
    "proposalId": "demo",
    "files": [{"path": "src/app.ts", "risk": "low"}]
  }
}
```

### Evaluate a batch

```json
{
  "command": "evaluate_batch",
  "operations": [
    {"type": "report", "summary": "Plan created."},
    {"type": "propose_patch", "proposalId": "demo", "files": [{"path": "src/app.ts", "risk": "low"}]}
  ]
}
```

## Current mapping

- `propose_patch` -> `write_code`
- `run_command` -> `run_code` or `access_network`
- `read_file` / `search_repo` -> `read_file`
- `report` / `final_report` / `ask_user` -> informational, approved without execution authority

This is the first integration slice. It is enough to bundle MoA into the repo and give Synthesize a stable local contract for future Rust or TypeScript wiring.