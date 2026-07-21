import json
import subprocess
import sys
from pathlib import Path

from synthesize_bridge import evaluate_operation, handle_request


BRIDGE = Path(__file__).resolve().parents[1] / "synthesize_bridge.py"


def test_health_reports_protocol():
    result = handle_request({"command": "health"})
    assert result["ok"] is True
    assert result["protocol"] == "synthesize-moa-bridge/v1"


def test_patch_operation_is_gated_and_allowed_when_small():
    result = evaluate_operation(
        {
            "type": "propose_patch",
            "proposalId": "p1",
            "files": [{"path": "src/app.ts", "risk": "low"}],
        }
    )
    assert result["ok"] is True
    assert result["category"] == "gated"
    assert result["approved"] is True


def test_informational_operation_skips_gate():
    result = evaluate_operation({"type": "report", "summary": "No change proposed."})
    assert result["approved"] is True
    assert result["category"] == "informational"


def test_network_command_requires_confirmation_but_can_pass():
    result = evaluate_operation(
        {
            "type": "run_command",
            "argv": ["curl", "https://example.com"],
            "requiresNetwork": True,
            "mayModifyFiles": False,
        }
    )
    assert result["ok"] is True
    assert result["action_type"] == "access_network"


def test_verify_command_runs_bundled_moa_proof():
    result = handle_request({"command": "verify"})
    assert result["ok"] is True
    assert result["returncode"] == 0
    assert "ALL CHECKS PASSED" in result["stdout"]


def test_high_risk_patch_is_rejected_by_moa_gate():
    result = evaluate_operation(
        {
            "type": "propose_patch",
            "proposalId": "risky",
            "files": [
                {"path": "src/a.ts", "risk": "critical"},
                {"path": "src/b.ts", "risk": "critical"},
                {"path": "src/c.ts", "risk": "critical"},
                {"path": "src/d.ts", "risk": "critical"},
            ],
        }
    )
    assert result["ok"] is True
    assert result["approved"] is False
    assert result["action_type"] == "write_code"
    assert result["metadata"]["irreversibility_score"] > 0.8


def test_bridge_stdin_json_protocol_round_trip():
    proc = subprocess.run(
        [sys.executable, str(BRIDGE)],
        input=json.dumps({"command": "health"}) + "\n",
        capture_output=True,
        text=True,
        timeout=10,
    )
    assert proc.returncode == 0
    response = json.loads(proc.stdout.strip())
    assert response["ok"] is True
    assert response["protocol"] == "synthesize-moa-bridge/v1"
