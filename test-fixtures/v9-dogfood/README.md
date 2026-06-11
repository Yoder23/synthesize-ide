# V9 Dogfood Fixture Path

Manual QA path when UI e2e tooling is unavailable:

1. Open fixture repo.
2. Use Fake Runtime.
3. Ask agent for default auth refresh task.
4. Confirm backend context bundle is created and visible.
5. Confirm fake runtime returns a typed `propose_patch` operation.
6. Validate proposal.
7. Approve proposal.
8. Apply proposal.
9. Confirm `src/auth/refresh.ts` now returns `"refreshed"`.
10. Roll back by proposal id.
11. Confirm the file is restored.
12. Confirm Session Log contains context/runtime/patch/checkpoint/rollback events.

Mock endpoint path:

- Run a local OpenAI-compatible server returning a JSON typed operation.
- Configure endpoint as localhost in Runtime Control.
- Test connection through backend.
- Ask agent and verify the same governed patch lifecycle.
