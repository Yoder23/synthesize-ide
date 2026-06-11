import pytest
from moa.ir import (
    Modality, IntentType, ActionType, EvidenceSource,
    Claim, Evidence, Action, ActionMeta, Intent,
    IRObject, Entity, MemoryWrite, Counterfactual, ReasoningTrace,
    create_claim, create_evidence,
)


def test_ir_object_gets_uuid():
    a = IRObject()
    b = IRObject()
    assert a.id != b.id
    assert len(a.id) == 36  # UUID format


def test_claim_defaults():
    c = create_claim("x", "is", "y")
    assert c.modality == Modality.HYPOTHESIS
    assert c.confidence == 0.5
    assert not c.is_verified()


def test_claim_verified_requires_all_three():
    c = Claim(subject="a", predicate="b", object="c",
              modality=Modality.FACT, confidence=0.9)
    # Missing evidence_ids — not verified yet
    assert not c.is_verified()

    c.evidence_ids.append("ev-1")
    assert c.is_verified()


def test_claim_not_verified_if_confidence_low():
    c = Claim(subject="a", predicate="b", object="c",
              modality=Modality.FACT, confidence=0.7,
              evidence_ids=["ev-1"])
    assert not c.is_verified()


def test_claim_not_verified_if_simulation():
    c = Claim(subject="a", predicate="b", object="c",
              modality=Modality.SIMULATION, confidence=0.99,
              evidence_ids=["ev-1"])
    assert not c.is_verified()


def test_evidence_links_to_claim():
    c = create_claim("a", "b", "c")
    e = create_evidence(c, EvidenceSource.EXECUTION_TRACE, "f.py:1", "output", 1.0)
    assert e.claim_id == c.id
    assert e.is_supporting is True


def test_action_meta_safe():
    m = ActionMeta(irreversibility_score=0.1, blast_radius=0.1,
                   deception_risk=0.0, permission_delta=0)
    assert m.is_safe()


def test_action_meta_not_safe_irreversible():
    m = ActionMeta(irreversibility_score=0.9)
    assert not m.is_safe()


def test_action_meta_not_safe_deception():
    m = ActionMeta(irreversibility_score=0.1, blast_radius=0.1, deception_risk=0.5)
    assert not m.is_safe()


def test_action_meta_forbidden_flag():
    m = ActionMeta(forbidden=True, irreversibility_score=0.0, blast_radius=0.0)
    assert not m.is_safe()


def test_modality_enum_values():
    assert Modality.FACT.value == "fact"
    assert Modality.SIMULATION.value == "simulation"


def test_action_type_forbidden_exist():
    assert ActionType.DECEIVE_USER in ActionType
    assert ActionType.MANIPULATE_SOCIAL in ActionType
    assert ActionType.ESCALATE_PERMISSION in ActionType


def test_counterfactual_repr():
    cf = Counterfactual(scenario="test", human_valence=0.5, importance=0.8)
    assert "0.50" in repr(cf)


def test_reasoning_trace_repr():
    rt = ReasoningTrace(reasoning_steps=["step1", "step2"], confidence=0.75)
    assert "2 steps" in repr(rt)
    assert "0.75" in repr(rt)
