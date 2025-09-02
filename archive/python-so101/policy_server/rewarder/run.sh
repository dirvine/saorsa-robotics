#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
python -m pip install -r requirements.txt
export REWARDER_HOST=${REWARDER_HOST:-0.0.0.0}
export REWARDER_PORT=${REWARDER_PORT:-18080}
uvicorn app:app --host "$REWARDER_HOST" --port "$REWARDER_PORT"