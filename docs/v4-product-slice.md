# V4 product slice: usable governed workbench

V4 moves Synthesize from fixture-only proof toward a usable local-first IDE slice.

## What is wired

- Open a deterministic fixture repo automatically.
- Open a user-supplied local repo path.
- Navigate repo files through guarded backend reads.
- Show dirty-buffer state and disable planning when the editor buffer no longer matches disk.
- Build a visible context bundle preview.
- Use the deterministic fake runtime to emit typed `propose_patch` operations.
- Parse strict JSON/fenced JSON operation payloads.
- Render proposed patches in the diff queue.
- Validate patches through the backend using RepoGuard, beforeSha256, current commit, diff header path matching, and at least one hunk.
- Apply approved patches with a checkpoint.
- Roll back from the last checkpoint.
- Show audit events in the Session Log panel.
- Classify requested commands through the backend command guard.
- Register a local GGUF model path and show curated model lanes.

## What is intentionally not overclaimed

- The fake runtime is still the active deterministic runtime for v4.
- Real llama.cpp process supervision is scaffolded but not fully wired.
- Model downloads are represented as curated lanes and local path registration; a checksum-verified downloader is next.
- Command execution is classified but not enabled from the UI.
- Command network isolation is not OS-enforced yet.
- The restricted runner is not yet a true container/bubblewrap/firejail/WSL sandbox.

## V5 target

The next high-leverage milestone is the guarded command path:

1. Command request appears from patch proposal.
2. Backend classifies risk.
3. User explicitly approves.
4. Restricted runner executes with timeout, clean env, output cap, cwd guard.
5. Output is logged and fed back into the agent harness.

## V6 target

Real local model runtime:

1. Import GGUF.
2. Locate or bundle llama.cpp server.
3. Launch supervised local server.
4. Health check.
5. Stream tokens into the same operation parser.
6. Use json_schema response mode where supported.
