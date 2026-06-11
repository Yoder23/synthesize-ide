# CLAIMS.md — What MoA Actually Proves

This document separates proven claims from aspirational ones.  
"Proven" means there is executable code or a test you can run right now.

---

## ✅ Proven — runnable code, zero deps

### P1: FORBIDDEN action types are rejected by code, not prompts
**Evidence:** `tests/test_safety.py::test_gate_rejects_deceive`  
`SafetyGate.evaluate()` checks `HardConstraints.FORBIDDEN_ACTIONS` (a Python frozenset) before
executing any action. The LLM never sees a FORBIDDEN action because the check runs outside the LLM context.

```python
assert ActionType.DECEIVE_USER in HardConstraints.FORBIDDEN_ACTIONS
gate = SafetyGate()
d = gate.evaluate(Action(type=ActionType.DECEIVE_USER))
assert not d.approved  # always False, regardless of LLM output
```

### P2: SIMULATION and FICTION claims cannot become FACT
**Evidence:** `tests/test_safety.py::test_verifier_rejects_simulation` and `test_verifier_rejects_fiction`  
`FormalVerifier.verify()` checks `claim.modality` before any other check. No amount of evidence
or confidence can override this:

```python
claim = create_claim("event", "would_cause", "effect", Modality.SIMULATION, confidence=0.99)
promoted, _ = FormalVerifier().promote_to_fact(claim, [evidence])
assert promoted == False   # always
```

### P3: EventLog is append-only — nothing is deleted
**Evidence:** `tests/test_ir.py`, `verify_moa.py::check_memory`  
`EventLog` has no `.delete()`, `.pop()`, or `.clear()` method. The only public mutation method is
`.write()`. This is enforced by the class definition, not by runtime checks.

### P4: Actions above threshold are rejected regardless of LLM output
**Evidence:** `tests/test_safety.py::test_gate_rejects_high_irreversibility`  
`HardConstraints.MAX_IRREVERSIBILITY = 0.8`. Any action with `ActionMeta.irreversibility_score > 0.8`
is rejected by `SafetyGate.evaluate()`. Same for `blast_radius > 0.7` and `deception_risk > 0.1`.

### P5: Framework works with any LLM implementing BaseLLMBackend
**Evidence:** `verify_moa.py`, `tests/test_agent.py`  
All 48 tests run with `MockBackend` — zero external dependencies. The same code path runs with
OpenAI, Anthropic, Ollama, HuggingFace, and LayerCake backends.

### P6: Formal verification requires evidence + confidence ≥ 0.8
**Evidence:** `tests/test_safety.py::test_verifier_rejects_low_confidence`  
Claims with `confidence < 0.8` cannot be promoted to FACT even with supporting evidence.
This prevents confident-sounding hallucinations from entering the FACT graph.

---

## ⬜ Aspirational — require production deployment to fully evaluate

### A1: Self-evolving domain modules
**Status:** Architecture exists (`LayerCakeBackend.paste_domain()`). Bit-exact paste proven in
the companion [LayerCake repo](https://github.com/Yoder23/layercake). "Self-evolution" at scale
(agent improves itself over time) requires training infrastructure and long evaluation runs.

### A2: Production-scale multi-agent coordination
**Status:** The OODA loop runs. Multi-agent orchestration (multiple `MoAAgent` instances sharing
memory) is not yet implemented. The memory layer is designed for it (shared `AgentMemory` is
threadsafe for reads), but it has not been stress-tested.

### A3: Semantic memory at scale
**Status:** `EpisodicBuffer.retrieve_semantic()` works with any `embed()` function. Quality of
retrieval depends entirely on the embedding model. `MockBackend` uses MD5-based embeddings that
test the pipeline but not semantic accuracy.

---

## ❌ Not claimed

- **AGI** — This is an agent framework with safety constraints. It is not general intelligence.
- **Superintelligence** — No component here exhibits superhuman capability on any benchmark.
- **Perfect safety** — Hard constraints eliminate specific forbidden action types. They do not
  prevent all possible misuse. Security depends on the threat model.
- **SOTA text generation** — MoA delegates generation to whatever LLM you provide. Generation
  quality equals the quality of the backend model.

---

## How to challenge these claims

Every ✅ claim has a test you can run:

```bash
python verify_moa.py          # 10 checks, 0 deps, < 1 second
pytest tests/ -v              # 48 tests, no API key
```

If a test fails, open an issue. Claims without passing tests get downgraded.
