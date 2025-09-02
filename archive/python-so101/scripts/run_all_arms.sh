#!/usr/bin/env bash
set -euo pipefail

echo "Starting all 4 SO-101 arms..."

# Launch all arms in background
cd "$(dirname "$0")/../robot"
./run_robot_client.sh arm01 &
./run_robot_client.sh arm02 &  
./run_robot_client.sh arm03 &
./run_robot_client.sh arm04 &

echo "All arms launched. Press Ctrl+C to stop all processes."
wait  # Wait for all background processes