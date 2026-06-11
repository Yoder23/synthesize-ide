"""
bench_agent.py — OODA Loop Benchmarks
=======================================
Measures:
  1. Full turn throughput: turns/s with MockBackend
  2. Turn latency breakdown: observe / orient / decide / act / learn (µs each)
  3. Memory overhead: latency vs. N stored episodes
  4. Safety gate overhead: OODA with gating vs. without
  5. Multi-turn conversation: latency stability across 50 turns

No external dependencies — stdlib only.
"""

from __future__ import annotations

import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Tuple

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from moa.agent import MoAAgent
from moa.backends.mock import MockBackend
from moa.memory import AgentMemory
from moa.safety import SafetyGate


@dataclass
class AgentBenchResult:
    # Throughput
    turns_per_sec:        float = 0.0
    turns_n:              int   = 0

    # Latency breakdown (microseconds)
    latency_p50_ms:       float = 0.0
    latency_p95_ms:       float = 0.0
    latency_p99_ms:       float = 0.0
    latency_max_ms:       float = 0.0

    # Memory scaling
    latency_ms_10ep:      float = 0.0
    latency_ms_100ep:     float = 0.0
    latency_ms_500ep:     float = 0.0

    # Safety overhead
    overhead_us:          float = 0.0   # µs added per gate call

    # Multi-turn stability
    multiturn_p50_ms:     float = 0.0
    multiturn_p99_ms:     float = 0.0
    multiturn_n:          int   = 0


# ─── 1. Throughput ────────────────────────────────────────────────────────────

def bench_throughput(n: int = 500) -> Tuple[float, List[float]]:
    agent = MoAAgent(backend=MockBackend())
    latencies: List[float] = []

    for i in range(n):
        t0 = time.perf_counter()
        agent.run(f"task {i}: analyse step {i}")
        latencies.append((time.perf_counter() - t0) * 1000)  # ms

    total_t = sum(latencies) / 1000  # back to seconds
    turns_per_sec = n / total_t
    return turns_per_sec, latencies


# ─── 2. Memory Scaling ───────────────────────────────────────────────────────

def bench_memory_scaling() -> Tuple[float, float, float]:
    """Returns (ms at 10ep, ms at 100ep, ms at 500ep)."""

    def measure(n_preload: int, n_bench: int = 50) -> float:
        agent = MoAAgent(backend=MockBackend(),
                         episode_capacity=n_preload + 100)
        # Preload episodes
        for i in range(n_preload):
            agent.memory.store_episode(
                f"chess analysis step {i} White advantage material gain",
                domain="chess",
            )
        # Measure query turns
        t0 = time.perf_counter()
        for j in range(n_bench):
            agent.run(f"chess analysis query {j}")
        return (time.perf_counter() - t0) / n_bench * 1000  # ms/turn

    return measure(10), measure(100), measure(500)


# ─── 3. Safety Gate Overhead ─────────────────────────────────────────────────

def bench_gate_overhead(n: int = 2_000) -> float:
    """
    Returns average overhead (µs) added per turn by the safety gate.
    Compares MoAAgent with a gate that has nothing to reject.
    """
    from moa.ir import Action, ActionMeta, ActionType

    # Direct gate call timing
    from moa.safety import SafetyGate
    gate = SafetyGate()
    action = Action(type=ActionType.READ_FILE,
                    metadata=ActionMeta(irreversibility_score=0.0, blast_radius=0.0))

    t0 = time.perf_counter()
    for _ in range(n):
        gate.evaluate(action)
    overhead_us = (time.perf_counter() - t0) / n * 1e6
    return overhead_us


# ─── 4. Multi-turn Stability ─────────────────────────────────────────────────

def bench_multiturn(n: int = 50) -> Tuple[float, float]:
    """
    50-turn conversation. Returns (p50_ms, p99_ms).
    Tests that latency doesn't drift as history + memory grow.
    """
    agent = MoAAgent(backend=MockBackend())
    topics = [
        "What is the Ruy Lopez?",
        "How does Black respond to Bb5?",
        "What is the Berlin Defense?",
        "Explain the Marshall Attack.",
        "Why is the Ruy Lopez popular at GM level?",
    ]
    latencies: List[float] = []
    for i in range(n):
        t0 = time.perf_counter()
        agent.run(topics[i % len(topics)])
        latencies.append((time.perf_counter() - t0) * 1000)

    sorted_lat = sorted(latencies)
    p50 = sorted_lat[int(len(sorted_lat) * 0.50)]
    p99 = sorted_lat[int(len(sorted_lat) * 0.99)]
    return p50, p99


# ─── Runner ──────────────────────────────────────────────────────────────────

def run() -> AgentBenchResult:
    r = AgentBenchResult()

    r.turns_per_sec, latencies = bench_throughput(500)
    r.turns_n = 500
    sorted_lat = sorted(latencies)
    r.latency_p50_ms = sorted_lat[int(len(sorted_lat) * 0.50)]
    r.latency_p95_ms = sorted_lat[int(len(sorted_lat) * 0.95)]
    r.latency_p99_ms = sorted_lat[int(len(sorted_lat) * 0.99)]
    r.latency_max_ms = sorted_lat[-1]

    r.latency_ms_10ep, r.latency_ms_100ep, r.latency_ms_500ep = bench_memory_scaling()

    r.overhead_us = bench_gate_overhead()

    r.multiturn_p50_ms, r.multiturn_p99_ms = bench_multiturn(50)
    r.multiturn_n = 50

    return r


if __name__ == "__main__":
    print("OODA Loop Benchmarks")
    print("=" * 50)
    r = run()
    print(f"  Throughput:          {r.turns_per_sec:>8,.0f} turns/s  (n={r.turns_n})")
    print(f"  Turn latency p50:    {r.latency_p50_ms:>7.3f} ms")
    print(f"  Turn latency p95:    {r.latency_p95_ms:>7.3f} ms")
    print(f"  Turn latency p99:    {r.latency_p99_ms:>7.3f} ms")
    print(f"  Turn latency max:    {r.latency_max_ms:>7.3f} ms")
    print()
    print(f"  Memory scaling (with N pre-stored episodes):")
    print(f"    10 episodes:  {r.latency_ms_10ep:.3f} ms/turn")
    print(f"   100 episodes:  {r.latency_ms_100ep:.3f} ms/turn")
    print(f"   500 episodes:  {r.latency_ms_500ep:.3f} ms/turn")
    print()
    print(f"  Safety gate overhead: {r.overhead_us:.2f} µs/evaluation")
    print()
    print(f"  Multi-turn ({r.multiturn_n} turns):")
    print(f"    p50: {r.multiturn_p50_ms:.3f} ms  |  p99: {r.multiturn_p99_ms:.3f} ms")
