# Synthesize v16 Manual QA

Run this after the release gate passes.

## Editor workbench

1. Open fixture repo.
2. Open several files through Repo Explorer and Quick Open.
3. Confirm tabs appear and dirty indicators work.
4. Modify a file and save through Command Palette.
5. Refresh from disk.
6. Create a new file under a nested directory.
7. Rename the file.
8. Delete the file.
9. Confirm Session Log contains file mutation events.

## Local agent patch lifecycle

1. Use Fake Runtime end-to-end.
2. Validate, approve, apply, and rollback the patch.
3. Confirm open tabs refresh after apply/rollback.

## Git workbench

1. Modify a file.
2. Refresh Source Control.
3. Stage the file.
4. Unstage the file.
5. Stage again.
6. Commit with a test message.
7. Confirm Git events appear in Session Log.

## Governed task runner

1. Detect tasks.
2. Approve a detected task by task id.
3. Run the approved task.
4. Confirm arbitrary command approval is not possible.
5. Confirm output is bounded and audited.

## Local model runtime

1. Start managed llama.cpp or a manual local model server.
2. Health check.
3. Ask Local Patcher for a small patch.
4. Validate/apply/rollback.
