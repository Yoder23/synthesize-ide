"""
bench_memory.py — Memory Layer Benchmarks
==========================================
Measures:
  1. EventLog write throughput (entries/s) and read throughput
  2. EpisodicBuffer store + keyword retrieval speed vs. N episodes
  3. FactGraph add + search speed vs. N facts
  4. AgentMemory unified recall latency
  5. Memory footprint (bytes) vs. episode count

No external dependencies — stdlib only.
"""

from __future__ import annotations

import sys
import time
import tracemalloc
from dataclasses import dataclass
from pathlib import Path
from typing import List, Tuple

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from moa.memory import EventLog, EpisodicBuffer, FactGraph, AgentMemory
from moa.ir import Modality, create_claim, create_evidence, EvidenceSource
from moa.safety import FormalVerifier


@dataclass
class MemoryBenchResult:
    # EventLog
    event_log_write_tps:  float = 0.0   # entries/s
    event_log_read_tps:   float = 0.0   # queries/s
    event_log_n:          int   = 0

    # EpisodicBuffer
    episode_write_tps:    float = 0.0
    episode_kw_recall_tps: float = 0.0   # keyword recall
    episode_n:            int   = 0

    # FactGraph
    fact_add_tps:         float = 0.0
    fact_search_tps:      float = 0.0
    fact_n:               int   = 0

    # AgentMemory unified
    recall_latency_ms:    float = 0.0
    recall_with_n:        int   = 0

    # Footprint
    footprint_100ep_kb:   float = 0.0
    footprint_1000ep_kb:  float = 0.0


# ─── 1. EventLog ──────────────────────────────────────────────────────────────

def bench_event_log(n: int = 10_000) -> Tuple[float, float]:
    log = EventLog()

    # Write
    t0 = time.perf_counter()
    for i in range(n):
        log.write("task", f"event_{i}", provenance="bench")
    write_tps = n / (time.perf_counter() - t0)

    # Read (query all task scope)
    t0 = time.perf_counter()
    for _ in range(1000):
        _ = log.query(scope="task")
    read_tps = 1000 / (time.perf_counter() - t0)

    return write_tps, read_tps


# ─── 2. EpisodicBuffer ────────────────────────────────────────────────────────

def bench_episodic(n: int = 1_000) -> Tuple[float, float]:
    buf = EpisodicBuffer(capacity=n + 100)

    # Write
    t0 = time.perf_counter()
    for i in range(n):
        buf.store(f"The agent analysed chess position {i}. Result: advantage to White.",
                  domain="chess")
    write_tps = n / (time.perf_counter() - t0)

    # Keyword recall (100 queries)
    t0 = time.perf_counter()
    for _ in range(100):
        buf.retrieve_keyword("chess advantage White", top_k=5)
    recall_tps = 100 / (time.perf_counter() - t0)

    return write_tps, recall_tps


# ─── 3. FactGraph ─────────────────────────────────────────────────────────────

def bench_fact_graph(n: int = 1_000) -> Tuple[float, float]:
    fg = FactGraph()
    verifier = FormalVerifier()

    # Pre-create and promote claims to FACT
    facts = []
    for i in range(n):
        c = create_claim(f"subject_{i}", "predicate", f"object_{i}",
                         Modality.HYPOTHESIS, 0.9)
        ev = create_evidence(c, EvidenceSource.EXECUTION_TRACE,
                             f"f.py:{i}", f"content {i}", 1.0)
        verifier.promote_to_fact(c, [ev])
        facts.append(c)

    # Add throughput
    t0 = time.perf_counter()
    for c in facts:
        fg.add(c)
    add_tps = n / (time.perf_counter() - t0)

    # Search throughput (100 queries)
    t0 = time.perf_counter()
    for i in range(100):
        fg.search(subject=f"subject_{i % n}")
    search_tps = 100 / (time.perf_counter() - t0)

    return add_tps, search_tps


# ─── 4. AgentMemory Unified Recall ───────────────────────────────────────────

def bench_agent_memory_recall(n_episodes: int = 500) -> float:
    """Returns recall latency in ms with n_episodes stored."""
    mem = AgentMemory(episode_capacity=n_episodes + 100)

    # Populate
    for i in range(n_episodes):
        mem.store_episode(
            f"Step {i}: analysed chess move e4 e5 Nf3 Nc6 with result favorable",
            domain="chess",
        )

    # Benchmark recall (no embedding = keyword fallback)
    t0 = time.perf_counter()
    for _ in range(200):
        mem.recall("chess move analysis", top_k=5)
    elapsed_ms = (time.perf_counter() - t0) / 200 * 1000

    return elapsed_ms


# ─── 5. Memory Footprint ─────────────────────────────────────────────────────

def bench_footprint() -> Tuple[float, float]:
    """Returns (kb_100_episodes, kb_1000_episodes)."""
    def measure(n: int) -> float:
        tracemalloc.start()
        mem = AgentMemory(episode_capacity=n + 50)
        for i in range(n):
            mem.store_episode(
                f"Episode {i}: The agent performed task {i} and produced a result.",
                domain="general",
            )
            mem.log_event("task", f"event_{i}")
        current, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()
        return peak / 1024  # KB

    kb_100  = measure(100)
    kb_1000 = measure(1000)
    return kb_100, kb_1000


# ─── Runner ──────────────────────────────────────────────────────────────────

def run() -> MemoryBenchResult:
    r = MemoryBenchResult()

    r.event_log_write_tps, r.event_log_read_tps = bench_event_log(10_000)
    r.event_log_n = 10_000

    r.episode_write_tps, r.episode_kw_recall_tps = bench_episodic(1_000)
    r.episode_n = 1_000

    r.fact_add_tps, r.fact_search_tps = bench_fact_graph(1_000)
    r.fact_n = 1_000

    r.recall_latency_ms = bench_agent_memory_recall(500)
    r.recall_with_n = 500

    r.footprint_100ep_kb, r.footprint_1000ep_kb = bench_footprint()

    return r


if __name__ == "__main__":
    print("Memory Layer Benchmarks")
    print("=" * 50)
    r = run()
    print(f"  EventLog write:     {r.event_log_write_tps:>10,.0f} entries/s  (n={r.event_log_n:,})")
    print(f"  EventLog read:      {r.event_log_read_tps:>10,.0f} queries/s")
    print()
    print(f"  EpisodicBuffer write: {r.episode_write_tps:>8,.0f} episodes/s  (n={r.episode_n:,})")
    print(f"  EpisodicBuffer recall:{r.episode_kw_recall_tps:>8,.0f} queries/s")
    print()
    print(f"  FactGraph add:      {r.fact_add_tps:>10,.0f} facts/s     (n={r.fact_n:,})")
    print(f"  FactGraph search:   {r.fact_search_tps:>10,.0f} queries/s")
    print()
    print(f"  AgentMemory recall: {r.recall_latency_ms:>7.3f} ms/query  "
          f"({r.recall_with_n} episodes stored)")
    print()
    print(f"  Memory footprint (100 episodes):   {r.footprint_100ep_kb:>6.1f} KB")
    print(f"  Memory footprint (1000 episodes):  {r.footprint_1000ep_kb:>6.1f} KB")
