#!/usr/bin/env bash
set -euo pipefail

# This script assumes Ubuntu 22.04+ with sudo.
# 1) NVIDIA drivers + Docker + nvidia-container-toolkit
# (Follow your cloud's official docs; below is a minimal outline.)

if ! command -v docker >/dev/null 2>&1; then
  echo "[gpu] Installing Docker..."
  curl -fsSL https://get.docker.com | sh
  sudo usermod -aG docker $USER
fi

# NVIDIA container toolkit (requires drivers pre-installed)
if ! command -v nvidia-smi >/dev/null 2>&1; then
  echo "[gpu] WARNING: nvidia-smi not found. Install NVIDIA drivers for your VM first."
fi

# Python + uv (no system pollution)
curl -LsSf https://astral.sh/uv/install.sh | sh
export PATH="$HOME/.local/bin:$PATH"
uv venv
source .venv/bin/activate

# OpenPI and rewarder deps via uv/pip; we run OpenPI typically outside this repo.
# You can place OpenPI under ~/openpi or inside policy_server/docker builds.

mkdir -p "$HOME/openpi"
if [ ! -d "$HOME/openpi/.git" ]; then
  git clone --recurse-submodules https://github.com/Physical-Intelligence/openpi.git "$HOME/openpi"
fi

pushd "$HOME/openpi" >/dev/null
uv sync || uv pip install -e . || true
popd >/dev/null

echo "[gpu] bootstrap complete. Use 'make serve-pi' to start the policy server."