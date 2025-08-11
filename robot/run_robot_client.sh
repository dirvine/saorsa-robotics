#!/usr/bin/env bash
set -euo pipefail
ARM_ID="${1:-}"
if [[ -z "$ARM_ID" ]]; then
  echo "Usage: $0 <arm01|arm02|arm03|arm04>"; exit 1; fi

# Load env
if [ -f ../.env ]; then set -a; source ../.env; set +a; fi
CFG="configs/so101_${ARM_ID}.yaml"
if [[ ! -f "$CFG" ]]; then echo "Missing $CFG"; exit 1; fi

mkdir -p "${LOG_DIR:-$HOME/.saorsa/logs}"

# Extract basic fields from YAML using python (avoid extra deps)
PORT=$(python - <<PY
import sys, yaml
print(yaml.safe_load(open("$CFG"))['robot']['port'])
PY
)

CAMCFG=$(python - <<PY
import yaml, json
cfg=yaml.safe_load(open("$CFG"))
print(json.dumps(cfg['robot']['cameras']))
PY
)

# Per-arm environment overrides
ARM_UPPER=$(echo "$ARM_ID" | tr '[:lower:]' '[:upper:]')
POLICY_PORT_VAR="${ARM_UPPER}_POLICY_PORT"
CAM_INDEX_VAR="${ARM_UPPER}_CAM_INDEX"
ARM_POLICY_PORT="${!POLICY_PORT_VAR:-$POLICY_SERVER_PORT}"
ARM_CAM_INDEX="${!CAM_INDEX_VAR:-0}"

APC=${ACTIONS_PER_CHUNK:-40}
CST=${CHUNK_SIZE_THRESHOLD:-0.6}

# Launch LeRobot RobotClient toward remote PolicyServer (OpenPI/OpenVLA)
python -m lerobot.scripts.server.robot_client \
  --server_address="${POLICY_SERVER_HOST:-127.0.0.1}:${ARM_POLICY_PORT:-8080}" \
  --robot.type=so101_follower \
  --robot.port="$PORT" \
  --robot.id="$ARM_ID" \
  --robot.cameras="${CAMCFG}" \
  --policy_type=pi0_fast \
  --pretrained_name_or_path=openpi/pi0_fast_droid \
  --policy_device=cuda \
  --actions_per_chunk=$APC \
  --chunk_size_threshold=$CST | tee -a "${LOG_DIR:-$HOME/.saorsa/logs}/$ARM_ID.log"