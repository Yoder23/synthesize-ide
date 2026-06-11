import pytest
from moa.ir import ActionType, Action, ActionMeta, Modality, create_claim, create_evidence, EvidenceSource
from moa.safety import SafetyGate, FormalVerifier, HardConstraints


# ── HardConstraints ────────────────────────────────────────────────────────

def test_hard_constraints_immutable():
    """HardConstraints values cannot be reassigned at runtime."""
    assert HardConstraints.MAX_IRREVERSIBILITY == 0.8
    assert HardConstraints.MAX_BLAST_RADIUS == 0.7
    assert HardConstraints.MAX_DECEPTION_RISK == 0.1
    # frozenset — cannot be mutated
    assert isinstance(HardConstraints.FORBIDDEN_ACTIONS, frozenset)


def test_forbidden_actions_set():
    forbidden = HardConstraints.FORBIDDEN_ACTIONS
    assert ActionType.DECEIVE_USER in forbidden
    assert ActionType.ESCALATE_PERMISSION in forbidden
    assert ActionType.MANIPULATE_SOCIAL in forbidden
    # Non-forbidden action not in set
    assert ActionType.READ_FILE not in forbidden


# ── SafetyGate ─────────────────────────────────────────────────────────────

def test_gate_approves_safe_read():
    gate = SafetyGate()
    a = Action(type=ActionType.READ_FILE,
               metadata=ActionMeta(irreversibility_score=0.0, blast_radius=0.1))
    d = gate.evaluate(a)
    assert d.approved
    assert gate.stats["approved"] == 1


def test_gate_rejects_deceive():
    gate = SafetyGate()
    a = Action(type=ActionType.DECEIVE_USER)
    d = gate.evaluate(a)
    assert not d.approved
    assert "FORBIDDEN" in d.rejection_reason.upper() or "forbidden" in d.rejection_reason.lower()


def test_gate_rejects_escalate():
    gate = SafetyGate()
    a = Action(type=ActionType.ESCALATE_PERMISSION)
    d = gate.evaluate(a)
    assert not d.approved


def test_gate_rejects_manipulate():
    gate = SafetyGate()
    a = Action(type=ActionType.MANIPULATE_SOCIAL)
    d = gate.evaluate(a)
    assert not d.approved


def test_gate_rejects_high_irreversibility():
    gate = SafetyGate()
    a = Action(type=ActionType.DELETE_FILE,
               metadata=ActionMeta(irreversibility_score=0.85, blast_radius=0.1))
    d = gate.evaluate(a)
    assert not d.approved


def test_gate_rejects_high_blast_radius():
    gate = SafetyGate()
    a = Action(type=ActionType.MODIFY_SYSTEM,
               metadata=ActionMeta(irreversibility_score=0.3, blast_radius=0.75))
    d = gate.evaluate(a)
    assert not d.approved


def test_gate_rejects_high_deception():
    gate = SafetyGate()
    a = Action(type=ActionType.READ_FILE,
               metadata=ActionMeta(irreversibility_score=0.0, blast_radius=0.0,
                                    deception_risk=0.5))
    d = gate.evaluate(a)
    assert not d.approved


def test_gate_audit_trail_populated():
    gate = SafetyGate()
    a = Action(type=ActionType.READ_FILE,
               metadata=ActionMeta(irreversibility_score=0.0, blast_radius=0.0))
    d = gate.evaluate(a)
    assert isinstance(d.audit_trail, list)
    assert len(d.audit_trail) > 0


def test_gate_stats_accumulate():
    gate = SafetyGate()
    safe = Action(type=ActionType.READ_FILE,
                  metadata=ActionMeta(irreversibility_score=0.0, blast_radius=0.0))
    risky = Action(type=ActionType.DECEIVE_USER)
    gate.evaluate(safe)
    gate.evaluate(risky)
    assert gate.stats["approved"] == 1
    assert gate.stats["rejected"] == 1


# ── FormalVerifier ─────────────────────────────────────────────────────────

def test_verifier_rejects_no_evidence():
    v = FormalVerifier()
    c = create_claim("a", "b", "c", Modality.HYPOTHESIS, 0.9)
    promoted, failures = v.promote_to_fact(c, [])
    assert not promoted
    assert any("evidence" in f.lower() for f in failures)


def test_verifier_rejects_low_confidence():
    v = FormalVerifier()
    c = create_claim("a", "b", "c", Modality.HYPOTHESIS, 0.5)
    ev = create_evidence(c, EvidenceSource.EXECUTION_TRACE, "f.py:1", "data", 0.9)
    promoted, failures = v.promote_to_fact(c, [ev])
    assert not promoted
    assert any("confidence" in f.lower() for f in failures)


def test_verifier_rejects_simulation():
    v = FormalVerifier()
    c = create_claim("a", "b", "c", Modality.SIMULATION, 0.99)
    ev = create_evidence(c, EvidenceSource.EXECUTION_TRACE, "f.py:1", "data", 1.0)
    promoted, failures = v.promote_to_fact(c, [ev])
    assert not promoted
    assert any("UNIVERSE_SEPARATION" in f.upper() or "simulation" in f.lower()
               for f in failures)


def test_verifier_rejects_fiction():
    v = FormalVerifier()
    c = create_claim("a", "b", "c", Modality.FICTION, 0.99)
    ev = create_evidence(c, EvidenceSource.EXECUTION_TRACE, "f.py:1", "data", 1.0)
    promoted, failures = v.promote_to_fact(c, [ev])
    assert not promoted


def test_verifier_promotes_valid_hypothesis():
    v = FormalVerifier()
    c = create_claim("requests", "requires_import", "True",
                     Modality.HYPOTHESIS, 0.9)
    ev = create_evidence(c, EvidenceSource.EXECUTION_TRACE,
                         "script.py:2", "NameError: requests not defined", 1.0)
    promoted, failures = v.promote_to_fact(c, [ev])
    assert promoted
    assert c.modality == Modality.FACT
    assert c.is_verified()
    assert failures == []


def test_verifier_fact_store_grows():
    v = FormalVerifier()
    assert len(v.fact_store) == 0
    c = create_claim("a", "b", "c", Modality.HYPOTHESIS, 0.9)
    ev = create_evidence(c, EvidenceSource.EXECUTION_TRACE, "f.py:1", "d", 1.0)
    v.promote_to_fact(c, [ev])
    assert len(v.fact_store) == 1
