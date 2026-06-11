"""
OpenAI Backend
===============
MoA backend for OpenAI GPT models (GPT-4o, GPT-4-turbo, GPT-3.5, etc.).

Install:
    pip install openai

Usage:
    from moa.backends.openai import OpenAIBackend
    backend = OpenAIBackend(model="gpt-4o")
"""

from __future__ import annotations

from typing import Dict, List, Optional

try:
    import openai as _openai
    _OPENAI_AVAILABLE = True
except ImportError:
    _OPENAI_AVAILABLE = False


class OpenAIBackend:
    """
    MoA backend for the OpenAI API.

    Supports text generation (all chat models) and embeddings
    (text-embedding-3-small / text-embedding-ada-002).
    """

    name = "openai"

    def __init__(
        self,
        model: str = "gpt-4o",
        embedding_model: str = "text-embedding-3-small",
        api_key: Optional[str] = None,
        **client_kwargs,
    ):
        if not _OPENAI_AVAILABLE:
            raise ImportError(
                "openai package is required for OpenAIBackend. "
                "Install with: pip install openai"
            )
        self.model = model
        self.embedding_model = embedding_model
        self._client = _openai.OpenAI(
            api_key=api_key,  # Falls back to OPENAI_API_KEY env var
            **client_kwargs,
        )
        self.name = f"openai/{model}"

    def generate(
        self,
        messages: List[Dict[str, str]],
        max_tokens: int = 512,
        temperature: float = 0.7,
        **kwargs,
    ) -> str:
        response = self._client.chat.completions.create(
            model=self.model,
            messages=messages,
            max_tokens=max_tokens,
            temperature=temperature,
            **kwargs,
        )
        return response.choices[0].message.content or ""

    def embed(self, text: str) -> List[float]:
        response = self._client.embeddings.create(
            model=self.embedding_model,
            input=text,
        )
        return response.data[0].embedding
