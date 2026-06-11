#!/usr/bin/env python3
"""
Synthesize IDE <-> MoA bridge.

This keeps the IDE as the primary shell while exposing MoA's safety gate as a
small JSON protocol over stdin/stdout. The bridge is intentionally narrow so it
can be launched locally, audited, and expanded without giving the model direct
execution authority.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, Tuple

ROOT = Path(__file__).resolve().parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from moa.ir import Action, ActionMeta, ActionType
from moa.safety import SafetyGate


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Run the Synthesize IDE MoA bridge")
    parser.add_argument("--self-test", action="store_true", help="run a local bridge self-test and exit")
    return parser


def operation_to_action(operation: Dict[str, Any]) -> Tuple[str, Action | None]:
    op_type = str(operation.get("type", "")).strip()
    if not op_type:
        raise ValueError("operation.type is required")

    if op_type in {"report", "final_report", "ask_user"}:
        return "informational", None

    if op_type in {"read_file", "search_repo"}:
        action = Action(
            type=ActionType.READ_FILE,
            args={"operation": op_type, **operation},
            metadata=ActionMeta(irreversibility_score=0.0, blast_radius=0.0, deception_risk=0.0),
        )
        return "gated", action

    if op_type == "propose_patch":
        files = operation.get("files", [])
        if not isinstance(files, list):
            raise ValueError("propose_patch.files must be an array")
        file_count = max(1, len(files))
        risk_labels = [str(item.get("risk", "low")).lower() for item in files if isinstance(item, dict)]
        high_risk = sum(label in {"high", "critical"} for label in risk_labels)
        irreversibility = min(0.15 + 0.1 * (file_count - 1) + 0.15 * high_risk, 0.95)
        blast_radius = min(0.1 + 0.08 * (file_count - 1) + 0.1 * high_risk, 0.95)
        action = Action(
            type=ActionType.WRITE_CODE,
            args={"operation": op_type, "file_count": file_count, **operation},
            metadata=ActionMeta(
                irreversibility_score=irreversibility,
                blast_radius=blast_radius,
                deception_risk=0.0,
                confirmation_required=file_count > 3 or high_risk > 0,
            ),
        )
        return "gated", action

    if op_type == "run_command":
        argv = operation.get("argv", [])
        if not isinstance(argv, list):
            raise ValueError("run_command.argv must be an array")
        requires_network = bool(operation.get("requiresNetwork", False))
        may_modify_files = bool(operation.get("mayModifyFiles", False))
        action_type = ActionType.ACCESS_NETWORK if requires_network else ActionType.RUN_CODE
        action = Action(
            type=action_type,
            args={"operation": op_type, "argv": argv, **operation},
            metadata=ActionMeta(
                irreversibility_score=0.55 if may_modify_files else 0.15,
                blast_radius=0.45 if may_modify_files else 0.15,
                deception_risk=0.0,
                confirmation_required=may_modify_files or requires_network,
            ),
        )
        return "gated", action

    raise ValueError(f"unsupported operation type: {op_type}")


def evaluate_operation(operation: Dict[str, Any]) -> Dict[str, Any]:
    category, action = operation_to_action(operation)
    if category == "informational":
        return {
            "ok": True,
            "category": category,
            "approved": True,
            "reason": "Informational operation; no execution authority requested.",
        }

    assert action is not None
    gate = SafetyGate()
    decision = gate.evaluate(action)
    return {
        "ok": True,
        "category": category,
        "approved": decision.approved,
        "reason": decision.rejection_reason or "approved",
        "confirmation_required": decision.confirmation_required,
        "audit_trail": decision.audit_trail,
        "action_type": action.type.value,
        "metadata": {
            "irreversibility_score": action.metadata.irreversibility_score if action.metadata else None,
            "blast_radius": action.metadata.blast_radius if action.metadata else None,
            "deception_risk": action.metadata.deception_risk if action.metadata else None,
        },
    }


def run_verify() -> Dict[str, Any]:
    proc = subprocess.run(
        [sys.executable, str(ROOT / "verify_moa.py")],
        capture_output=True,
        text=True,
        cwd=str(ROOT),
    )
    return {
        "ok": proc.returncode == 0,
        "returncode": proc.returncode,
        "stdout": proc.stdout,
        "stderr": proc.stderr,
    }


def handle_request(request: Dict[str, Any]) -> Dict[str, Any]:
    command = str(request.get("command", "")).strip()
    if command == "health":
        return {
            "ok": True,
            "service": "synthesize-moa-bridge",
            "protocol": "synthesize-moa-bridge/v1",
            "bridge_path": str(Path(__file__).resolve()),
            "available_commands": ["health", "verify", "evaluate_operation", "evaluate_batch"],
        }
    if command == "verify":
        return run_verify()
    if command == "evaluate_operation":
        operation = request.get("operation")
        if not isinstance(operation, dict):
            raise ValueError("evaluate_operation requires an object field named operation")
        return evaluate_operation(operation)
    if command == "evaluate_batch":
        operations = request.get("operations")
        if not isinstance(operations, list):
            raise ValueError("evaluate_batch requires an array field named operations")
        results = [evaluate_operation(op) for op in operations]
        return {
            "ok": True,
            "results": results,
            "all_approved": all(item.get("approved", False) for item in results),
        }
    raise ValueError(f"unsupported command: {command}")


def run_self_test() -> int:
    checks = [
        handle_request({"command": "health"}),
        handle_request(
            {
                "command": "evaluate_operation",
                "operation": {
                    "type": "propose_patch",
                    "proposalId": "self-test",
                    "files": [{"path": "src/app.ts", "risk": "low"}],
                },
            }
        ),
        handle_request(
            {
                "command": "evaluate_operation",
                "operation": {
                    "type": "run_command",
                    "argv": ["pytest"],
                    "requiresNetwork": False,
                    "mayModifyFiles": False,
                },
            }
        ),
    ]
    print(json.dumps({"ok": True, "checks": checks}, indent=2))
    return 0


def main() -> int:
    args = build_parser().parse_args()
    if args.self_test:
        return run_self_test()

    for raw_line in sys.stdin:
        line = raw_line.strip()
        if not line:
            continue
        try:
            payload = json.loads(line)
            response = handle_request(payload)
        except Exception as exc:  # pragma: no cover - defensive top-level bridge behavior
            response = {"ok": False, "error": str(exc)}
        sys.stdout.write(json.dumps(response) + "\n")
        sys.stdout.flush()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())