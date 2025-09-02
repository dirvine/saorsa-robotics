#!/usr/bin/env bash
set -euo pipefail

if ! command -v huggingface-cli >/dev/null 2>&1; then
  uv pip install huggingface_hub
fi
huggingface-cli login