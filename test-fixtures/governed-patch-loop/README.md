# Governed Patch Loop Fixture

This fixture mirrors the temp repo created by `open_repo_mock` in the Tauri backend.

It exists so reviewers can inspect the exact source content that the fake runtime patch is meant to modify. The manifest includes the expected `beforeSha256` for the fixture file.

The v3 UI path computes the hash from the current editor content, passes it into `FakeRuntimeAdapter`, validates it through the backend, and applies the patch only after explicit approval.
