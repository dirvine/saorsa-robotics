#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

# Load env
if [ -f ../.env ]; then set -a; source ../.env; set +a; fi
HOST="${POLICY_SERVER_HOST:-0.0.0.0}"
PORT="${POLICY_SERVER_PORT:-8080}"

# We assume OpenPI is installed on the GPU host under $HOME/openpi
OPENPI_DIR="${OPENPI_DIR:-$HOME/openpi}"
if [ ! -d "$OPENPI_DIR" ]; then
  echo "OPENPI_DIR not found ($OPENPI_DIR). Run gpu-bootstrap first."; exit 1; fi

# Start π0‑FAST policy server (edit config/checkpoint as needed)
cd "$OPENPI_DIR"
uv run scripts/serve_policy.py policy:checkpoint \
  --policy.config=pi0_fast_droid \
  --policy.dir="$HOME/.cache/openpi/checkpoints/pi0_fast_droid" \
  --server.host="$HOST" --server.port="$PORT"