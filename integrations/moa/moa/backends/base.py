"""
MoA Backend Protocol
=====================
A minimal protocol any LLM must implement to work as a MoA backend.

Implementing this protocol is all that is required to plug any LLM —
OpenAI, Anthropic, Ollama, HuggingFace, LayerCake, or your own model —
into the MoA framework.

The two required methods:
  generate(messages) → str     Generate a response from a message list.
  name → str                   A short identifier for logging.

Optional method:
  embed(text) → list[float]    Return an embedding vector.
                               Enables semantic memory retrieval.
                               If not implemented, keyword retrieval is used.
"""

from __future__ import annotations

from typing import Dict, List, Optional, Protocol, runtime_checkable


Message = Dict[str, str]   # {"role": "user"|"system"|"assistant", "content": "..."}


@runtime_checkable
class BaseLLMBackend(Protocol):
    """
    Protocol for MoA LLM backends.

    Implement generate() and you're done.
    Implement embed() to unlock semantic memory retrieval.
    """

    @property
    def name(self) -> str:
        """Short identifier used in logging and telemetry."""
        ...

    def generate(
        self,
        messages: List[Message],
        max_tokens: int = 512,
        temperature: float = 0.7,
        **kwargs,
    ) -> str:
        """
        Generate a response from a conversation.

        Args:
            messages: List of {"role": ..., "content": ...} dicts.
                      Always in OpenAI message format for portability.
            max_tokens: Maximum tokens to generate.
            temperature: Sampling temperature (0.0 = deterministic).

        Returns:
            str: The assistant's response text.
        """
        ...

    def embed(self, text: str) -> Optional[List[float]]:
        """
        Return a float embedding vector for the given text.

        Returns None if this backend does not support embeddings.
        MoA falls back to keyword search when embeddings are unavailable.
        """
        return None
