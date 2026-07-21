# Known Limitations

Synthesize is a local-first AI IDE and outcome-governed Studio candidate, not a full VS Code clone or enterprise production security product.

## IDE limitations

- No full LSP implementation yet.
- No full debugger/DAP implementation yet.
- No extension marketplace.
- No remote development, WSL, SSH, or dev-container workflow.
- Monaco editor features are intentionally narrower than VS Code.

## Runtime limitations

- Local model quality depends on the model and server you run.
- The general runtime adapter uses a conservative, explicitly labeled UTF-8-byte token estimate when a matching runtime tokenizer is unavailable. It rejects a configuration that claims `runtime_tokenizer` without an installed counter, but estimates can still differ from a provider's tokenizer.
- Managed llama.cpp starts local binary/model paths. The bootstrap script can download a smoke GGUF model and llama.cpp binary into `.synthesize-runtime/`, but the desktop app does not yet provide an in-app progress UI for downloads.
- Private-LAN and remote local-model endpoints require explicit approval, but Synthesize does not claim OS-level network sandboxing.

## Patch limitations

- Model output is untrusted and must parse into typed operations.
- Apply/rollback are backend-governed and checkpointed, but not a substitute for Git and backups.
- Use clean branches for serious work.

## Command execution limitations

Synthesize v19.2 has two bounded command execution pathways:

- Governed Tasks: backend-detected and persisted test/build/lint commands.
- Personal Terminal: user-entered commands allowed only through strict explicit safe rules.

Personal Terminal intentionally blocks unknown commands and common mutation/network commands such as `git add`, `git checkout`, `git pull`, `pnpm exec`, `node`, `python`, `bash`, `curl`, and `rm`.

Synthesize does not provide a general shell or terminal emulator.

## Security limitations

- No OS-level sandbox is claimed.
- No container isolation is claimed.
- No network egress firewall is claimed.
- Do not open untrusted repos and run commands inside them.
- Do not treat Synthesize as an enterprise policy-enforcement product.

## Studio and Dream limitations

- Runtime/model quality depends on the configured role runtime. The deterministic Fake Runtime proves control flow, not real-model output quality.
- Dream continuous operation exists only as repeated bounded cycles while enabled and while the application is running; it is not a background service.
- Dream never merges autonomously. A human must approve prototype/incubator autonomy, promote goals, review diffs, and merge.
- Governed Git worktrees isolate the active checkout but are not containers or OS sandboxes.
- Proof reports establish recorded traceability and evidence, not that a business outcome has occurred. Outcome-pending remains explicit.
- Pulse findings are advisory. The experimental liquid observer is shadow-only, requires validated calibrated weights, and does not establish truth.
- Declarative prototypes are intentionally limited to allowlisted primitives and local scalar state.
- Initial repository context retrieval is deterministic and lexical; semantic embeddings are deliberately not required. Very large mandatory specifications must be partitioned or narrowed rather than silently truncated.
