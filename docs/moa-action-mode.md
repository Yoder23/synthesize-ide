# MoA Action Mode

Synthesize IDE can run in a **MoA Action Planner** profile. In this mode, the local model is used for planning and proposal generation, while Synthesize/MoA governance remains the actor.

The invariant is:

> The model proposes. MoA/Synthesize validates, approves, applies, rolls back, and audits.

## Flow

1. The user opens a repo and selects **MoA Action Planner** in Local Agent Profile.
2. The user asks for a coding action in Agent Chat.
3. The backend builds an exact context bundle and records the prompt hash.
4. The local model emits strict Synthesize typed operations.
5. The IDE renders a plan/action trace from typed operations.
6. Patch proposals enter the diff queue.
7. The trusted backend validates file paths, before-hashes, policy, lifecycle state, and approval.
8. The user approves or rejects.
9. The backend applies transactionally with a checkpoint or rolls back.
10. The audit log records context, runtime request, operation parse, validation, approval, apply, rollback, and command policy events.

## What is shown in the chat window

The chat window shows a **plan/action trace** derived from typed operations and report fields. It does not claim to expose private model chain-of-thought. This keeps the product useful, inspectable, and safer for demos and publication.

## Local model requirement

MoA Action Mode works with:

- Fake Runtime for deterministic demos.
- Local OpenAI-compatible endpoints such as Ollama, LM Studio, llama.cpp server, or vLLM.
- Managed llama.cpp runtime when configured.

Remote or private-LAN endpoints require explicit endpoint approval before repository context is sent.

## Submission demo idea

Use MoA Action Mode to repair a failing test:

1. Open the fixture repo.
2. Select **MoA Action Planner**.
3. Click **Draft MoA action**.
4. Ask the agent.
5. Show the action trace.
6. Validate and approve the patch.
7. Run the suggested test through governed tasks/personal terminal.
8. Show an unsafe command being blocked.
9. Show the audit log.
