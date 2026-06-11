# V8 Patch Applier Fixtures

V8 adds tests/fixtures for the supported text unified-diff subset:

- valid single-hunk patch
- valid multi-hunk patch with offsets
- context mismatch rejection
- malformed hunk count rejection
- nested file creation
- binary patch rejection
- rename/delete/mode change rejection
- no-op rejection
- path mismatch / outside repo / denied .env inherited from earlier fixtures
