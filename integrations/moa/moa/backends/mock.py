"""
Mock LLM Backend
=================
A deterministic, zero-dependency backend for testing and CI.

No API keys. No network. No GPU. Instant responses.
Used by verify_moa.py and the test suite.
"""

from __future__ import annotations

import hashlib
from typing import Dict, List, Optional


class MockBackend:
    """
    Deterministic mock LLM for testing.

    Responses are based on a simple keyword lookup table so tests
    are repeatable without any external dependencies.

    Also provides embed() via a deterministic hashing scheme so
    semantic memory retrieval can be tested without a real embedding model.
    """

    name = "mock"

    def __init__(self, responses: Optional[Dict[str, str]] = None):
        """
        Args:
            responses: Optional dict mapping keyword → response text.
                       If a message contains the keyword, that response
                       is returned. Falls back to a default reply.
        """
        self._responses = responses or {}
        self._call_count = 0

    def generate(
        self,
        messages: List[Dict[str, str]],
        max_tokens: int = 512,
        temperature: float = 0.7,
        **kwargs,
    ) -> str:
        self._call_count += 1
        last_user = ""
        for m in reversed(messages):
            if m.get("role") == "user":
                last_user = m.get("content", "").lower()
                break

        for keyword, response in self._responses.items():
            if keyword.lower() in last_user:
                return response

        # Default: echo a brief reply
        return f"[mock response #{self._call_count}] Understood: {last_user[:60]}"

    def embed(self, text: str) -> List[float]:
        """
        Deterministic pseudo-embedding via MD5 hashing.

        Not semantically meaningful, but stable and reproducible — suitable
        for testing that the memory retrieval pipeline works correctly.
        """
        h = hashlib.md5(text.encode()).digest()   # 16 bytes
        # Expand to 32 floats in [-1, 1] by interpreting each byte
        vec = [(b / 127.5) - 1.0 for b in h] * 2   # 32-dim
        norm = (sum(x * x for x in vec) ** 0.5) or 1.0
        return [x / norm for x in vec]

    @property
    def call_count(self) -> int:
        return self._call_count
