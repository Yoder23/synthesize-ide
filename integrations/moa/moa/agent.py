"""
MoA Agent — OODA Loop
======================
The Master of Apps agent: Observe → Orient → Decide → Act → Learn.

Works with any backend that implements BaseLLMBackend.
Safety constraints are enforced regardless of which LLM is used.
"""

from __future__ import annotations

import json
import logging
import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

from .ir import (
    Action, ActionMeta, ActionType, Claim, Evidence, EvidenceSource,
    Intent, IntentType, Modality, ReasoningTrace,
    create_claim, create_evidence,
)
from .safety import FormalVerifier, SafetyGate, GateDecision
from .memory import AgentMemory, Episode
from .backends.base import BaseLLMBackend, Message

log = logging.getLogger(__name__)


# ============================================================================
# Result types
# ============================================================================

@dataclass
class AgentResult:
    """Everything produced by one agent turn."""
    response: str
    intent: Optional[Intent] = None
    claims_made: List[Claim] = field(default_factory=list)
    actions_taken: List[Action] = field(default_factory=list)
    actions_rejected: List[GateDecision] = field(default_factory=list)
    reasoning: Optional[ReasoningTrace] = None
    duration_s: float = 0.0
    metadata: Dict[str, Any] = field(default_factory=dict)

    def __repr__(self):
        n_approved = len(self.actions_taken)
        n_rejected = len(self.actions_rejected)
        return (
            f"AgentResult({len(self.response)} chars, "
            f"{n_approved} approved, {n_rejected} rejected, "
            f"{self.duration_s:.2f}s)"
        )


# ============================================================================
# MoA Agent
# ============================================================================

class MoAAgent:
    """
    Master of Apps agent with hard-constraint safety gating.

    Core guarantees (independent of which LLM backend is used):
      1. FORBIDDEN actions (DECEIVE_USER, ESCALATE_PERMISSION, MANIPULATE_SOCIAL)
         are rejected before the LLM can act on them.
      2. Actions with irreversibility > 0.8 or blast_radius > 0.7 are rejected.
      3. SIMULATION claims can never become FACT (FormalVerifier blocks it).
      4. Every action is logged to an append-only audit trail.
      5. All memory writes are type-separated: FACT vs HYPOTHESIS vs SIMULATION.

    Usage:
        agent = MoAAgent(backend=MockBackend())
        result = agent.run("Analyse the chess position: e4 e5 Nf3 Nc6")
        print(result.response)

    Swap in any backend:
        agent = MoAAgent(backend=OpenAIBackend("gpt-4o"))
        agent = MoAAgent(backend=OllamaBackend("llama3"))
        agent = MoAAgent(backend=LayerCakeBackend.from_checkpoint("core.pt", ...))
    """

    SYSTEM_PROMPT = (
        "You are MoA, a trustworthy AI agent. "
        "You reason carefully, distinguish facts from hypotheses, "
        "and never deceive the user. "
        "When proposing actions, always state your confidence level "
        "and whether the action is reversible."
    )

    def __init__(
        self,
        backend: BaseLLMBackend,
        system_prompt: Optional[str] = None,
        storage_dir: Optional[str] = None,
        episode_capacity: int = 500,
        audit_path: Optional[str] = None,
        max_history: int = 20,
    ):
        """
        Args:
            backend:          Any BaseLLMBackend implementation.
            system_prompt:    Override the default system prompt.
            storage_dir:      Optional directory for persisting memory to disk.
            episode_capacity: Maximum number of episodes in the episodic buffer.
            audit_path:       Optional JSONL file path for the safety audit log.
            max_history:      Maximum conversation turns to include in each call.
        """
        self.backend = backend
        self._system_prompt = system_prompt or self.SYSTEM_PROMPT
        self._max_history = max_history

        self.memory    = AgentMemory(storage_dir=storage_dir,
                                     episode_capacity=episode_capacity)
        self.safety    = SafetyGate(audit_path=audit_path)
        self.verifier  = FormalVerifier()

        self._history: List[Message] = []
        self._step = 0

    # ── Public API ────────────────────────────────────────────────────────────

    def run(self, user_message: str, domain: str = "general") -> AgentResult:
        """
        Run one agent turn through the full OODA loop.

        Args:
            user_message: The user's input.
            domain:       Optional domain hint (e.g. "chess", "code", "general").

        Returns:
            AgentResult with response, safety decisions, and memory updates.
        """
        t0 = time.perf_counter()
        self._step += 1

        # ── Observe ──────────────────────────────────────────────────────────
        embedding = self.backend.embed(user_message)
        similar = self.memory.recall(user_message, query_embedding=embedding, top_k=3)
        context_snippets = [ep.text for ep in similar]

        # ── Orient ───────────────────────────────────────────────────────────
        intent = self._decode_intent(user_message)
        self.memory.log_event("intent", str(intent), provenance="user")

        # ── Decide ───────────────────────────────────────────────────────────
        messages = self._build_messages(user_message, context_snippets)
        response = self.backend.generate(messages, max_tokens=512, temperature=0.7)
        self._history.append({"role": "user", "content": user_message})
        self._history.append({"role": "assistant", "content": response})
        if len(self._history) > self._max_history * 2:
            self._history = self._history[-(self._max_history * 2):]

        # ── Act — evaluate any proposed actions ──────────────────────────────
        proposed_actions = self._extract_actions(response, intent)
        approved_actions: List[Action] = []
        rejected_decisions: List[GateDecision] = []

        for action in proposed_actions:
            decision = self.safety.evaluate(action)
            if decision.approved:
                action.status = "approved"
                approved_actions.append(action)
                self.memory.log_event(
                    "action_approved",
                    f"{action.type.value}: {action.args}",
                    provenance=f"step={self._step}",
                )
            else:
                action.status = "rejected"
                rejected_decisions.append(decision)
                self.memory.log_event(
                    "action_rejected",
                    f"{action.type.value}: {decision.rejection_reason}",
                    provenance=f"step={self._step}",
                )
                log.info("[SafetyGate] Rejected: %s", decision.rejection_reason)

        # ── Learn — store episode ─────────────────────────────────────────────
        self.memory.store_episode(
            text=f"Q: {user_message}\nA: {response}",
            domain=domain,
            importance=1.0,
            embedding=embedding,
        )

        # Build result
        result = AgentResult(
            response=response,
            intent=intent,
            actions_taken=approved_actions,
            actions_rejected=rejected_decisions,
            duration_s=time.perf_counter() - t0,
        )

        log.debug(
            "Step %d | backend=%s | %d approved | %d rejected | %.2fs",
            self._step, self.backend.name,
            len(approved_actions), len(rejected_decisions),
            result.duration_s,
        )
        return result

    def store_fact(
        self,
        subject: str,
        predicate: str,
        object_: str,
        evidence_content: str,
        confidence: float = 0.9,
    ) -> Optional[Claim]:
        """
        Attempt to store a verified fact.

        The claim must pass the FormalVerifier's 5-check process.
        Returns the Claim if verified and stored, None if rejected.
        """
        claim = create_claim(subject, predicate, object_,
                             modality=Modality.HYPOTHESIS, confidence=confidence)
        evidence = create_evidence(
            claim=claim,
            source=EvidenceSource.EXECUTION_TRACE,
            locator=f"step={self._step}",
            content=evidence_content,
            confidence=confidence,
        )
        promoted, failures = self.verifier.promote_to_fact(claim, [evidence])
        if promoted:
            self.memory.add_fact(claim)
            return claim
        log.info("Fact rejected: %s", failures)
        return None

    def reset_history(self) -> None:
        """Clear conversation history (does not affect memory)."""
        self._history.clear()

    @property
    def safety_stats(self) -> Dict[str, int]:
        return self.safety.stats

    @property
    def step(self) -> int:
        return self._step

    # ── Internal helpers ──────────────────────────────────────────────────────

    def _build_messages(
        self,
        user_message: str,
        context_snippets: List[str],
    ) -> List[Message]:
        """Assemble the message list to send to the backend."""
        messages: List[Message] = [
            {"role": "system", "content": self._system_prompt}
        ]

        # Inject relevant memory as context
        if context_snippets:
            ctx = "\n".join(f"- {s[:200]}" for s in context_snippets)
            messages.append({
                "role": "system",
                "content": f"Relevant context from memory:\n{ctx}",
            })

        # Recent conversation history
        messages.extend(self._history[-(self._max_history * 2):])

        # Current user message (already in history after this call, not yet)
        messages.append({"role": "user", "content": user_message})
        return messages

    def _decode_intent(self, text: str) -> Intent:
        """Simple keyword-based intent classification."""
        text_lower = text.lower()
        if any(w in text_lower for w in ("delete", "remove", "drop", "rm ")):
            return Intent(type=IntentType.MODIFY, description=text[:80], priority=3)
        if any(w in text_lower for w in ("run", "execute", "install", "deploy")):
            return Intent(type=IntentType.EXECUTE, description=text[:80], priority=5)
        if any(w in text_lower for w in ("write", "create", "generate", "build")):
            return Intent(type=IntentType.CREATE, description=text[:80], priority=4)
        if any(w in text_lower for w in ("edit", "fix", "update", "change", "modify")):
            return Intent(type=IntentType.MODIFY, description=text[:80], priority=4)
        if any(w in text_lower for w in ("learn", "remember", "store", "save")):
            return Intent(type=IntentType.LEARN, description=text[:80], priority=2)
        if any(w in text_lower for w in ("verify", "check", "validate", "confirm")):
            return Intent(type=IntentType.VERIFY, description=text[:80], priority=3)
        return Intent(type=IntentType.QUERY, description=text[:80], priority=1)

    def _extract_actions(
        self,
        response: str,
        intent: Intent,
    ) -> List[Action]:
        """
        Extract proposed actions from the agent response.

        Currently uses a simple heuristic based on the decoded intent.
        In production, this would parse structured action proposals from
        the LLM output (e.g., a JSON block with action type and args).
        """
        actions = []

        # Map intent to a candidate action type
        action_map = {
            IntentType.EXECUTE: ActionType.RUN_CODE,
            IntentType.CREATE:  ActionType.WRITE_CODE,
            IntentType.MODIFY:  ActionType.MODIFY_SYSTEM,
            IntentType.LEARN:   ActionType.READ_FILE,
            IntentType.QUERY:   ActionType.READ_FILE,
            IntentType.VERIFY:  ActionType.READ_FILE,
        }

        action_type = action_map.get(intent.type, ActionType.READ_FILE)

        # Assign safety scores based on action type
        meta_map = {
            ActionType.READ_FILE:       ActionMeta(irreversibility_score=0.0, blast_radius=0.1, deception_risk=0.0),
            ActionType.WRITE_CODE:      ActionMeta(irreversibility_score=0.2, blast_radius=0.2, deception_risk=0.0),
            ActionType.RUN_CODE:        ActionMeta(irreversibility_score=0.3, blast_radius=0.3, deception_risk=0.0, confirmation_required=True),
            ActionType.MODIFY_SYSTEM:   ActionMeta(irreversibility_score=0.6, blast_radius=0.5, deception_risk=0.0, confirmation_required=True),
            ActionType.INSTALL_PACKAGE: ActionMeta(irreversibility_score=0.4, blast_radius=0.3, deception_risk=0.0, confirmation_required=True),
        }

        meta = meta_map.get(action_type, ActionMeta())
        action = Action(type=action_type, args={"description": intent.description}, metadata=meta)
        actions.append(action)
        return actions
