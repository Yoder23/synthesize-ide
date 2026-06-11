"""
Example 02 — OpenAI backend
============================
Requires: pip install moa[openai]
Set your API key: export OPENAI_API_KEY=sk-...

Run:
    python examples/02_openai_agent.py
"""

import os
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from moa import MoAAgent

try:
    from moa.backends.openai_backend import OpenAIBackend
except ImportError:
    print("Install the openai backend:  pip install moa[openai]")
    sys.exit(1)

api_key = os.environ.get("OPENAI_API_KEY", "")
if not api_key:
    print("Set OPENAI_API_KEY environment variable.")
    sys.exit(1)

# ── GPT-4o ────────────────────────────────────────────────────────────────────
backend = OpenAIBackend(
    model="gpt-4o",
    api_key=api_key,
)

agent = MoAAgent(
    backend=backend,
    system_prompt=(
        "You are a chess analysis assistant. "
        "Give concise, accurate opening theory."
    ),
    storage_dir="moa_state/openai",  # Persist memory to disk
    audit_path="moa_state/openai_audit.jsonl",
)

# Multi-turn conversation with persistent memory
turns = [
    "Analyse 1.e4 e5 2.Nf3 Nc6 3.Bb5 — the Ruy Lopez.",
    "What are Black's main responses?",
    "Which response is most popular at the GM level?",
]

for turn in turns:
    print(f"\n> {turn}")
    result = agent.run(turn, domain="chess")
    print(f"  {result.response}")

print(f"\nSafety stats: {agent.safety_stats}")
print(f"Episodes stored: {agent.memory.episodes.size}")
