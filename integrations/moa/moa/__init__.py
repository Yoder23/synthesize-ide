"""
MoA — Master of Apps

A safety-first, model-agnostic agent framework with hard-constraint action gating,
universe-separated memory, and formal verification.

Works with any LLM: OpenAI, Anthropic, Ollama, HuggingFace, LayerCake.

Quick start:
    from moa import MoAAgent
    from moa.backends import MockBackend

    agent = MoAAgent(backend=MockBackend())
    result = agent.run("Analyse chess opening: e4 e5 Nf3 Nc6")
    print(result.response)
"""

from .agent import MoAAgent, AgentResult
from .ir import (
    Modality, IntentType, ActionType, EvidenceSource,
    Claim, Evidence, Action, ActionMeta, Intent,
    create_claim, create_evidence,
)
from .safety import SafetyGate, FormalVerifier, HardConstraints
from .memory import AgentMemory, EventLog, FactGraph, EpisodicBuffer

__version__ = "0.1.0"
__all__ = [
    # Agent
    "MoAAgent",
    "AgentResult",
    # IR
    "Modality",
    "IntentType",
    "ActionType",
    "EvidenceSource",
    "Claim",
    "Evidence",
    "Action",
    "ActionMeta",
    "Intent",
    "create_claim",
    "create_evidence",
    # Safety
    "SafetyGate",
    "FormalVerifier",
    "HardConstraints",
    # Memory
    "AgentMemory",
    "EventLog",
    "FactGraph",
    "EpisodicBuffer",
]
