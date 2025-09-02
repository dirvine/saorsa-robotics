SHELL := /bin/bash

.PHONY: mac-bootstrap gpu-bootstrap serve-pi serve-rewarder run-arm run-all-arms serve-pi-arm01 serve-pi-arm02 serve-pi-arm03 serve-pi-arm04 train-rl hf-login
.PHONY: rust-build rust-fmt rust-clippy rust-all
.
.PHONY: stt-moshi-bootstrap stt-moshi-serve
stt-moshi-bootstrap:
	@echo "[stt] Bootstrapping Kyutai Moshi"
	./stt/bootstrap_moshi.sh

stt-moshi-serve:
	@echo "[stt] Serving Kyutai Moshi"
	./stt/serve_moshi.sh

.PHONY: run-kyutai-stt-app
run-kyutai-stt-app:
	@echo "[voice] Running Kyutai STT hotkey app"
	cargo run -p kyutai-stt-app -- --config config.json --hotkey F12 --duration 5

mac-bootstrap:
	@echo "[mac] Installing uv, lerobot + extras, and basics"
	./scripts/bootstrap_mac.sh

gpu-bootstrap:
	@echo "[gpu] Installing CUDA/Docker + pulling OpenPI (requires sudo)"
	./scripts/bootstrap_gpu.sh

hf-login:
	./scripts/hf_login.sh

serve-pi:
	cd policy_server && ./serve_pi0_fast.sh

serve-rewarder:
	cd policy_server/rewarder && ./run.sh

run-arm:
	@if [ -z "$$ARM_ID" ]; then echo "Set ARM_ID=arm01..arm04"; exit 1; fi
	cd robot && ./run_robot_client.sh $$ARM_ID

run-all-arms:
	@echo "[mac] Starting all 4 SO-101 arms"
	./scripts/run_all_arms.sh

serve-pi-arm01:
	cd policy_server && ./serve_pi0_fast.sh 8081

serve-pi-arm02:
	cd policy_server && ./serve_pi0_fast.sh 8082

serve-pi-arm03:
	cd policy_server && ./serve_pi0_fast.sh 8083

serve-pi-arm04:
	cd policy_server && ./serve_pi0_fast.sh 8084

train-rl:
	cd rl && ./run_hilserl.sh
# Camera commands
calibrate-camera:
	@echo "Calibrating camera..."
	python scripts/calibrate_cameras.py --camera 0

calibrate-all-cameras:
	@echo "Calibrating all cameras..."
	python scripts/calibrate_cameras.py --camera 0 --output robot/configs/cam0_calib.yaml
	python scripts/calibrate_cameras.py --camera 1 --output robot/configs/cam1_calib.yaml

collect-demos:
	@echo "Starting demonstration collection..."
	python scripts/collect_demonstrations.py

test-camera:
	@echo "Testing camera connection..."
	python -c "import cv2; cap = cv2.VideoCapture(0); print('Camera 0:', 'OK' if cap.isOpened() else 'NOT FOUND'); cap.release()"
	python -c "import cv2; cap = cv2.VideoCapture(1); print('Camera 1:', 'OK' if cap.isOpened() else 'NOT FOUND'); cap.release()"

install-camera-deps:
	@echo "Installing camera dependencies..."
	pip install opencv-python opencv-contrib-python pyrealsense2 imageio imageio-ffmpeg

.PHONY: calibrate-camera calibrate-all-cameras collect-demos test-camera install-camera-deps

# Rust workspace helpers
rust-build:
	cargo build --workspace --all-targets

rust-fmt:
	cargo fmt --all

rust-clippy:
	cargo clippy --all-features -- -D clippy::panic -D clippy::unwrap_used -D clippy::expect_used

rust-all: rust-fmt rust-build rust-clippy
