#!/usr/bin/env bash
set -euo pipefail

# Ensure Homebrew (optional for ffmpeg etc.)
if ! command -v brew >/dev/null 2>&1; then
  echo "Homebrew not found. Install from https://brew.sh (recommended)."
fi

# Ensure uv
if ! command -v uv >/dev/null 2>&1; then
  curl -LsSf https://astral.sh/uv/install.sh | sh
  export PATH="$HOME/.local/bin:$PATH"
fi

# Python env
uv venv
source .venv/bin/activate

# Core Python deps from pyproject
uv pip install -e .

# LeRobot from PyPI with Feetech + async extras
uv pip install 'lerobot[feetech,async]'

# OpenCV preview utils
if ! command -v ffmpeg >/dev/null 2>&1; then
  brew install ffmpeg || true
fi

# Torch for MPS (Apple Silicon)
# Use PyTorch's selector script to grab correct wheels.
python - <<'PY'
import sys, subprocess
cmd = [sys.executable, "-m", "pip", "install", "torch==2.3.1", "--index-url", "https://download.pytorch.org/whl/cpu"]
subprocess.check_call(cmd)
print("Installed torch CPU; switching to MPS at runtime if available.")
PY

# MPS sanity check
python scripts/check_mps.py || true

echo "[mac] bootstrap complete. Copy .env.example to .env and edit if needed."