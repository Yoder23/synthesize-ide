"""
MoA Intermediate Representation (IR)
======================================
Structured, typed objects for memory, reasoning, verification, and action control.

Design Principle: Memory ≠ Belief
  - Everything may be stored
  - Only verified information may be treated as fact
  - SIMULATION → FACT transitions are architecturally blocked

Zero external dependencies — stdlib only.
"""

from dataclasses import dataclass, field
from typing import Literal, Dict, List, Optional, Any, Set
from enum import Enum
from datetime import datetime
import uuid


# ============================================================================
# Core Enums
# ============================================================================

class Modality(Enum):
    """
    Universe separation: every piece of information has a type.

    FACT        Verified with evidence, confidence >= 0.8.
                Can justify real-world actions.
    HYPOTHESIS  Unverified claim. Needs more evidence before acting on it.
    SIMULATION  Counterfactual reasoning only. NEVER promoted to FACT.
    FICTION     Exploratory, creative, non-binding. Cannot escape this status.
    """
    FACT = "fact"
    HYPOTHESIS = "hypothesis"
    SIMULATION = "simulation"
    FICTION = "fiction"


class IntentType(Enum):
    """Types of user intents."""
    QUERY   = "query"    # Ask a question
    CREATE  = "create"   # Write code, file, artefact
    MODIFY  = "modify"   # Edit something existing
    EXECUTE = "execute"  # Run a command
    LEARN   = "learn"    # Store knowledge
    VERIFY  = "verify"   # Check facts


class ActionType(Enum):
    """All possible actions an agent may take, ordered by risk."""
    # Reversible — no confirmation needed
    READ_FILE        = "read_file"
    WRITE_CODE       = "write_code"
    RUN_CODE         = "run_code"
    QUERY_API        = "query_api"

    # Requires confirmation
    INSTALL_PACKAGE  = "install_package"
    DELETE_FILE      = "delete_file"
    MODIFY_SYSTEM    = "modify_system"
    ACCESS_NETWORK   = "access_network"

    # Architecturally forbidden — always rejected
    ESCALATE_PERMISSION = "escalate_permission"
    DECEIVE_USER        = "deceive_user"
    MANIPULATE_SOCIAL   = "manipulate_social"


class EvidenceSource(Enum):
    """Where evidence comes from."""
    EXECUTION_TRACE    = "execution_trace"    # Code ran, we observed output
    API_RESPONSE       = "api_response"       # External service returned data
    HUMAN_LABEL        = "human_label"        # User confirmed
    VERIFICATION_TOOL  = "verification_tool"  # A checker validated
    CROSS_REFERENCE    = "cross_reference"    # Multiple independent sources


# ============================================================================
# IR Dataclasses
# ============================================================================

@dataclass
class IRObject:
    """Base for all IR objects. Every object gets a UUID and timestamp."""
    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    timestamp: datetime = field(default_factory=datetime.now)


@dataclass
class Intent(IRObject):
    """A decoded user intent."""
    type: IntentType = IntentType.QUERY
    description: str = ""
    priority: int = 1          # 1 (low) to 10 (critical)
    constraints: List[str] = field(default_factory=list)
    deadline: Optional[datetime] = None
    provenance: Optional[str] = None

    def __repr__(self):
        return f"Intent({self.type.value!r}, p={self.priority}, {self.description[:40]!r})"


@dataclass
class Entity(IRObject):
    """A normalised entity — file, API endpoint, concept, package, etc."""
    type: str = ""             # "file" | "api" | "concept" | "package"
    canonical_name: str = ""
    aliases: List[str] = field(default_factory=list)
    properties: Dict[str, Any] = field(default_factory=dict)
    provenance: Optional[str] = None

    def __repr__(self):
        return f"Entity({self.type}: {self.canonical_name!r})"


@dataclass
class Claim(IRObject):
    """
    A statement about the world.

    Three conditions must ALL be true before a Claim is verified as FACT:
      1. modality == FACT
      2. len(evidence_ids) >= 1
      3. confidence >= 0.8

    Critically: a Claim originating from SIMULATION is blocked from
    ever reaching FACT by the FormalVerifier (see safety.py).
    """
    subject: str = ""
    predicate: str = ""
    object: str = ""
    modality: Modality = Modality.HYPOTHESIS
    confidence: float = 0.5
    evidence_ids: List[str] = field(default_factory=list)
    contradicts: List[str] = field(default_factory=list)
    provenance: Optional[str] = None

    def is_verified(self) -> bool:
        """Can this claim be used to justify real-world actions?"""
        return (
            self.modality == Modality.FACT
            and len(self.evidence_ids) > 0
            and self.confidence >= 0.8
        )

    def __repr__(self):
        return (f"Claim({self.subject!r} {self.predicate!r} {self.object!r} "
                f"[{self.modality.value}] conf={self.confidence:.2f})")


@dataclass
class Evidence(IRObject):
    """Evidence supporting or refuting a Claim."""
    claim_id: str = ""
    source: EvidenceSource = EvidenceSource.EXECUTION_TRACE
    locator: str = ""          # "file.py:42", URL, trace_id
    content: str = ""
    confidence: float = 1.0
    is_supporting: bool = True  # False if this contradicts the claim
    provenance: Optional[str] = None

    def __repr__(self):
        sign = "+" if self.is_supporting else "–"
        return f"Evidence({sign}{self.source.value}, conf={self.confidence:.2f})"


@dataclass
class ActionMeta(IRObject):
    """
    Safety metadata for a proposed action.

    All scores are in [0.0, 1.0]:
      0.0 = minimum risk / maximum safety
      1.0 = maximum risk / minimum safety

    is_safe() → True means the SafetyGate will approve without confirmation.
    """
    action_id: str = ""
    irreversibility_score: float = 0.5   # Can we undo this?
    blast_radius: float = 0.5            # How much of the system can this affect?
    permission_delta: int = 0            # Change in privilege level (+1, 0, -1)
    deception_risk: float = 0.0          # Could this mislead the user?
    data_access_change: int = 0          # New data sources opened (+1, 0, -1)
    confirmation_required: bool = False
    rationale_required: bool = False
    forbidden: bool = False
    provenance: Optional[str] = None

    def is_safe(self) -> bool:
        """True if the action can proceed without explicit confirmation."""
        return (
            not self.forbidden
            and self.irreversibility_score < 0.3
            and self.blast_radius < 0.3
            and self.permission_delta <= 0
            and self.deception_risk < 0.1
            and self.data_access_change <= 0
        )

    def __repr__(self):
        if self.forbidden:
            label = "FORBIDDEN"
        elif self.is_safe():
            label = "SAFE"
        else:
            label = "CONFIRM"
        return (f"ActionMeta({label}, irrev={self.irreversibility_score:.2f}, "
                f"blast={self.blast_radius:.2f})")


@dataclass
class Action(IRObject):
    """A proposed or completed action."""
    type: ActionType = ActionType.READ_FILE
    args: Dict[str, Any] = field(default_factory=dict)
    metadata: Optional[ActionMeta] = None
    status: Literal["proposed", "approved", "executing",
                    "completed", "failed", "rejected"] = "proposed"
    result: Optional[Any] = None
    error: Optional[str] = None
    preserves_optionality: bool = True
    future_actions_enabled: Set[str] = field(default_factory=set)
    future_actions_blocked: Set[str] = field(default_factory=set)
    provenance: Optional[str] = None

    def __repr__(self):
        return f"Action({self.type.value}, status={self.status!r})"


@dataclass
class MemoryWrite(IRObject):
    """Record of writing to memory (append-only audit trail)."""
    scope: str = ""
    payload: Optional[IRObject] = None
    provenance: str = ""
    version: int = 1

    def __repr__(self):
        payload_type = type(self.payload).__name__ if self.payload else "None"
        return f"MemoryWrite(scope={self.scope!r}, v{self.version}, {payload_type})"


@dataclass
class ValueDiff(IRObject):
    """A proposed change to the agent's value function."""
    rationale: str = ""
    predicted_behavior_change: str = ""
    old_weights: Dict[str, float] = field(default_factory=dict)
    new_weights: Dict[str, float] = field(default_factory=dict)
    red_lines: List[str] = field(default_factory=list)
    regression_tests: List[str] = field(default_factory=list)
    status: Literal["proposed", "testing", "approved",
                    "rejected", "reverted"] = "proposed"
    test_results: Dict[str, bool] = field(default_factory=dict)
    provenance: Optional[str] = None

    def __repr__(self):
        return f"ValueDiff({self.status}, {len(self.new_weights)} weights)"


@dataclass
class Counterfactual(IRObject):
    """
    An immutable counterfactual for learning.

    Counterfactuals are stored in SIMULATION modality and CANNOT
    be promoted to FACT. They are used to learn from hypotheticals
    without leaking speculative reasoning into the factual knowledge base.
    """
    scenario: str = ""
    actual_outcome: str = ""
    hypothetical_outcome: str = ""
    human_valence: Optional[float] = None   # -1.0 (bad) to +1.0 (good)
    actions_taken: List[Action] = field(default_factory=list)
    actions_avoided: List[Action] = field(default_factory=list)
    importance: float = 0.5
    surprise: float = 0.0
    provenance: Optional[str] = None

    def __repr__(self):
        v = f"valence={self.human_valence:+.2f}" if self.human_valence else "unlabeled"
        return f"Counterfactual({v}, importance={self.importance:.2f})"


@dataclass
class ReasoningTrace(IRObject):
    """Audit record of a reasoning chain."""
    input_claims: List[Claim] = field(default_factory=list)
    reasoning_steps: List[str] = field(default_factory=list)
    output_claims: List[Claim] = field(default_factory=list)
    confidence: float = 0.5
    universe: Modality = Modality.HYPOTHESIS
    verified: bool = False
    verification_failures: List[str] = field(default_factory=list)
    provenance: Optional[str] = None

    def __repr__(self):
        marker = "[v]" if self.verified else "[ ]"
        return (f"ReasoningTrace({marker}, {len(self.reasoning_steps)} steps, "
                f"conf={self.confidence:.2f})")


# ============================================================================
# Utility Functions
# ============================================================================

def create_claim(
    subject: str,
    predicate: str,
    object: str,
    modality: Modality = Modality.HYPOTHESIS,
    confidence: float = 0.5,
) -> Claim:
    """Convenience constructor for a Claim."""
    return Claim(
        subject=subject, predicate=predicate, object=object,
        modality=modality, confidence=confidence,
    )


def create_evidence(
    claim: Claim,
    source: EvidenceSource,
    locator: str,
    content: str,
    confidence: float = 1.0,
) -> Evidence:
    """Convenience constructor for Evidence linked to a Claim."""
    return Evidence(
        claim_id=claim.id, source=source,
        locator=locator, content=content,
        confidence=confidence,
    )
