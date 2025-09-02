# SO-101 Python Implementation (Archived)

> **Note**: This is the original Python implementation for SO-101 robotic arms. For the current production Rust implementation, see the [main README](../../README.md).

## Overview

This archive contains the original Python-based control system for Hugging Face SO-101 robotic arms. This implementation provided the foundation for understanding VLA policies and robot control before the migration to the production Rust system.

## Original Features

### 🤖 Multi-Arm Control
- Single Mac controlling up to 4 SO-101 arms via USB hub
- Per-arm configuration with independent policies
- Async action chunking for low-latency control

### 🧠 VLA Policy Integration
- OpenVLA and π0-FAST model support
- Remote GPU server inference
- Action chunking to handle network latency
- Real-Time Chunking (RTC) for smooth transitions

### 🎯 No-Demonstration Training
- VLM-based reward generation
- Language goal specification
- On-robot reinforcement learning
- HIL-SERL integration

### 📷 Camera Integration
- Dual camera support per arm
- Camera quality improvements
- Calibration utilities

## Directory Structure

```
python-so101/
├── robot/               # Core robot control code
│   ├── configs/        # Per-arm YAML configurations
│   ├── safety/         # Workspace and joint limits
│   └── camera_manager.py
├── scripts/            # Utility scripts
│   ├── calibrate_cameras.py
│   ├── collect_demonstrations.py
│   └── check_mps.py
├── policy_server/      # GPU server for policies
│   └── openvla_policy.py
└── stt/               # Speech-to-text experiments
    ├── bootstrap_moshi.sh
    └── serve_moshi.sh
```

## Historical Usage

### Mac Setup
```bash
# Install LeRobot with async support
pip install lerobot[feetech,async]

# Calibrate arms
lerobot-find-port
lerobot-setup-motors --port /dev/tty.usbmodem1101
lerobot-calibrate --port /dev/tty.usbmodem1101
```

### Running Arms
```bash
# Single arm
ARM_ID=arm01 python robot/main.py

# All arms
python robot/run_all.py
```

### GPU Server
```bash
# Serve OpenVLA policy
python policy_server/openvla_policy.py --port 8080
```

## Migration Notes

This Python implementation served as a rapid prototyping platform and helped establish:

1. **Architecture patterns** - Multi-arm control, async inference, action chunking
2. **Safety requirements** - Workspace limits, joint constraints, e-stop integration
3. **Network topology** - Mac client + GPU server design
4. **VLA integration** - OpenVLA/π0-FAST model serving patterns

These learnings directly informed the production Rust implementation, which provides:
- Memory safety and performance
- Comprehensive type safety
- Zero-copy data paths
- Formal safety verification
- Production-grade error handling

## Legacy Configuration

Key environment variables from original `.env`:
```bash
POLICY_SERVER_HOST=192.168.1.100
POLICY_SERVER_PORT=8080
ACTIONS_PER_CHUNK=40
CHUNK_SIZE_THRESHOLD=0.6
USE_RTC=true
```

## Deprecation Notice

This Python implementation is archived for reference and is no longer actively maintained. All new development happens in the Rust implementation. For production deployments, please use the main Rust-based system.

## Original Contributors

The Python implementation was developed as part of the Saorsa Robotics initiative to democratize robotic learning with local, privacy-preserving AI models.

---

*For the current production system, see the [main repository](https://github.com/dirvine/saorsa-robotics)*