SHELL := /bin/bash

.PHONY: mac-bootstrap gpu-bootstrap serve-pi serve-rewarder run-arm run-all-arms serve-pi-arm01 serve-pi-arm02 serve-pi-arm03 serve-pi-arm04 train-rl hf-login

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