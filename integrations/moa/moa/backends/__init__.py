"""
MoA backends package.

All backends implement the BaseLLMBackend protocol defined in base.py.
Import only what you need — each backend has its own optional dependency.
"""

from .base import BaseLLMBackend, Message
from .mock import MockBackend

__all__ = [
    "BaseLLMBackend",
    "Message",
    "MockBackend",
]
