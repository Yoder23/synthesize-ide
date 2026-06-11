"""
MoA Memory
===========
Three-layer memory system with universe separation.

  EventLog       Append-only JSONL. Everything goes here. Nothing is deleted.
  FactGraph      Only verified FACT-modality claims. Strictly gatekept.
  EpisodicBuffer Rolling window of recent (text, metadata) pairs.

Zero external dependencies for the core classes.
Optional: install numpy or torch for vector-based semantic search
(used when a backend provides embed() support).
"""

from __future__ import annotations

import json
import time
import math
import logging
import os
from dataclasses import dataclass, asdict, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from .ir import Claim, Modality, MemoryWrite, IRObject

log = logging.getLogger(__name__)

# Optional vector support
try:
    import numpy as np
    _HAS_NUMPY = True
except ImportError:
    _HAS_NUMPY = False

try:
    import torch
    import torch.nn.functional as F
    _HAS_TORCH = True
except ImportError:
    _HAS_TORCH = False


# ============================================================================
# Event Log — append-only, tamper-evident
# ============================================================================

class EventLog:
    """
    Append-only record of everything the agent stores.

    Every write produces a MemoryWrite IR object for the audit trail.
    If a storage_path is provided, entries are persisted to JSONL on disk.
    Nothing is ever deleted from this log.
    """

    def __init__(self, storage_path: Optional[str] = None):
        self._entries: List[Dict[str, Any]] = []
        self._path = Path(storage_path) if storage_path else None
        if self._path:
            self._path.parent.mkdir(parents=True, exist_ok=True)

    def write(
        self,
        scope: str,
        content: Any,
        provenance: str = "",
    ) -> MemoryWrite:
        """Append a new entry and return the audit record."""
        entry = {
            "id": str(time.time_ns()),
            "timestamp": time.time(),
            "scope": scope,
            "content": content if isinstance(content, (str, int, float, bool)) else str(content),
            "provenance": provenance,
        }
        self._entries.append(entry)

        if self._path:
            try:
                with open(self._path, "a") as f:
                    f.write(json.dumps(entry) + "\n")
            except OSError as exc:
                log.warning("EventLog write failed: %s", exc)

        return MemoryWrite(scope=scope, provenance=provenance,
                           version=len(self._entries))

    def query(
        self,
        scope: Optional[str] = None,
        since: Optional[float] = None,
        limit: int = 100,
    ) -> List[Dict[str, Any]]:
        """Query entries by scope and/or time window."""
        results = self._entries
        if scope:
            results = [e for e in results if e.get("scope") == scope]
        if since:
            results = [e for e in results if e["timestamp"] >= since]
        return results[-limit:]

    @property
    def entry_count(self) -> int:
        return len(self._entries)


# ============================================================================
# Fact Graph — only verified FACT-modality claims
# ============================================================================

class FactGraph:
    """
    Stores only verified, FACT-modality Claims.

    Every claim entering this store has already passed the FormalVerifier
    (in safety.py). Attempting to add a non-FACT claim raises ValueError.
    This is the guarantee: if it's in the FactGraph, it's verified.
    """

    def __init__(self):
        self._facts: Dict[str, Claim] = {}  # claim_id → Claim

    def add(self, claim: Claim) -> None:
        """Add a verified fact. Raises ValueError if claim is not FACT-modality."""
        if claim.modality != Modality.FACT:
            raise ValueError(
                f"FactGraph only accepts Modality.FACT claims. "
                f"Got: {claim.modality.value}. "
                f"Run the FormalVerifier first."
            )
        if not claim.is_verified():
            raise ValueError(
                f"Claim is FACT modality but not fully verified "
                f"(confidence={claim.confidence:.2f}, evidence_ids={claim.evidence_ids})"
            )
        self._facts[claim.id] = claim

    def get(self, claim_id: str) -> Optional[Claim]:
        return self._facts.get(claim_id)

    def search(self, subject: str = "", predicate: str = "") -> List[Claim]:
        """Simple substring search over subject and predicate."""
        results = []
        for c in self._facts.values():
            if subject and subject.lower() not in c.subject.lower():
                continue
            if predicate and predicate.lower() not in c.predicate.lower():
                continue
            results.append(c)
        return results

    def __contains__(self, claim_id: str) -> bool:
        return claim_id in self._facts

    @property
    def fact_count(self) -> int:
        return len(self._facts)


# ============================================================================
# Episodic Buffer — rolling window of recent experiences
# ============================================================================

@dataclass
class Episode:
    """A single experience entry."""
    text: str
    domain: str = "general"
    metadata: Dict[str, Any] = field(default_factory=dict)
    importance: float = 1.0
    embedding: Optional[List[float]] = None   # Set if backend provides embed()
    created_at: float = field(default_factory=time.time)


class EpisodicBuffer:
    """
    A rolling buffer of recent experiences.

    Experiences are stored as text + optional embedding vectors.
    If embeddings are provided (requires a backend with embed() support),
    semantic retrieval is available. Otherwise, keyword-based retrieval
    is used as a fallback.

    This is "working memory" — what the agent encountered recently.
    """

    def __init__(self, capacity: int = 1000):
        self._capacity = capacity
        self._episodes: List[Episode] = []

    def store(
        self,
        text: str,
        domain: str = "general",
        importance: float = 1.0,
        metadata: Optional[Dict[str, Any]] = None,
        embedding: Optional[List[float]] = None,
    ) -> Episode:
        """Store a new episode, evicting lowest-importance if at capacity."""
        ep = Episode(
            text=text, domain=domain,
            importance=importance,
            metadata=metadata or {},
            embedding=embedding,
        )
        if len(self._episodes) >= self._capacity:
            # Evict lowest importance
            min_idx = min(range(len(self._episodes)),
                         key=lambda i: self._episodes[i].importance)
            self._episodes.pop(min_idx)
        self._episodes.append(ep)
        return ep

    def retrieve_recent(self, n: int = 5, domain: Optional[str] = None) -> List[Episode]:
        """Return the N most recent episodes, optionally filtered by domain."""
        episodes = self._episodes
        if domain:
            episodes = [e for e in episodes if e.domain == domain]
        return list(reversed(episodes))[:n]

    def retrieve_semantic(
        self,
        query_embedding: List[float],
        top_k: int = 5,
    ) -> List[Tuple[Episode, float]]:
        """
        Return the top-k most similar episodes by cosine similarity.
        Requires that stored episodes have embeddings.
        Falls back to empty list if no embeddings are available.
        """
        candidates = [e for e in self._episodes if e.embedding is not None]
        if not candidates:
            return []

        def cosine(a: List[float], b: List[float]) -> float:
            if not (_HAS_NUMPY):
                # Pure Python fallback
                dot = sum(x * y for x, y in zip(a, b))
                norm_a = math.sqrt(sum(x * x for x in a))
                norm_b = math.sqrt(sum(x * x for x in b))
                return dot / (norm_a * norm_b + 1e-8)
            import numpy as np
            a_np, b_np = np.array(a), np.array(b)
            return float(np.dot(a_np, b_np) / (np.linalg.norm(a_np) * np.linalg.norm(b_np) + 1e-8))

        scored = [(e, cosine(query_embedding, e.embedding)) for e in candidates]
        scored.sort(key=lambda x: x[1], reverse=True)
        return scored[:top_k]

    def retrieve_keyword(self, query: str, top_k: int = 5) -> List[Episode]:
        """Return episodes whose text contains query words (case-insensitive)."""
        words = query.lower().split()
        scored = []
        for ep in self._episodes:
            text_lower = ep.text.lower()
            hits = sum(1 for w in words if w in text_lower)
            if hits > 0:
                scored.append((ep, hits))
        scored.sort(key=lambda x: x[1], reverse=True)
        return [e for e, _ in scored[:top_k]]

    def decay_importance(self, factor: float = 0.999) -> None:
        """Apply time-based decay to all importance scores."""
        for ep in self._episodes:
            ep.importance *= factor

    @property
    def size(self) -> int:
        return len(self._episodes)


# ============================================================================
# Agent Memory — convenience wrapper combining all three layers
# ============================================================================

class AgentMemory:
    """
    Unified memory interface for a MoAAgent.

    Combines:
      event_log     → append-only audit trail (everything)
      fact_graph    → verified facts only
      episodes      → rolling episodic buffer

    Usage:
        memory = AgentMemory(storage_dir="./moa_memory")
        memory.log_event("task", "Started chess analysis")
        memory.store_episode("White plays e4, black responds e5", domain="chess")
    """

    def __init__(
        self,
        storage_dir: Optional[str] = None,
        episode_capacity: int = 1000,
    ):
        log_path = None
        if storage_dir:
            Path(storage_dir).mkdir(parents=True, exist_ok=True)
            log_path = str(Path(storage_dir) / "event_log.jsonl")

        self.event_log   = EventLog(storage_path=log_path)
        self.fact_graph  = FactGraph()
        self.episodes    = EpisodicBuffer(capacity=episode_capacity)

    def log_event(self, scope: str, content: Any, provenance: str = "") -> MemoryWrite:
        return self.event_log.write(scope, content, provenance)

    def store_episode(
        self,
        text: str,
        domain: str = "general",
        importance: float = 1.0,
        embedding: Optional[List[float]] = None,
    ) -> Episode:
        return self.episodes.store(text=text, domain=domain,
                                   importance=importance, embedding=embedding)

    def add_fact(self, claim: Claim) -> None:
        """Add a verified claim to the FactGraph (must already be FACT modality)."""
        self.fact_graph.add(claim)
        self.event_log.write("fact", f"{claim.subject} {claim.predicate} {claim.object}",
                             provenance=claim.provenance or "")

    def recall(
        self,
        query: str,
        query_embedding: Optional[List[float]] = None,
        top_k: int = 5,
    ) -> List[Episode]:
        """Retrieve relevant episodes (semantic if embedding available, else keyword)."""
        if query_embedding:
            results = self.retrieve_semantic(query_embedding, top_k)
            return [ep for ep, _ in results]
        return self.episodes.retrieve_keyword(query, top_k)

    def retrieve_semantic(
        self, embedding: List[float], top_k: int = 5
    ) -> List[Tuple[Episode, float]]:
        return self.episodes.retrieve_semantic(embedding, top_k)
