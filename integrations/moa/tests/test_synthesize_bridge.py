from synthesize_bridge import evaluate_operation, handle_request


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
