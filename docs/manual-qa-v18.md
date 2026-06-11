# Manual QA v18

Run this after `./scripts/release-check.sh` passes.

## Fake runtime path

1. Open Synthesize.
2. Open fixture repo.
3. Select Fake Runtime and Fake Demo Agent or Local Patcher.
4. Ask agent for a small patch.
5. Inspect exact context.
6. Validate, approve, apply.
7. Roll back.
8. Confirm Session Log records context/model/patch events.

## Inline selection path

1. Open a file.
2. Select a function.
3. Click Explain selection.
4. Confirm Agent Chat is populated.
5. Ask agent.
6. Confirm response is a report or typed operations.

## Governed task repair path

1. Detect tasks.
2. Approve a detected test/build task.
3. Run it.
4. Click Feed output to agent.
5. Ask agent for a repair.
6. Validate/apply any proposed patch.
7. Rerun the task.

## Local model path

1. Start managed llama.cpp or a manual local model server.
2. Health check.
3. Ask for a small patch on a throwaway repo.
4. Validate/apply/rollback.
5. Confirm no commands run from chat suggestions.
