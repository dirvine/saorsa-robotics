# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview
Saorsa Robotics is a complete AI robotics platform with CAN bus control, computer vision, voice commands, and continual learning. The system consists of a Rust workspace with safety-critical components for robot control and Python services for AI/ML inference.

## Architecture

### System Components
- **Rust Core (Workspace)**: Safety-critical real-time control
  - `can-transport`: CAN bus communication (SocketCAN/USB-CAN)
  - `device-registry`: Robot device abstractions (ODrive, T-Motor, CANopen)
  - `vision-stereo`: Camera backends (DepthAI, RealSense, OpenCV)
  - `voice-local`: Local ASR/TTS (whisper, Chatterbox)
  - `vla-policy`: Vision-Language-Action policy interface
  - `safety-guard`: Hardware safety limits and E-stop
  - `continual-learning`: Online model improvement
  - `intent-parser`: Natural language command parsing
  
- **Applications**: 
  - `brain-daemon`: Main orchestrator daemon
  - `sr-cli`: Command-line interface
  - `kyutai-stt-app`: Speech-to-text hotkey app
  - Various demo apps for testing subsystems

- **Python Services**:
  - Policy server: OpenVLA/π0-FAST inference on GPU
  - VLM rewarder: Language-based reward generation
  - Robot client: LeRobot control with async action chunking
  - Camera calibration and demonstration collection tools

### Deployment Architecture
- **Mac Workstation**: Controls 1-4 SO-101 arms via USB, runs Rust control stack
- **GPU Server**: Runs policy models as HTTP services (π0-FAST/OpenVLA)
- **Network**: Action chunking handles up to 150ms latency

## Development Commands

### Build & Test (Rust)
```bash
# Full Rust workspace build
cargo build --workspace --all-targets

# Run all tests (must pass with zero failures)
cargo test --all

# Format code (required before commit)
cargo fmt --all

# Lint with clippy (zero warnings enforced)
cargo clippy --all-features -- -D clippy::panic -D clippy::unwrap_used -D clippy::expect_used

# Combined quality check
make rust-all  # Runs fmt, build, clippy
```

### Robot Control
```bash
# Bootstrap Mac environment (installs uv, LeRobot, PyTorch)
make mac-bootstrap

# Find and calibrate arms (after USB connection)
lerobot-find-port                                  # List all connected arms
lerobot-setup-motors --port /dev/tty.usbmodem1101  # Configure motors
lerobot-calibrate --port /dev/tty.usbmodem1101     # Calibrate position

# Run single arm
ARM_ID=arm01 make run-arm

# Run all 4 arms concurrently
make run-all-arms
```

### GPU Server Setup
```bash
# Bootstrap GPU server (Docker + CUDA)
make gpu-bootstrap

# Serve policy models
make serve-pi              # π0-FAST on port 8080
make serve-pi-arm01        # Per-arm servers (8081-8084)
make serve-rewarder        # VLM rewarder on port 18080
```

### Camera Operations
```bash
# Test camera connections
make test-camera

# Calibrate cameras (generates calib YAML)
make calibrate-camera        # Single camera
make calibrate-all-cameras   # All cameras

# Collect demonstrations
make collect-demos

# Install camera dependencies
make install-camera-deps
```

### Voice/STT
```bash
# Bootstrap Kyutai Moshi STT
make stt-moshi-bootstrap

# Serve Moshi STT server
make stt-moshi-serve

# Run STT hotkey app (F12 default)
make run-kyutai-stt-app
```

### Training & Learning
```bash
# Start RL training with HIL-SERL
make train-rl

# Hugging Face login for model access
make hf-login
```

### Testing & Validation
```bash
# Check Apple Silicon GPU (MPS) availability
python3 scripts/check_mps.py

# Python linting/formatting
ruff check .
black . --line-length 100
```

## Critical Safety & Quality Requirements

### Rust Code Standards (ZERO TOLERANCE)
- **NO `unwrap()` or `expect()` in production code** (OK in tests only)
- **NO `panic!()`, `todo!()`, `unimplemented!()`**
- **Zero clippy warnings** (`cargo clippy -- -D warnings`)
- **All unsafe code requires justification**
- **100% test pass rate required**

### Safety Systems
- Physical E-stop required for all robot operations
- Workspace limits enforced in `robot/safety/ee_limits.yaml`
- CAN heartbeat monitoring (20ms timeout)
- Human supervision mandatory during training

## Configuration

### Environment Variables (`.env`)
```bash
# Policy server (GPU)
POLICY_SERVER_HOST=127.0.0.1
POLICY_SERVER_PORT=8080

# Per-arm configuration
ARM01_POLICY_PORT=8081  # Per-arm policy ports
ARM01_CAM_INDEX=0       # Camera assignments

# Action chunking (latency compensation)
ACTIONS_PER_CHUNK=40        # Actions per inference
CHUNK_SIZE_THRESHOLD=0.6    # Queue refill threshold

# Logging
LOG_DIR=$HOME/.saorsa/logs
```

### Per-Arm Config Files
- `robot/configs/so101_armNN.yaml`: USB port, camera settings
- `robot/safety/ee_limits.yaml`: Shared workspace boundaries
- Device registry: `configs/devices/*.yaml` for robot definitions

## Key Implementation Patterns

### Action Chunking (Async Control)
Compensates for network latency with predictive action queuing:
1. Request 40 actions from policy server
2. Execute from queue while predicting next chunk
3. Refill at 60% threshold to prevent stutter
4. Optional RTC smooths chunk transitions

### CAN Bus Protocol Support
- **CANopen**: CiA-402 device profiles
- **CANSimple**: ODrive protocol
- **T-Motor**: AK series proprietary
- **Cyphal/UAVCAN**: Aerospace standard

### No-Demonstration Training
- VLM generates rewards from language goals
- On-robot RL with safety constraints
- HIL-SERL for sample efficiency
- Optional human demonstrations

## Common Workflows

### Adding New Robot Type
1. Create device descriptor in `configs/devices/robot_name.yaml`
2. Implement driver in `crates/device-registry/src/drivers/`
3. Add safety limits to `robot/safety/`
4. Test with `apps/safety-demo`

### Debugging Control Issues
```bash
# Monitor action queue health
tail -f ~/.saorsa/logs/arm01.log | grep queue_size

# Check CAN bus traffic
candump can0  # Linux
# or use sr-cli can monitor

# Measure network latency
python3 scripts/measure_latency.py

# Adjust chunking if stuttering
# Increase ACTIONS_PER_CHUNK or decrease CHUNK_SIZE_THRESHOLD
```

### Training New Task
1. Define task in `rl/configs/task_name.yaml`
2. Set language goal and reward function
3. Configure safety constraints
4. Run `make train-rl` with supervision

### Running Tests
```bash
# Run specific test
cargo test test_name

# Run tests for specific crate
cargo test -p can-transport

# Run with output for debugging
cargo test -- --nocapture

# Run single threaded (for hardware tests)
cargo test -- --test-threads=1
```

## Project Structure
```
.
├── crates/           # Rust workspace libraries
├── apps/            # Rust applications
├── robot/           # Python robot control (LeRobot)
├── policy_server/   # GPU inference services
├── rl/              # Reinforcement learning configs
├── stt/             # Speech-to-text services
├── scripts/         # Utility scripts
├── configs/         # Device and calibration configs
└── docs/            # Architecture documentation
```

## Important Files
- `Cargo.toml`: Rust workspace configuration
- `Makefile`: Primary build orchestration
- `.env.example`: Environment variable template
- `docs/SPEC.md`: Complete system specification
- `robot/configs/`: Per-arm and camera configurations
- `robot/safety/ee_limits.yaml`: Safety constraints