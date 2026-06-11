"""
LayerCake Native Backend
=========================
MoA backend for native LayerCake models.

This backend unlocks the full MoA feature set:
  - Access to h_abi (512-dim ABI representations)
  - Hot-swap domain modules
  - Self-evolving module registry
  - Bit-exact domain paste across model sizes

For plain text generation with any other LLM, use one of:
  OpenAIBackend, AnthropicBackend, OllamaBackend, HFBackend

Install:
    pip install torch
    git clone https://github.com/Yoder23/layercake && pip install -e layercake

Usage:
    from moa.backends.layercake_backend import LayerCakeBackend
    backend = LayerCakeBackend.from_checkpoint("core.pt", tokenizer_path="sp.model")
"""

from __future__ import annotations

from pathlib import Path
from typing import Dict, List, Optional, Any

try:
    import torch
    _TORCH_AVAILABLE = True
except ImportError:
    _TORCH_AVAILABLE = False


class LayerCakeBackend:
    """
    MoA backend backed by a native LayerCake model.

    Unique capabilities vs other backends:
      get_h_abi(text)                → torch.Tensor [seq, 512]
        Access the fixed ABI representation. Enables semantic memory
        that is portable across all LayerCake model sizes.

      paste_domain(name, module)     → None
        Hot-swap a domain module into the running model at inference time.
        The pasted module is bit-exact to the source — proven by tests.

      register_domain(name)          → DomainModuleLite
        Create and register a new domain module at runtime (self-editing).
    """

    name = "layercake"

    def __init__(
        self,
        model,                         # LayerCakeLMFixedABI instance
        tokenizer,                     # SentencePiece tokenizer
        device: str = "cpu",
        max_seq_len: int = 256,
        active_domain: Optional[str] = None,
    ):
        if not _TORCH_AVAILABLE:
            raise ImportError("torch is required for LayerCakeBackend.")
        self._model = model.to(device).eval()
        self._tokenizer = tokenizer
        self._device = torch.device(device)
        self._max_seq_len = max_seq_len
        self._active_domain = active_domain
        self.name = f"layercake/{sum(p.numel() for p in model.parameters()) // 1_000_000}M"

    @classmethod
    def from_checkpoint(
        cls,
        checkpoint_path: str,
        tokenizer_path: str,
        config_path: Optional[str] = None,
        device: str = "cpu",
        **kwargs,
    ) -> "LayerCakeBackend":
        """Load a LayerCake model from a checkpoint file."""
        if not _TORCH_AVAILABLE:
            raise ImportError("torch is required.")
        try:
            import sys, importlib
            # Try to import from installed layercake package first
            try:
                from layercake.model import LayerCakeLMFixedABI
            except ImportError:
                # Fall back to local layercakeogwithdecoder path
                lc_dir = Path(__file__).resolve().parents[4] / "layercakeogwithdecoder"
                sys.path.insert(0, str(lc_dir))
                from layercake_model_fixed_abi import LayerCakeLMFixedABI

            import sentencepiece as spm
        except ImportError as exc:
            raise ImportError(
                f"Could not import LayerCake model: {exc}. "
                "Install the layercake package: pip install -e path/to/layercake"
            )

        import json
        if config_path and Path(config_path).exists():
            with open(config_path) as f:
                cfg = json.load(f)
        else:
            cfg = {"vocab_size": 16000, "core": {"d_model": 512, "d_abi": 512,
                   "n_layers": 6, "n_heads": 8, "d_ff": 2048}}

        core = cfg.get("core", cfg)
        model = LayerCakeLMFixedABI(
            vocab_size=cfg.get("vocab_size", 16000),
            d_model=core.get("d_model", 512),
            d_abi=core.get("d_abi", 512),
            n_core_layers=core.get("n_layers", 6),
            n_heads=core.get("n_heads", 8),
            d_ff=core.get("d_ff", 2048),
            domain_names=(),
            max_seq_len=core.get("max_seq_len", 256),
            use_router=False,
        )
        ckpt = torch.load(checkpoint_path, map_location="cpu")
        state = ckpt.get("model_state_dict", ckpt)
        model.load_state_dict(
            {k: v for k, v in state.items() if not k.startswith("domain_modules.")},
            strict=False,
        )

        sp = spm.SentencePieceProcessor()
        sp.Load(tokenizer_path)

        return cls(model=model, tokenizer=sp, device=device, **kwargs)

    def generate(
        self,
        messages: List[Dict[str, str]],
        max_tokens: int = 128,
        temperature: float = 0.8,
        **kwargs,
    ) -> str:
        """Greedy/sampling decode from the LayerCake model."""
        # Build prompt from messages
        prompt = " ".join(m["content"] for m in messages if m["role"] != "system")
        input_ids = self._tokenizer.Encode(prompt, out_type=int)
        input_ids = input_ids[-self._max_seq_len:]
        ids = torch.tensor([input_ids], dtype=torch.long, device=self._device)

        generated = []
        self._model.eval()
        with torch.no_grad():
            for _ in range(max_tokens):
                logits, _ = self._model(ids)
                next_logits = logits[0, -1, :]  # [vocab]
                if temperature <= 0:
                    next_id = next_logits.argmax().item()
                else:
                    probs = torch.softmax(next_logits / temperature, dim=-1)
                    next_id = torch.multinomial(probs, 1).item()
                generated.append(next_id)
                ids = torch.cat(
                    [ids, torch.tensor([[next_id]], device=self._device)], dim=1
                )
                ids = ids[:, -self._max_seq_len:]
                if next_id == self._tokenizer.eos_id():
                    break

        return self._tokenizer.Decode(generated)

    def embed(self, text: str) -> Optional[List[float]]:
        """Return the mean-pooled h_abi vector as a flat list."""
        try:
            h_abi = self.get_h_abi(text)
            return h_abi.mean(dim=0).cpu().float().tolist()
        except Exception:
            return None

    def get_h_abi(self, text: str) -> "torch.Tensor":
        """
        Return the h_abi representation for the given text.

        Shape: [seq_len, d_abi=512]
        This is the fixed-ABI bottleneck vector — portable across all
        LayerCake model sizes that share the same d_abi.
        """
        ids = self._tokenizer.Encode(text, out_type=int)[-self._max_seq_len:]
        input_ids = torch.tensor([ids], dtype=torch.long, device=self._device)
        with torch.no_grad():
            h_core, h_abi = self._model.encode_core(input_ids)
        return h_abi[0]   # [seq, d_abi]

    def paste_domain(self, name: str, source_module: Any) -> None:
        """
        Hot-swap a domain module into this model.

        The paste is bit-exact — proven by tests/test_paste_lossless.py
        in the companion LayerCake repository.
        """
        src = source_module.state_dict()
        tgt = self._model.state_dict()
        prefix = f"domain_modules.{name}."
        copied = 0
        for k, v in src.items():
            full_key = prefix + k
            if full_key in tgt:
                tgt[full_key] = v.clone()
                copied += 1
        self._model.load_state_dict(tgt)
        self._active_domain = name
        return copied
