"""
MoA Safety Gate
================
Hard-constraint action gating and formal verification.

Design: Safety is ARCHITECTURE, not prompting.
  - Hard constraints are defined as Python constants, not system prompts.
  - A FORBIDDEN action is rejected before it reaches the LLM.
  - A claim originating in SIMULATION is blocked from becoming FACT by code.
  - This cannot be "jailbroken" by a clever user prompt.

Zero external dependencies — stdlib only.
"""

from __future__ import annotations

import time
import logging
from dataclasses import dataclass, field
from typing import List, Optional, Tuple, Dict, Any

from .ir import (
    Action, ActionType, ActionMeta, Claim, Evidence, Modality,
    EvidenceSource, create_claim,
)

log = logging.getLogger(__name__)


# ============================================================================
# Hard Constraints
# ============================================================================

class HardConstraints:
    """
    Immutable safety thresholds.

    These are not configurable at runtime. The only way to change them is to
    edit this file — making any weakening an explicit, auditable code change.
    """
    # Action scoring limits
    MAX_IRREVERSIBILITY: float = 0.8   # Actions with score > this are rejected
    MAX_BLAST_RADIUS:    float = 0.7   # Actions affecting > 70% of system rejected
    MAX_DECEPTION_RISK:  float = 0.1   # Actions with > 10% deception risk rejected

    # Architecturally forbidden action types — always rejected, no exceptions
    FORBIDDEN_ACTIONS: frozenset = frozenset({
        ActionType.ESCALATE_PERMISSION,
        ActionType.DECEIVE_USER,
        ActionType.MANIPULATE_SOCIAL,
    })

    # Claim verification requirements
    MIN_CONFIDENCE_FOR_FACT: float = 0.8
    MIN_EVIDENCE_FOR_FACT:   int   = 1

    # Simulation isolation — SIMULATION claims can NEVER become FACT
    SIMULATION_PROMOTION_FORBIDDEN: bool = True


# ============================================================================
# Formal Verifier
# ============================================================================

class FormalVerifier:
    """
    Five-check verifier that a Claim must pass before it becomes FACT.

    Check 1: Must have at least one piece of evidence.
    Check 2: Confidence must be >= MIN_CONFIDENCE_FOR_FACT (0.8).
    Check 3: Must not contradict existing verified facts.
    Check 4: Must not originate from SIMULATION universe.
    Check 5: High confidence without evidence is flagged as hallucination risk.
    """

    def __init__(self):
        self._verified_facts: Dict[str, Claim] = {}

    def verify(
        self,
        claim: Claim,
        evidence: List[Evidence],
        existing_facts: Optional[Dict[str, Claim]] = None,
    ) -> Tuple[bool, List[str]]:
        """
        Run all five checks.

        Returns:
            (passed, list_of_failure_messages)
            If passed is True, the claim may be promoted to FACT.
        """
        failures: List[str] = []
        facts = existing_facts or self._verified_facts

        # Check 1 — Evidence required
        supporting = [e for e in evidence if e.is_supporting and e.claim_id == claim.id]
        if len(supporting) < HardConstraints.MIN_EVIDENCE_FOR_FACT:
            failures.append(
                f"EVIDENCE: needs {HardConstraints.MIN_EVIDENCE_FOR_FACT} "
                f"supporting piece(s), has {len(supporting)}"
            )

        # Check 2 — Confidence threshold
        if claim.confidence < HardConstraints.MIN_CONFIDENCE_FOR_FACT:
            failures.append(
                f"CONFIDENCE: {claim.confidence:.2f} < "
                f"{HardConstraints.MIN_CONFIDENCE_FOR_FACT}"
            )

        # Check 3 — No contradiction with existing verified facts
        for fid in claim.contradicts:
            if fid in facts:
                failures.append(
                    f"CONTRADICTION: claim contradicts verified fact {fid!r}"
                )

        # Check 4 — Universe separation (SIMULATION / FICTION → FACT FORBIDDEN)
        if claim.modality in (Modality.SIMULATION, Modality.FICTION):
            failures.append(
                f"UNIVERSE_SEPARATION: {claim.modality.value.upper()} claims cannot become FACT"
            )

        # Check 5 — Hallucination detector: high confidence with no evidence
        if claim.confidence > 0.95 and len(supporting) == 0:
            failures.append(
                "HALLUCINATION_RISK: confidence > 0.95 with zero evidence"
            )

        passed = len(failures) == 0
        return passed, failures

    def promote_to_fact(
        self,
        claim: Claim,
        evidence: List[Evidence],
    ) -> Tuple[bool, List[str]]:
        """
        Attempt to promote a Claim to FACT.
        Records it in the internal verified-facts store if successful.

        Returns:
            (promoted, failure_reasons)
        """
        passed, failures = self.verify(claim, evidence)
        if passed:
            claim.modality = Modality.FACT
            for e in evidence:
                if e.is_supporting and e.claim_id == claim.id:
                    if e.id not in claim.evidence_ids:
                        claim.evidence_ids.append(e.id)
            self._verified_facts[claim.id] = claim
            log.debug("Claim %s promoted to FACT", claim.id[:8])
        return passed, failures

    @property
    def verified_fact_count(self) -> int:
        return len(self._verified_facts)

    @property
    def fact_store(self) -> dict:
        """Read-only view of the verified facts store, keyed by claim ID."""
        return dict(self._verified_facts)


# ============================================================================
# Safety Gate — Main interface
# ============================================================================

@dataclass
class GateDecision:
    """Result of the SafetyGate evaluation."""
    approved: bool
    action: Action
    rejection_reason: Optional[str] = None
    confirmation_required: bool = False
    audit_trail: List[str] = field(default_factory=list)

    def __repr__(self):
        status = "APPROVED" if self.approved else f"REJECTED({self.rejection_reason})"
        return f"GateDecision({status})"


class SafetyGate:
    """
    The safety gate every action must pass before execution.

    Evaluation pipeline (in order):
      1. Forbidden action type?  → immediate rejection
      2. Forbidden flag set?     → immediate rejection
      3. Irreversibility > 0.8?  → immediate rejection
      4. Blast radius > 0.7?     → immediate rejection
      5. Deception risk > 0.1?   → immediate rejection
      6. Permission escalation?  → immediate rejection
      7. Confirmation required?  → approved with confirmation flag

    Audit:
      Every evaluation is logged to an append-only audit trail.
      Pass the path= argument to persist it to disk (JSONL).
    """

    def __init__(self, audit_path: Optional[str] = None):
        self._audit: List[Dict[str, Any]] = []
        self._audit_path = audit_path
        self._total_evaluated = 0
        self._total_approved = 0
        self._total_rejected = 0

    def evaluate(self, action: Action) -> GateDecision:
        """
        Evaluate a proposed action against all hard constraints.

        Args:
            action: The proposed Action (must have .metadata set for full check)

        Returns:
            GateDecision with .approved, .rejection_reason, .audit_trail
        """
        self._total_evaluated += 1
        trail: List[str] = []

        # ── Check 1: Forbidden action type ──────────────────────────────────
        if action.type in HardConstraints.FORBIDDEN_ACTIONS:
            reason = f"FORBIDDEN_ACTION_TYPE: {action.type.value}"
            trail.append(reason)
            decision = GateDecision(
                approved=False, action=action,
                rejection_reason=reason, audit_trail=trail,
            )
            self._record(action, decision)
            return decision

        meta = action.metadata

        # ── Check 2: Explicit forbidden flag ────────────────────────────────
        if meta and meta.forbidden:
            reason = "FORBIDDEN_FLAG: action.metadata.forbidden is True"
            trail.append(reason)
            decision = GateDecision(
                approved=False, action=action,
                rejection_reason=reason, audit_trail=trail,
            )
            self._record(action, decision)
            return decision

        if meta:
            # ── Check 3: Irreversibility ─────────────────────────────────
            if meta.irreversibility_score > HardConstraints.MAX_IRREVERSIBILITY:
                reason = (
                    f"IRREVERSIBILITY: {meta.irreversibility_score:.2f} > "
                    f"{HardConstraints.MAX_IRREVERSIBILITY}"
                )
                trail.append(reason)
                decision = GateDecision(
                    approved=False, action=action,
                    rejection_reason=reason, audit_trail=trail,
                )
                self._record(action, decision)
                return decision

            # ── Check 4: Blast radius ────────────────────────────────────
            if meta.blast_radius > HardConstraints.MAX_BLAST_RADIUS:
                reason = (
                    f"BLAST_RADIUS: {meta.blast_radius:.2f} > "
                    f"{HardConstraints.MAX_BLAST_RADIUS}"
                )
                trail.append(reason)
                decision = GateDecision(
                    approved=False, action=action,
                    rejection_reason=reason, audit_trail=trail,
                )
                self._record(action, decision)
                return decision

            # ── Check 5: Deception risk ──────────────────────────────────
            if meta.deception_risk > HardConstraints.MAX_DECEPTION_RISK:
                reason = (
                    f"DECEPTION_RISK: {meta.deception_risk:.2f} > "
                    f"{HardConstraints.MAX_DECEPTION_RISK}"
                )
                trail.append(reason)
                decision = GateDecision(
                    approved=False, action=action,
                    rejection_reason=reason, audit_trail=trail,
                )
                self._record(action, decision)
                return decision

            # ── Check 6: Permission escalation ──────────────────────────
            if meta.permission_delta > 0:
                reason = f"PERMISSION_ESCALATION: delta={meta.permission_delta}"
                trail.append(reason)
                decision = GateDecision(
                    approved=False, action=action,
                    rejection_reason=reason, audit_trail=trail,
                )
                self._record(action, decision)
                return decision

            # ── Check 7: Confirmation required ──────────────────────────
            if meta.confirmation_required:
                trail.append("CONFIRMATION_REQUIRED")
                decision = GateDecision(
                    approved=True, action=action,
                    confirmation_required=True, audit_trail=trail,
                )
                self._record(action, decision)
                return decision

        # ── All checks passed ────────────────────────────────────────────────
        trail.append("ALL_CHECKS_PASSED")
        decision = GateDecision(approved=True, action=action, audit_trail=trail)
        self._record(action, decision)
        return decision

    def _record(self, action: Action, decision: GateDecision) -> None:
        """Append to internal + optional on-disk audit trail."""
        entry = {
            "timestamp": time.time(),
            "action_id": action.id,
            "action_type": action.type.value,
            "approved": decision.approved,
            "reason": decision.rejection_reason,
            "trail": decision.audit_trail,
        }
        self._audit.append(entry)

        if decision.approved:
            self._total_approved += 1
        else:
            self._total_rejected += 1

        if self._audit_path:
            import json
            try:
                with open(self._audit_path, "a") as f:
                    f.write(json.dumps(entry) + "\n")
            except OSError as exc:
                log.warning("Could not write to audit file: %s", exc)

    @property
    def stats(self) -> Dict[str, int]:
        return {
            "evaluated": self._total_evaluated,
            "approved": self._total_approved,
            "rejected": self._total_rejected,
        }

    @property
    def audit_log(self) -> List[Dict[str, Any]]:
        return list(self._audit)
