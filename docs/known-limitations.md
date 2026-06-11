# Known Limitations

Synthesize v19.2 is a local-first AI IDE personal-production candidate, not a full VS Code clone or enterprise production security product.

## IDE limitations

- No full LSP implementation yet.
- No full debugger/DAP implementation yet.
- No extension marketplace.
- No remote development, WSL, SSH, or dev-container workflow.
- Monaco editor features are intentionally narrower than VS Code.

## Runtime limitations

- Local model quality depends on the model and server you run.
- Managed llama.cpp starts a user-provided binary; Synthesize does not ship models or llama.cpp.
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
