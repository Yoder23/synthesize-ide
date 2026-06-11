# Session Log

Synthesize records human-readable audit/session events for the governed workflow.

Expected event kinds include:

- `context.bundle_created`
- `runtime.health_check`
- `runtime.endpoint_approved`
- `runtime.request_started`
- `runtime.request_completed`
- `runtime.request_failed`
- `operation.parse_succeeded`
- `operation.parse_failed`
- `patch.validated`
- `patch.approved`
- `patch.applying`
- `patch.checkpoint_created`
- `patch.applied`
- `patch.apply_failed`
- `patch.rolling_back`
- `patch.rolled_back`
- `patch.rollback_failed`
- `command.classified`

Visible logs should show IDs and summaries, not giant prompts, credentials, or full patch bodies.
