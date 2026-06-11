"""
Ollama Backend
===============
MoA backend for local Ollama models (Llama 3, Mistral, Qwen, etc.).

No API key required. Runs entirely on your machine.

Install Ollama: https://ollama.com
Pull a model:   ollama pull llama3
Install dep:    pip install requests

Usage:
    from moa.backends.ollama_backend import OllamaBackend
    backend = OllamaBackend(model="llama3")
"""

from __future__ import annotations

import json
from typing import Dict, List, Optional

try:
    import requests as _requests
    _REQUESTS_AVAILABLE = True
except ImportError:
    _REQUESTS_AVAILABLE = False


class OllamaBackend:
    """
    MoA backend for locally-running Ollama models.

    Supports text generation and optional embeddings
    (via the Ollama /api/embeddings endpoint).
    """

    name = "ollama"

    def __init__(
        self,
        model: str = "llama3",
        base_url: str = "http://localhost:11434",
        embedding_model: Optional[str] = None,
        timeout: int = 120,
    ):
        if not _REQUESTS_AVAILABLE:
            raise ImportError(
                "requests package is required for OllamaBackend. "
                "Install with: pip install requests"
            )
        self.model = model
        self.base_url = base_url.rstrip("/")
        self.embedding_model = embedding_model or model
        self.timeout = timeout
        self.name = f"ollama/{model}"

    def generate(
        self,
        messages: List[Dict[str, str]],
        max_tokens: int = 512,
        temperature: float = 0.7,
        **kwargs,
    ) -> str:
        url = f"{self.base_url}/api/chat"
        payload = {
            "model": self.model,
            "messages": messages,
            "stream": False,
            "options": {
                "num_predict": max_tokens,
                "temperature": temperature,
            },
        }
        response = _requests.post(url, json=payload, timeout=self.timeout)
        response.raise_for_status()
        data = response.json()
        return data["message"]["content"]

    def embed(self, text: str) -> Optional[List[float]]:
        url = f"{self.base_url}/api/embeddings"
        payload = {"model": self.embedding_model, "prompt": text}
        try:
            response = _requests.post(url, json=payload, timeout=self.timeout)
            response.raise_for_status()
            return response.json()["embedding"]
        except Exception:
            return None
