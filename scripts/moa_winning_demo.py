#!/usr/bin/env python3
"""
Synthesize/MoA competition demo.

Runs the winning loop without Ollama:
local GGUF model -> typed Synthesize operation -> MoA gate -> path/hash validation
-> checkpointed apply -> audit report -> high-risk action blocked.
"""

from __future__ import annotations

import argparse
import hashlib
import html
import json
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT / ".synthesize-runtime"
CONFIG = RUNTIME / "local-model.json"
DEMO_ROOT = RUNTIME / "winning-demo"
DEMO_REPO = DEMO_ROOT / "repo"
AUDIT = DEMO_ROOT / "audit.jsonl"
REPORT = DEMO_ROOT / "PRESENTATION_REPORT.md"
MISSION_CONTROL = DEMO_ROOT / "MISSION_CONTROL.html"
MOA_BRIDGE = ROOT / "integrations" / "moa" / "synthesize_bridge.py"


FIXTURE = """export function refreshToken() {
  throw new Error("not implemented");
}
"""


def now() -> str:
    return datetime.now(timezone.utc).isoformat()


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def audit(event: str, payload: dict[str, Any]) -> None:
    AUDIT.parent.mkdir(parents=True, exist_ok=True)
    with AUDIT.open("a", encoding="utf-8") as f:
        f.write(json.dumps({"ts": now(), "event": event, **payload}, sort_keys=True) + "\n")


def http_json(method: str, url: str, body: dict[str, Any] | None = None, timeout: int = 180) -> dict[str, Any]:
    data = None if body is None else json.dumps(body).encode("utf-8")
    req = urllib.request.Request(url, data=data, method=method, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read().decode("utf-8"))


def wait_ready(endpoint: str, seconds: int = 60) -> None:
    for _ in range(seconds):
        try:
            http_json("GET", endpoint.rstrip("/") + "/models", timeout=2)
            return
        except Exception:
            time.sleep(1)
    raise RuntimeError(f"local model server did not become ready at {endpoint}")


def start_model(config: dict[str, Any]) -> subprocess.Popen[str]:
    server = Path(config["llamaServerPath"])
    model = Path(config["modelPath"])
    if not server.exists():
        raise RuntimeError(f"llama-server.exe not found: {server}")
    if not model.exists():
        raise RuntimeError(f"GGUF model not found: {model}")

    log = RUNTIME / "winning-demo-llama.log"
    stdout = (RUNTIME / "winning-demo-llama.stdout.log").open("w", encoding="utf-8")
    stderr = (RUNTIME / "winning-demo-llama.stderr.log").open("w", encoding="utf-8")
    args = [
        str(server),
        "--model", str(model),
        "--host", "127.0.0.1",
        "--port", "8080",
        "--ctx-size", "2048",
        "--threads", "4",
        "--n-gpu-layers", "0",
        "--no-webui",
        "--alias", "local-gguf",
        "--log-file", str(log),
    ]
    proc = subprocess.Popen(args, cwd=str(server.parent), stdout=stdout, stderr=stderr, text=True)
    audit("runtime.starting", {"pid": proc.pid, "model": config.get("displayName"), "model_sha256": config.get("modelSha256")})
    wait_ready("http://127.0.0.1:8080/v1")
    audit("runtime.ready", {"endpoint": "http://127.0.0.1:8080/v1", "pid": proc.pid})
    return proc


def create_fixture_repo() -> tuple[Path, str]:
    if DEMO_REPO.exists():
        shutil.rmtree(DEMO_REPO)
    src = DEMO_REPO / "src" / "auth"
    src.mkdir(parents=True)
    path = src / "refresh.ts"
    path.write_text(FIXTURE, encoding="utf-8")
    before = sha256_text(FIXTURE)
    audit("repo.fixture_created", {"repo": str(DEMO_REPO), "file": "src/auth/refresh.ts", "before_sha256": before})
    return path, before


def ask_model(before_sha: str, goal: str) -> str:
    prompt = f"""
You are Synthesize IDE's MoA Action Planner.

Return only JSON. No markdown. No prose.

The model is not allowed to act directly. It may only emit typed Synthesize operations.

Task: {goal}
Path: src/auth/refresh.ts
beforeSha256: {before_sha}

Content:
{FIXTURE}

Return exactly one propose_patch operation with this shape:
{{
  "operations": [
    {{
      "type": "propose_patch",
      "proposalId": "winning-demo-refresh",
      "summary": "Short description of the repair.",
      "files": [
        {{
          "id": "winning-demo-refresh-file-001",
          "path": "src/auth/refresh.ts",
          "beforeSha256": "{before_sha}",
          "patch": "diff --git a/src/auth/refresh.ts b/src/auth/refresh.ts\\n--- a/src/auth/refresh.ts\\n+++ b/src/auth/refresh.ts\\n@@ -1,3 +1,3 @@\\n export function refreshToken() {{\\n-  throw new Error(\\\"not implemented\\\");\\n+  return \\\"refreshed\\\";\\n }}\\n"
        }}
      ],
      "riskNotes": ["Low-risk single-file fixture repair."],
      "suggestedCommands": [
        {{
          "type": "run_command",
          "argv": ["pnpm", "test", "auth"],
          "cwd": ".",
          "reason": "Verify the auth refresh repair.",
          "expectedOutcome": "Auth refresh tests pass.",
          "requiresNetwork": false,
          "mayModifyFiles": false
        }}
      ]
    }}
  ]
}}

Do not leave placeholder text in the patch. The added line must be a concrete TypeScript return statement.
"""
    context_hash = sha256_text(prompt)
    audit("context.bundle_created", {"context_sha256": context_hash, "exact_context": True})
    body = {
        "model": "local-gguf",
        "messages": [
            {"role": "system", "content": "Return only strict JSON containing Synthesize typed operations."},
            {"role": "user", "content": prompt},
        ],
        "temperature": 0.1,
        "max_tokens": 1000,
        "stream": False,
        "response_format": {"type": "json_object"},
    }
    started = time.perf_counter()
    resp = http_json("POST", "http://127.0.0.1:8080/v1/chat/completions", body)
    elapsed_ms = int((time.perf_counter() - started) * 1000)
    content = resp["choices"][0]["message"]["content"]
    audit("runtime.request_completed", {"duration_ms": elapsed_ms, "output_chars": len(content)})
    return content


def ask_model_to_repair_operation(raw: str, error: str, before_sha: str, goal: str) -> str:
    prompt = f"""
Your previous Synthesize operation was rejected by the trusted host.

Goal: {goal}
Validation error: {error}

Previous output:
{raw}

Return corrected JSON only. Do not use markdown.
The patch must be a real unified diff with one '-' removed line and one '+' added line.
Use beforeSha256 exactly: {before_sha}
Target file content:
{FIXTURE}
"""
    audit("operation.repair_requested", {"error": error, "previous_output_sha256": sha256_text(raw)})
    body = {
        "model": "local-gguf",
        "messages": [
            {"role": "system", "content": "Repair invalid Synthesize typed operations. Return only strict JSON."},
            {"role": "user", "content": prompt},
        ],
        "temperature": 0.1,
        "max_tokens": 1000,
        "stream": False,
        "response_format": {"type": "json_object"},
    }
    resp = http_json("POST", "http://127.0.0.1:8080/v1/chat/completions", body)
    content = resp["choices"][0]["message"]["content"]
    audit("operation.repair_completed", {"output_chars": len(content)})
    return content


def parse_operations(raw: str) -> list[dict[str, Any]]:
    text = raw.strip()
    if text.startswith("```"):
        text = text.strip("`")
        if text.lower().startswith("json"):
            text = text[4:].strip()
    parsed = json.loads(text)
    ops = parsed.get("operations")
    if not isinstance(ops, list) or not ops:
        raise RuntimeError("model did not return operations")
    audit("operation.parse_succeeded", {"count": len(ops), "operation_types": [op.get("type") for op in ops]})
    return ops


def moa_evaluate(operation: dict[str, Any]) -> dict[str, Any]:
    proc = subprocess.run(
        [sys.executable, str(MOA_BRIDGE)],
        input=json.dumps({"command": "evaluate_operation", "operation": operation}) + "\n",
        capture_output=True,
        text=True,
        cwd=str(ROOT),
        timeout=30,
    )
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr or proc.stdout)
    decision = json.loads(proc.stdout.strip())
    audit("moa.gate_decision", {"operation_type": operation.get("type"), "approved": decision.get("approved"), "reason": decision.get("reason"), "action_type": decision.get("action_type")})
    return decision


def validate_patch_operation(op: dict[str, Any], before_sha: str) -> dict[str, Any]:
    if op.get("type") != "propose_patch":
        raise RuntimeError(f"expected propose_patch, got {op.get('type')}")
    files = op.get("files")
    if not isinstance(files, list) or len(files) != 1:
        raise RuntimeError("demo expects exactly one patch file")
    file = files[0]
    rel = Path(str(file.get("path", "")))
    if rel.is_absolute() or ".." in rel.parts:
        raise RuntimeError(f"unsafe path rejected: {rel}")
    target = DEMO_REPO / rel
    if not target.exists():
        raise RuntimeError(f"target file does not exist: {rel}")
    actual = sha256_text(target.read_text(encoding="utf-8"))
    if file.get("beforeSha256") != before_sha or actual != before_sha:
        raise RuntimeError("beforeSha256 mismatch; refusing apply")
    audit("patch.validated", {"proposal_id": op.get("proposalId"), "file": str(rel), "before_sha256": before_sha})
    return file


def ensure_model_patch_is_structured(op: dict[str, Any]) -> None:
    files = op.get("files")
    if not isinstance(files, list) or not files:
        raise RuntimeError("model operation has no patch files")
    extract_single_line_replacement(str(files[0].get("patch", "")))


def apply_patch(file_op: dict[str, Any]) -> Path:
    rel = Path(file_op["path"])
    target = DEMO_REPO / rel
    checkpoint = DEMO_ROOT / "checkpoint" / rel
    checkpoint.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(target, checkpoint)

    old = target.read_text(encoding="utf-8")
    removed, added = extract_single_line_replacement(str(file_op.get("patch", "")))
    if removed not in old:
        raise RuntimeError(f"model patch removal line was not found in target: {removed!r}")
    new = old.replace(removed, added, 1)
    if old == new:
        raise RuntimeError("patch had no effect")
    target.write_text(new, encoding="utf-8")
    audit("patch.applied", {"file": str(rel), "checkpoint": str(checkpoint), "after_sha256": sha256_text(new)})
    return checkpoint


def rollback_patch(checkpoint: Path) -> None:
    rel = checkpoint.relative_to(DEMO_ROOT / "checkpoint")
    target = DEMO_REPO / rel
    before = target.read_text(encoding="utf-8")
    shutil.copy2(checkpoint, target)
    restored = target.read_text(encoding="utf-8")
    audit("patch.rolled_back", {
        "file": str(rel),
        "before_rollback_sha256": sha256_text(before),
        "restored_sha256": sha256_text(restored),
    })


def extract_single_line_replacement(patch: str) -> tuple[str, str]:
    removed: str | None = None
    added: str | None = None
    for line in patch.splitlines():
        if line.startswith("---") or line.startswith("+++") or line.startswith("@@"):
            continue
        if line.startswith("-") and removed is None:
            removed = line[1:]
        elif line.startswith("+") and added is None:
            added = line[1:]
    if removed is None or added is None:
        raise RuntimeError("demo patch must contain one removed line and one added line")
    return removed, added


def prove_blocked_action() -> dict[str, Any]:
    risky = {
        "type": "propose_patch",
        "proposalId": "judge-unsafe-blast-radius",
        "files": [
            {"path": "src/a.ts", "risk": "critical"},
            {"path": "src/b.ts", "risk": "critical"},
            {"path": "src/c.ts", "risk": "critical"},
            {"path": "src/d.ts", "risk": "critical"},
        ],
    }
    decision = moa_evaluate(risky)
    if decision.get("approved") is not False:
        raise RuntimeError("expected high-risk action to be rejected")
    audit("moa.high_risk_blocked", {"reason": decision.get("reason"), "metadata": decision.get("metadata")})
    return decision


def write_report(raw_model: str, op: dict[str, Any], blocked: dict[str, Any], goal: str, applied_text: str) -> None:
    final_file = DEMO_REPO / "src" / "auth" / "refresh.ts"
    events = [json.loads(line) for line in AUDIT.read_text(encoding="utf-8").splitlines() if line.strip()]
    REPORT.write_text(
        "\n".join([
            "# Synthesize/MoA Winning Demo Report",
            "",
            f"Goal: `{goal}`",
            "",
            "## What The Judges See",
            "",
            "A local GGUF coding model proposed a typed code action. It did not touch the filesystem.",
            "MoA evaluated the action. The trusted host validated path and before-hash, checkpointed the file, applied the change, and recorded every step.",
            "Then a high-risk multi-file action was rejected by MoA before it could enter the apply path. Finally, rollback restored the original file from checkpoint.",
            "",
            "## Proof Points",
            "",
            f"- Model operation: `{op.get('type')}` / `{op.get('proposalId')}`",
            f"- Final file SHA-256: `{sha256_text(final_file.read_text(encoding='utf-8'))}`",
            f"- High-risk action blocked: `{blocked.get('reason')}`",
            f"- Audit events recorded: `{len(events)}`",
            f"- Audit log: `{AUDIT}`",
            f"- Mission Control: `{MISSION_CONTROL}`",
            "",
            "## Final File",
            "",
            "After rollback, the repo is restored to its original state:",
            "",
            "```ts",
            final_file.read_text(encoding="utf-8").rstrip(),
            "```",
            "",
            "## Applied File Before Rollback",
            "",
            "```ts",
            applied_text.rstrip(),
            "```",
            "",
            "## Raw Model Output",
            "",
            "```json",
            raw_model.strip(),
            "```",
        ]),
        encoding="utf-8",
    )
    write_mission_control(raw_model, op, blocked, events, goal, applied_text)


def write_mission_control(raw_model: str, op: dict[str, Any], blocked: dict[str, Any], events: list[dict[str, Any]], goal: str, applied_text: str) -> None:
    safe_events = "\n".join(
        f"<li><strong>{html.escape(e.get('event', 'event'))}</strong><span>{html.escape(json.dumps(e, sort_keys=True))}</span></li>"
        for e in events
    )
    final_file = (DEMO_REPO / "src" / "auth" / "refresh.ts").read_text(encoding="utf-8")
    checkpoint_file = (DEMO_ROOT / "checkpoint" / "src" / "auth" / "refresh.ts").read_text(encoding="utf-8")
    MISSION_CONTROL.write_text(f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Synthesize/MoA Mission Control</title>
  <style>
    :root {{ color-scheme: dark; --bg:#0d1117; --panel:#151b23; --panel2:#1f2937; --text:#f4f7fb; --muted:#a8b3c2; --ok:#44d17d; --warn:#ffcc66; --bad:#ff6b6b; --line:#334155; --blue:#6cb6ff; }}
    * {{ box-sizing: border-box; }}
    body {{ margin:0; font-family:Segoe UI, Arial, sans-serif; background:var(--bg); color:var(--text); }}
    header {{ padding:28px 36px 18px; border-bottom:1px solid var(--line); }}
    h1 {{ margin:0; font-size:34px; letter-spacing:0; }}
    h2 {{ margin:0 0 10px; font-size:18px; }}
    p {{ color:var(--muted); line-height:1.45; }}
    .sub {{ color:var(--muted); margin-top:8px; max-width:1100px; line-height:1.45; }}
    .grid {{ display:grid; grid-template-columns:repeat(4,minmax(180px,1fr)); gap:14px; padding:22px 36px; }}
    .card {{ background:var(--panel); border:1px solid var(--line); border-radius:8px; padding:16px; min-height:150px; }}
    .big {{ font-size:28px; font-weight:700; }}
    .ok {{ color:var(--ok); }} .bad {{ color:var(--bad); }} .warn {{ color:var(--warn); }} .blue {{ color:var(--blue); }}
    .flow {{ display:grid; grid-template-columns:repeat(5,1fr); gap:10px; padding:0 36px 22px; }}
    .step {{ background:var(--panel2); border:1px solid var(--line); border-radius:8px; padding:14px; min-height:120px; }}
    .step strong {{ display:block; margin-bottom:8px; }}
    .step span {{ color:var(--muted); font-size:13px; line-height:1.35; }}
    .section {{ padding:0 36px 24px; }}
    .cols {{ display:grid; grid-template-columns:1fr 1fr; gap:14px; }}
    pre {{ margin:0; white-space:pre-wrap; overflow-wrap:anywhere; background:#0b1220; border:1px solid var(--line); border-radius:8px; padding:14px; color:#dbeafe; max-height:420px; overflow:auto; }}
    ol {{ margin:0; padding-left:22px; }}
    li {{ margin:8px 0; }}
    li span {{ display:block; color:var(--muted); font-size:12px; overflow-wrap:anywhere; }}
    @media (max-width:900px) {{ .grid,.flow,.cols {{ grid-template-columns:1fr; }} header,.grid,.flow,.section {{ padding-left:18px; padding-right:18px; }} }}
  </style>
</head>
<body>
  <header>
    <h1>Synthesize/MoA Mission Control</h1>
    <div class="sub">A local model proposed an action. MoA gated it. The trusted host validated, checkpointed, applied, blocked an unsafe alternate action, rolled back, and wrote an audit trail. Goal: <strong>{html.escape(goal)}</strong></div>
  </header>
  <section class="grid">
    <div class="card"><h2>Local Model</h2><div class="big blue">GGUF</div><p>Qwen2.5 Coder via local llama.cpp. No Ollama. No cloud account.</p></div>
    <div class="card"><h2>Safe Action</h2><div class="big ok">Approved</div><p>{html.escape(str(op.get("proposalId")))}</p></div>
    <div class="card"><h2>Unsafe Action</h2><div class="big bad">Blocked</div><p>{html.escape(str(blocked.get("reason")))}</p></div>
    <div class="card"><h2>Audit Events</h2><div class="big warn">{len(events)}</div><p>Every transition is recorded in JSONL.</p></div>
  </section>
  <section class="flow">
    <div class="step"><strong>1. Observe</strong><span>Backend-created context with before-hash.</span></div>
    <div class="step"><strong>2. Propose</strong><span>Local model emits typed Synthesize operation.</span></div>
    <div class="step"><strong>3. Gate</strong><span>MoA approves safe action and rejects risky action.</span></div>
    <div class="step"><strong>4. Act</strong><span>Trusted host validates path/hash and applies from checkpoint.</span></div>
    <div class="step"><strong>5. Rewind</strong><span>Rollback restores original file from checkpoint.</span></div>
  </section>
  <section class="section cols">
    <div><h2>Raw Model Proposal</h2><pre>{html.escape(raw_model.strip())}</pre></div>
    <div><h2>Applied File Before Rollback</h2><pre>{html.escape(applied_text.rstrip())}</pre><h2 style="margin-top:14px;">Final File After Rollback</h2><pre>{html.escape(final_file.rstrip())}</pre><h2 style="margin-top:14px;">Checkpoint</h2><pre>{html.escape(checkpoint_file.rstrip())}</pre></div>
  </section>
  <section class="section"><h2>Audit Flight Recorder</h2><ol>{safe_events}</ol></section>
</body>
</html>
""", encoding="utf-8")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Run the Synthesize/MoA competition Mission Control demo.")
    parser.add_argument(
        "--goal",
        default="Repair refreshToken so the auth flow returns a stable success token instead of throwing.",
        help="Goal sent to the local model.",
    )
    return parser


def main() -> int:
    args = build_parser().parse_args()
    if not CONFIG.exists():
        raise RuntimeError("Missing .synthesize-runtime/local-model.json. Run scripts/bootstrap-local-model.ps1 -Model coder-1.5b first.")
    if DEMO_ROOT.exists():
        shutil.rmtree(DEMO_ROOT)
    DEMO_ROOT.mkdir(parents=True)

    config = json.loads(CONFIG.read_text(encoding="utf-8-sig"))
    proc: subprocess.Popen[str] | None = None
    try:
        target, before_sha = create_fixture_repo()
        proc = start_model(config)
        raw = ask_model(before_sha, args.goal)
        print("\n=== LOCAL MODEL PROPOSED ===")
        print(raw.strip())
        ops = parse_operations(raw)
        op = ops[0]
        try:
            ensure_model_patch_is_structured(op)
        except RuntimeError as exc:
            print("\n=== TRUSTED HOST REJECTED MALFORMED OPERATION ===")
            print(str(exc))
            raw = ask_model_to_repair_operation(raw, str(exc), before_sha, args.goal)
            print("\n=== LOCAL MODEL REPAIRED OPERATION ===")
            print(raw.strip())
            ops = parse_operations(raw)
            op = ops[0]
            ensure_model_patch_is_structured(op)
        decision = moa_evaluate(op)
        if decision.get("approved") is not True:
            raise RuntimeError(f"MoA rejected model operation: {decision}")
        file_op = validate_patch_operation(op, before_sha)
        checkpoint = apply_patch(file_op)
        applied_text = target.read_text(encoding="utf-8")
        blocked = prove_blocked_action()
        rollback_patch(checkpoint)
        rollback_text = target.read_text(encoding="utf-8")
        write_report(raw, op, blocked, args.goal, applied_text)

        print("\n=== TRUSTED ACTION RESULT ===")
        print(applied_text.strip())
        print("\n=== ROLLBACK RESULT ===")
        print(rollback_text.strip())
        print("\n=== SAFETY PROOF ===")
        print(f"MoA blocked high-risk action: {blocked.get('reason')}")
        print("\n=== AUDIT ===")
        print(AUDIT)
        print(REPORT)
        print(MISSION_CONTROL)
        print("\nPASS: local model + MoA + audited checkpointed action loop is operational.")
        audit("demo.pass", {"report": str(REPORT), "checkpoint": str(checkpoint)})
        return 0
    finally:
        if proc and proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()


if __name__ == "__main__":
    raise SystemExit(main())
