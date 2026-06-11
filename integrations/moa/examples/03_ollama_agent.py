"""
Example 03 — Ollama backend (local, no API key)
================================================
Requires:
  1. Install Ollama: https://ollama.ai
  2. Pull a model: ollama pull llama3
  3. pip install moa[ollama]

Run:
    python examples/03_ollama_agent.py
    python examples/03_ollama_agent.py mistral     # choose a different model
"""

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from moa import MoAAgent

try:
    from moa.backends.ollama_backend import OllamaBackend
except ImportError:
    print("Install the ollama backend:  pip install moa[ollama]")
    sys.exit(1)

model_name = sys.argv[1] if len(sys.argv) > 1 else "llama3"

print(f"Using Ollama model: {model_name}")
print("Make sure Ollama is running: ollama serve")
print()

backend = OllamaBackend(
    model=model_name,
    base_url="http://localhost:11434",
)

agent = MoAAgent(
    backend=backend,
    storage_dir=f"moa_state/ollama_{model_name}",
)

# Test the agent
result = agent.run("Explain the OODA loop in 3 sentences.")
print(f"> {result.response}")
print(f"\nBackend: {backend.name}")
print(f"Safety: {agent.safety_stats}")

# Demonstrate semantic memory (Ollama provides embeddings via /api/embeddings)
agent.run("The OODA loop was developed by John Boyd for air combat.")
agent.run("Boyd also created Energy–Maneuverability theory.")

# Third query — memory should recall relevant episodes
result = agent.run("Who created the OODA loop?")
print(f"\n[With memory context]")
print(f"> {result.response}")
