# Model Library

Synthesize includes a local metadata-only Model Library.

Supported import path:

- `.gguf` model files for llama.cpp compatibility.

Import records store:

- display name,
- local path,
- format,
- runtime compatibility,
- file size,
- optional sha256 if enabled by the backend command.

Synthesize does not automatically download models in v11. Download open-source coding models manually, verify their license/source/checksum yourself, and import the local GGUF path.
