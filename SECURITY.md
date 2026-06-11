# Security Policy

Synthesize is security-conscious but does not claim enterprise production security, OS-level sandboxing, or network isolation.

## Supported security model

- Backend-governed patch validation/apply/rollback.
- Backend-owned context bundles and model calls.
- RepoGuard for repo file access.
- Governed Tasks for backend-detected test/build/lint commands.
- Personal Terminal with strict explicit-rule-only command policy.
- Agent-suggested commands are classification-only.
- Command execution is argv-only, repo-bounded, timeout-bounded, env-scrubbed, output-bounded, reclassified, and audited.

## Personal Terminal security stance

Personal Terminal is not a general shell. It allows a narrow set of local read/test/build commands and blocks unknown commands by default.

Allowed examples include `pnpm test`, `cargo test`, `pytest`, `go test ./...`, `git status`, `git diff`, `git log`, `rg`, `ls`, and `cat`.

Blocked examples include `git add`, `git commit`, `git checkout`, `git pull`, `git fetch`, `git push`, `pnpm exec`, `npm install`, `node`, `python`, `bash`, `sh`, `curl`, `wget`, `rm`, `chmod`, and `sudo`.

## Not currently provided

- OS-level sandboxing.
- Network isolation or egress firewalling.
- Full terminal security model.
- Extension sandboxing.
- Remote development isolation.
- Safe execution of untrusted repositories.

## Reporting issues

Please report issues with clear reproduction steps. Especially important classes:

- repo escape/path traversal
- denied file/context leakage
- frontend authority bypass
- task execution bypass
- Personal Terminal policy bypass
- patch apply/rollback corruption
- unsafe runtime process behavior
