"""
Example 01 — Basic agent with MockBackend
==========================================
No API key. No GPU. No network.
Demonstrates the full OODA loop and safety gate.

Run:
    python examples/01_basic_agent.py
"""

import sys
from pathlib import Path

# Make sure moa is importable when running from examples/
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from moa import MoAAgent
from moa.backends import MockBackend

# ── 1. Create an agent ────────────────────────────────────────────────────────

backend = MockBackend(responses={
    "chess":   "1.e4 e5 2.Nf3 Nc6 is the Spanish or Italian depending on White's third move.",
    "python":  "Python is an interpreted, dynamically-typed language by Guido van Rossum.",
    "safety":  "Hard constraints are code, not prompts. They cannot be jailbroken.",
})

agent = MoAAgent(
    backend=backend,
    system_prompt="You are a helpful assistant. Be concise.",
)

# ── 2. Run some queries ───────────────────────────────────────────────────────

queries = [
    "Tell me about chess openings.",
    "Explain Python in one sentence.",
    "How does MoA handle safety?",
]

for q in queries:
    print(f"\n> {q}")
    result = agent.run(q)
    print(f"  {result.response}")
    print(f"  Actions: {len(result.actions_taken)} approved, "
          f"{len(result.actions_rejected)} rejected "
          f"({result.duration_s:.3f}s)")

# ── 3. Store a verified fact ──────────────────────────────────────────────────

print("\n--- Storing a verified fact ---")
fact = agent.store_fact(
    subject="MoA",
    predicate="forbids",
    object_="DECEIVE_USER",
    evidence_content="HardConstraints.FORBIDDEN_ACTIONS contains ActionType.DECEIVE_USER",
    confidence=0.99,
)

if fact:
    print(f"  Stored: [{fact.subject}] {fact.predicate} [{fact.object}]")
    print(f"  Verified: {fact.is_verified()}")
else:
    print("  Fact rejected (should not happen in this example)")

# ── 4. Stats ──────────────────────────────────────────────────────────────────

print(f"\n--- Session stats ---")
print(f"  Steps: {agent.step}")
print(f"  Safety: {agent.safety_stats}")
print(f"  Episodes stored: {agent.memory.episodes.size}")
print(f"  Events logged: {agent.memory.event_log.entry_count}")
