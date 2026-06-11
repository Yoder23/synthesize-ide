"""
Example 04 — LayerCake native backend
=======================================
Requires:
  pip install moa[layercake]
  git clone https://github.com/Yoder23/layercake && pip install -e layercake

Run:
    python examples/04_layercake_agent.py --checkpoint core.pt --tokenizer sp.model

This example shows features unique to the LayerCake backend:
  - get_h_abi(text)        → 512-dim ABI representation
  - paste_domain(name, m)  → bit-exact hot-swap of domain modules
  - ABI-based semantic memory (portable across model sizes)
"""

import sys
import argparse
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from moa import MoAAgent

try:
    import torch
    from moa.backends.layercake_backend import LayerCakeBackend
except ImportError:
    print("Install the layercake backend:  pip install moa[layercake]")
    print("Then:  git clone https://github.com/Yoder23/layercake && pip install -e layercake")
    sys.exit(1)

parser = argparse.ArgumentParser()
parser.add_argument("--checkpoint", required=True, help="Path to LayerCake core checkpoint")
parser.add_argument("--tokenizer", required=True, help="Path to SentencePiece tokenizer")
parser.add_argument("--device", default="cpu", choices=["cpu", "cuda"])
args = parser.parse_args()

print(f"Loading LayerCake from {args.checkpoint} ...")
backend = LayerCakeBackend.from_checkpoint(
    checkpoint_path=args.checkpoint,
    tokenizer_path=args.tokenizer,
    device=args.device,
)
print(f"Backend: {backend.name}")

agent = MoAAgent(
    backend=backend,
    storage_dir="moa_state/layercake",
)

# ── Feature 1: ABI representation ────────────────────────────────────────────
text = "e4 e5 Nf3 Nc6 Bb5"
h_abi = backend.get_h_abi(text)
print(f"\nget_h_abi('{text}')")
print(f"  Shape: {h_abi.shape}   (seq_len x 512)")
print(f"  Mean:  {h_abi.mean():.4f}  Std: {h_abi.std():.4f}")

# ── Feature 2: Embedding via ABI (powers semantic memory) ────────────────────
embedding = backend.embed(text)
print(f"\nembed() dimension: {len(embedding)}")

# ── Feature 3: Agent run ──────────────────────────────────────────────────────
print("\n--- Agent run ---")
result = agent.run("Analyse the chess position after e4 e5 Nf3 Nc6 Bb5")
print(f"> {result.response[:300]}")

# ── Feature 4: domain paste (if a chess domain module is available) ────────────
chess_module_path = Path("chess_domain.pt")
if chess_module_path.exists():
    print("\n--- Pasting chess domain module ---")
    chess_module = torch.load(chess_module_path, map_location=args.device)
    n_copied = backend.paste_domain("chess", chess_module)
    print(f"  Pasted {n_copied} tensors (bit-exact)")
    result2 = agent.run("Evaluate White's position after Bb5.")
    print(f"> {result2.response[:300]}")
else:
    print("\n(Skipping domain paste — chess_domain.pt not found)")
    print("Train a domain module with: https://github.com/Yoder23/layercake")

print(f"\nSafety stats: {agent.safety_stats}")
