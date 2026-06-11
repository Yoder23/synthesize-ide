"""
HuggingFace Backend
====================
MoA backend for any HuggingFace text-generation model.

Works with instruction-tuned models from the Hub:
  Qwen2.5, Mistral, Llama, Phi-3, Falcon, etc.

Install:
    pip install transformers torch accelerate

Usage:
    from moa.backends.hf_backend import HFBackend
    backend = HFBackend("Qwen/Qwen2.5-7B-Instruct")
"""

from __future__ import annotations

from typing import Dict, List, Optional

try:
    from transformers import AutoTokenizer, AutoModelForCausalLM, pipeline
    import torch as _torch
    _HF_AVAILABLE = True
except ImportError:
    _HF_AVAILABLE = False


class HFBackend:
    """
    MoA backend for HuggingFace text-generation models.

    Uses the text-generation pipeline under the hood. Models are loaded
    on first use and cached in memory. For large models, pass device_map="auto"
    and ensure you have enough VRAM.
    """

    def __init__(
        self,
        model_id: str,
        device_map: str = "auto",
        torch_dtype: str = "auto",
        trust_remote_code: bool = False,
        **pipeline_kwargs,
    ):
        if not _HF_AVAILABLE:
            raise ImportError(
                "transformers and torch are required for HFBackend. "
                "Install with: pip install transformers torch accelerate"
            )
        self.model_id = model_id
        self.name = f"hf/{model_id.split('/')[-1]}"

        tokenizer = AutoTokenizer.from_pretrained(
            model_id, trust_remote_code=trust_remote_code
        )
        model = AutoModelForCausalLM.from_pretrained(
            model_id,
            device_map=device_map,
            torch_dtype=torch_dtype,
            trust_remote_code=trust_remote_code,
        )
        self._pipe = pipeline(
            "text-generation",
            model=model,
            tokenizer=tokenizer,
            **pipeline_kwargs,
        )
        self._tokenizer = tokenizer

    def generate(
        self,
        messages: List[Dict[str, str]],
        max_tokens: int = 512,
        temperature: float = 0.7,
        **kwargs,
    ) -> str:
        # Use chat template if available (works for instruction-tuned models)
        prompt = self._tokenizer.apply_chat_template(
            messages, tokenize=False, add_generation_prompt=True
        )
        outputs = self._pipe(
            prompt,
            max_new_tokens=max_tokens,
            temperature=temperature,
            do_sample=temperature > 0,
            return_full_text=False,
            **kwargs,
        )
        return outputs[0]["generated_text"]

    def embed(self, text: str) -> Optional[List[float]]:
        # Use mean-pool over last hidden states for a simple embedding
        try:
            import torch
            tokens = self._tokenizer(
                text, return_tensors="pt", truncation=True, max_length=512
            ).to(self._pipe.device)
            with torch.no_grad():
                outputs = self._pipe.model(**tokens, output_hidden_states=True)
                hidden = outputs.hidden_states[-1].mean(dim=1).squeeze(0)
            return hidden.cpu().float().tolist()
        except Exception:
            return None
