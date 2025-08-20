# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview
Saorsa Robotics provides a production-ready scaffold for training Hugging Face SO-101 robotic arms using Vision-Language-Action (VLA) policies without demonstrations. The system uses a single MacBook Pro to control up to 4 arms, with remote NVIDIA GPU servers running policy models (π0-FAST/OpenVLA).

## Key Commands

### Mac Setup & Development
```bash
# Bootstrap Mac environment (installs uv, LeRobot with Feetech/async extras, PyTorch)
make mac-bootstrap

# Run individual arm (ARM_ID: arm01, arm02, arm03, arm04)
ARM_ID=arm01 make run-arm

# Run all 4 arms concurrently from single Mac
make run-all-arms

# Calibrate arms (after USB connection)
lerobot-find-port                              # Find all connected arms
lerobot-setup-motors --port /dev/tty.usbmodem1101  # Setup each arm
lerobot-calibrate --port /dev/tty.usbmodem1101     # Calibrate each arm
```

### GPU Server Operations
```bash
# Bootstrap GPU server (installs Docker, OpenPI)
make gpu-bootstrap

# Serve policy models
make serve-pi                 # Default π0-FAST on port 8080
make serve-pi-arm01           # Arm-specific on port 8081
make serve-rewarder          # VLM rewarder on port 18080
```

### Training & Evaluation
```bash
# Start RL training
make train-rl

# Hugging Face integration
make hf-login
```

### Testing & Validation
```bash
# Check Apple Silicon GPU availability
python scripts/check_mps.py

# Lint/format (when available)
ruff check .
black .
```

## Architecture & Key Design Patterns

### Distributed System Architecture
- **Single Mac Workstation**: Controls all 4 SO-101 arms via USB hub, runs LeRobot clients with async inference
- **Remote GPU Server**: Runs policy models (π0-FAST/OpenVLA) as HTTP services
- **VLM Rewarder**: Optional service for language-based reward generation in no-demo training

### Async Control Flow
The system uses action chunking with async inference to maintain smooth control despite network latency:
1. Robot client requests action chunks from policy server (40 actions default)
2. Actions execute from queue while next chunk is predicted
3. Queue refills at threshold (60% default) to prevent underflow
4. Real-Time Chunking (RTC) optionally smooths transitions between chunks

### Per-Arm Configuration
Each arm has independent configuration but shares safety limits:
- `robot/configs/so101_armNN.yaml`: USB port, camera settings
- `robot/safety/ee_limits.yaml`: Shared workspace boundaries
- Environment vars: `ARMNN_POLICY_PORT`, `ARMNN_CAM_INDEX`

### No-Demonstration Training
Uses VLM-based rewards instead of human demonstrations:
- Language goals specify tasks
- VLM scores frames against goals
- On-robot RL with safety interventions
- HIL-SERL integration for sample efficiency

## Critical Implementation Details

### USB Port Management
- All 4 arms connect to single Mac via powered USB hub
- Ports typically: `/dev/tty.usbmodem1101-1104`
- Each arm must be calibrated individually after connection

### Network Latency Handling
- Target latency: <50ms LAN, <150ms acceptable
- Action chunking compensates for latency
- Adjust `ACTIONS_PER_CHUNK` and `CHUNK_SIZE_THRESHOLD` based on network

### Safety Constraints
- Physical E-stop mandatory for all arms
- Cartesian workspace limits enforced in `ee_limits.yaml`
- Joint angle limits per arm
- Human supervision required during training

### Environment Variables
Key variables from `.env`:
- `POLICY_SERVER_HOST/PORT`: GPU server endpoint
- `ACTIONS_PER_CHUNK`: Actions per inference (default 40)
- `CHUNK_SIZE_THRESHOLD`: Queue refill threshold (default 0.6)
- `ARMNN_POLICY_PORT`: Per-arm policy server ports (8081-8084)

## Common Workflows

### Adding New Task
1. Create task config in `rl/configs/task_name.yaml`
2. Define goal text and rewarder settings
3. Configure actors (which arms to use)
4. Run with `make train-rl`

### Debugging Control Issues
1. Check queue health: `tail -f ~/.saorsa/logs/armNN.log | grep queue_size`
2. Monitor latency: `python scripts/measure_latency.py`
3. Adjust chunking parameters if stuttering occurs
4. Enable RTC if seeing pauses at chunk boundaries

### Scaling to Multiple Arms
1. Connect all arms via powered USB hub
2. Run `lerobot-find-port` to identify ports
3. Update each `robot/configs/so101_armNN.yaml` with correct port
4. Launch with `make run-all-arms` or individual `ARM_ID=armNN make run-arm`