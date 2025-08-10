#!/usr/bin/env bash
set -euo pipefail
# Skeleton to illustrate where you'd launch an RL loop that queries the rewarder
# and drives actions via the remote policy server (or a smaller local policy).

if [ -f ../.env ]; then set -a; source ../.env; set +a; fi

export REWARDER_URL="${REWARDER_URL:-http://127.0.0.1:18080}"
export POLICY_SERVER_URL="${POLICY_SERVER_URL:-http://127.0.0.1:8080}"

CONFIG="configs/pick_place_red_to_blue.yaml"
echo "[rl] starting training with $CONFIG"

# Placeholder: call into your RL trainer (HIL-SERL style) here; for now, print.
python - <<PY
import os, yaml
cfg=yaml.safe_load(open("$CONFIG"))
print("Loaded RL config:", cfg)
print("Rewarder:", os.environ.get("REWARDER_URL"))
print("Policy server:", os.environ.get("POLICY_SERVER_URL"))
print("TODO: integrate with your RL loop using these endpoints.")
PY