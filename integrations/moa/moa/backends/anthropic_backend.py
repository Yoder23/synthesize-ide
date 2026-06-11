"""
Anthropic Backend
==================
MoA backend for Anthropic Claude models (claude-3-5-sonnet, etc.).

Install:
    pip install anthropic

Usage:
    from moa.backends.anthropic_backend import AnthropicBackend
    backend = AnthropicBackend(model="claude-3-5-sonnet-20241022")
"""

from __future__ import annotations

from typing import Dict, List, Optional

try:
    import anthropic as _anthropic
    _ANTHROPIC_AVAILABLE = True
except ImportError:
    _ANTHROPIC_AVAILABLE = False


class AnthropicBackend:
    """
    MoA backend for the Anthropic API.

    Text generation only (Anthropic does not expose a public embeddings API).
    MoA falls back to keyword-based memory retrieval when embed() is not available.
    """

    name = "anthropic"

    def __init__(
        self,
        model: str = "claude-3-5-sonnet-20241022",
        api_key: Optional[str] = None,
        **client_kwargs,
    ):
        if not _ANTHROPIC_AVAILABLE:
            raise ImportError(
                "anthropic package is required for AnthropicBackend. "
                "Install with: pip install anthropic"
            )
        self.model = model
        self._client = _anthropic.Anthropic(
            api_key=api_key,  # Falls back to ANTHROPIC_API_KEY env var
            **client_kwargs,
        )
        self.name = f"anthropic/{model}"

    def generate(
        self,
        messages: List[Dict[str, str]],
        max_tokens: int = 512,
        temperature: float = 0.7,
        **kwargs,
    ) -> str:
        # Separate system message from conversation
        system = ""
        chat_messages = []
        for m in messages:
            if m["role"] == "system":
                system = m["content"]
            else:
                chat_messages.append(m)

        response = self._client.messages.create(
            model=self.model,
            system=system or "You are a helpful assistant.",
            messages=chat_messages,
            max_tokens=max_tokens,
            temperature=temperature,
            **kwargs,
        )
        return response.content[0].text

    def embed(self, text: str) -> Optional[List[float]]:
        # Anthropic does not currently expose a public embeddings endpoint.
        return None
