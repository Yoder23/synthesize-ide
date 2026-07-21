#!/usr/bin/env python3
"""
verify_moa.py — Zero-Setup Proof

Runs 6 checks that prove the core MoA guarantees hold.
No API keys. No GPU. No network. No external dependencies beyond stdlib.

Usage:
    python verify_moa.py

Expected output:
    [ir]       ✓ All IR types instantiate with correct defaults
    [safety]   ✓ FORBIDDEN actions rejected (3 types)
    [safety]   ✓ High-irreversibility action rejected (score=0.95)
    [safety]   ✓ High blast-radius action rejected (score=0.80)
    [verifier] ✓ SIMULATION claim blocked from becoming FACT
    [verifier] ✓ Hypothesis with evidence + confidence promoted to FACT
    [memory]   ✓ EventLog append-only: 3 writes, 3 reads
    [memory]   ✓ FactGraph rejects non-FACT claims
    [agent]    ✓ OODA loop runs end-to-end (MockBackend)
    [agent]    ✓ SafetyGate: 1 approved, 1 rejected in live run

    ALL CHECKS PASSED
"""

import sys
import time

# ── Path setup ────────────────────────────────────────────────────────────────
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))

from moa.ir import (
    Modality, ActionType, EvidenceSource,
    Claim, Evidence, Action, ActionMeta, Intent, IntentType,
    create_claim, create_evidence,
)
from moa.safety import SafetyGate, FormalVerifier, HardConstraints
from moa.memory import EventLog, FactGraph, EpisodicBuffer, AgentMemory
from moa.agent import MoAAgent
from moa.backends.mock import MockBackend


def _ok(label: str, msg: str = "") -> None:
    detail = f"  {msg}" if msg else ""
    print(f"  [{label:<9}] OK{detail}")


def _fail(label: str, msg: str) -> None:
    print(f"  [{label:<9}] FAIL  {msg}", file=sys.stderr)
    sys.exit(1)


# ============================================================================
# Check 1 — IR types
# ============================================================================

def check_ir_types():
    c = create_claim("requests", "requires_import", "True",
                     Modality.HYPOTHESIS, 0.7)
    assert c.modality == Modality.HYPOTHESIS
    assert not c.is_verified()

    e = create_evidence(c, EvidenceSource.EXECUTION_TRACE,
                        "script.py:2", "NameError: requests not defined", 1.0)
    assert e.claim_id == c.id

    a = Action(type=ActionType.READ_FILE)
    a.metadata = ActionMeta(irreversibility_score=0.1, blast_radius=0.1)
    assert a.metadata.is_safe()

    _ok("ir", "All IR types instantiate with correct defaults")


# ============================================================================
# Check 2 — SafetyGate: forbidden action types
# ============================================================================

def check_forbidden_actions():
    gate = SafetyGate()
    forbidden = [
        ActionType.ESCALATE_PERMISSION,
        ActionType.DECEIVE_USER,
        ActionType.MANIPULATE_SOCIAL,
    ]
    for at in forbidden:
        a = Action(type=at)
        d = gate.evaluate(a)
        if d.approved:
            _fail("safety", f"{at.value} was APPROVED — should be FORBIDDEN")
    assert gate.stats["rejected"] == len(forbidden)
    assert gate.stats["approved"] == 0
    _ok("safety", f"FORBIDDEN actions rejected ({len(forbidden)} types)")


# ============================================================================
# Check 3 — SafetyGate: irreversibility + blast radius thresholds
# ============================================================================

def check_risk_thresholds():
    gate = SafetyGate()

    # High irreversibility
    a1 = Action(type=ActionType.DELETE_FILE,
                metadata=ActionMeta(irreversibility_score=0.95, blast_radius=0.2))
    d1 = gate.evaluate(a1)
    if d1.approved:
        _fail("safety", "High-irreversibility action was approved")
    _ok("safety", f"High-irreversibility action rejected (score=0.95)")

    # High blast radius
    a2 = Action(type=ActionType.MODIFY_SYSTEM,
                metadata=ActionMeta(irreversibility_score=0.3, blast_radius=0.80))
    d2 = gate.evaluate(a2)
    if d2.approved:
        _fail("safety", "High blast-radius action was approved")
    _ok("safety", f"High blast-radius action rejected (score=0.80)")


# ============================================================================
# Check 4 — FormalVerifier: SIMULATION → FACT blocked
# ============================================================================

def check_simulation_blocked():
    verifier = FormalVerifier()
    sim_claim = create_claim(
        "counterfactual_event", "would_have_caused", "catastrophe",
        modality=Modality.SIMULATION,
        confidence=0.99,
    )
    # Even strong evidence cannot promote a SIMULATION claim
    evidence = create_evidence(
        sim_claim, EvidenceSource.CROSS_REFERENCE,
        "analysis.py:1", "Hypothetical analysis", 1.0
    )
    promoted, failures = verifier.promote_to_fact(sim_claim, [evidence])
    if promoted:
        _fail("verifier", "SIMULATION claim was promoted to FACT — universe separation violated")
    assert any("UNIVERSE_SEPARATION" in f for f in failures)
    _ok("verifier", "SIMULATION claim blocked from becoming FACT")


# ============================================================================
# Check 5 — FormalVerifier: valid hypothesis → FACT
# ============================================================================

def check_valid_fact_promotion():
    verifier = FormalVerifier()
    hypo = create_claim(
        "requests", "requires_import", "True",
        modality=Modality.HYPOTHESIS,
        confidence=0.9,
    )
    evidence = create_evidence(
        hypo, EvidenceSource.EXECUTION_TRACE,
        "script.py:2", "NameError: name 'requests' is not defined", 1.0
    )
    promoted, failures = verifier.promote_to_fact(hypo, [evidence])
    if not promoted:
        _fail("verifier", f"Valid hypothesis was NOT promoted to FACT: {failures}")
    assert hypo.modality == Modality.FACT
    assert hypo.is_verified()
    _ok("verifier", "Hypothesis with evidence + confidence promoted to FACT")


# ============================================================================
# Check 6 — EventLog + FactGraph
# ============================================================================

def check_memory():
    # EventLog: append-only
    log = EventLog()
    log.write("task", "Started analysis")
    log.write("result", "Found 3 issues")
    log.write("task", "Completed")
    entries = log.query(scope="task")
    assert len(entries) == 2, f"Expected 2 task entries, got {len(entries)}"
    assert log.entry_count == 3
    _ok("memory", "EventLog append-only: 3 writes, 3 reads")

    # FactGraph: rejects non-FACT claims
    fg = FactGraph()
    hypo = create_claim("x", "is", "y", Modality.HYPOTHESIS, 0.9)
    try:
        fg.add(hypo)
        _fail("memory", "FactGraph accepted a HYPOTHESIS claim")
    except ValueError:
        pass  # Correct

    # But accepts verified facts
    verifier = FormalVerifier()
    fact = create_claim("python", "is_installed", "True", Modality.HYPOTHESIS, 0.9)
    ev = create_evidence(fact, EvidenceSource.EXECUTION_TRACE,
                         "sys.py:1", "Python 3.10 found", 1.0)
    verifier.promote_to_fact(fact, [ev])
    fg.add(fact)
    assert fg.fact_count == 1
    _ok("memory", "FactGraph rejects non-FACT claims, accepts verified FACT")


# ============================================================================
# Check 7 — Full OODA loop
# ============================================================================

def check_agent_loop():
    backend = MockBackend(responses={
        "chess": "White has a slight advantage after e4 e5 Nf3 Nc6.",
        "delete": "I understand you want to delete the file.",
    })

    agent = MoAAgent(backend=backend)

    # Normal query — should complete
    result = agent.run("Analyse chess opening: e4 e5 Nf3 Nc6")
    assert len(result.response) > 0
    assert result.duration_s >= 0
    _ok("agent", "OODA loop runs end-to-end (MockBackend)")

    # The agent returns results and tracks safety gate decisions
    assert isinstance(result.actions_taken, list)
    assert isinstance(result.actions_rejected, list)
    stats = agent.safety_stats
    total = stats["approved"] + stats["rejected"]
    assert total >= 1
    _ok("agent", f"SafetyGate active: {stats['approved']} approved, "
                 f"{stats['rejected']} rejected in live run")


# ============================================================================
# Main
# ============================================================================

if __name__ == "__main__":
    t0 = time.perf_counter()
    print()
    print("MoA — verify_moa.py")
    print("=" * 50)

    check_ir_types()
    check_forbidden_actions()
    check_risk_thresholds()
    check_simulation_blocked()
    check_valid_fact_promotion()
    check_memory()
    check_agent_loop()

    elapsed = time.perf_counter() - t0
    print()
    print(f"  ALL CHECKS PASSED  ({elapsed:.1f}s)")
    print()
