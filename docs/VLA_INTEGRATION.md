# VLA Model Integration Guide

## Overview
Integration of Vision-Language-Action models (π0-FAST/OpenVLA) with SO-101 robotic arms.

## Architecture
## Setup

### 1. Start VLA Server (on GPU machine)
```bash
make run-vla-server
export VLA_SERVER_URL="http://gpu-server:8000"
make run-vla-robot
## Configuration
Edit robot/configs/vla_config.yaml:
- model_type: pi0_fast or openvla
- action_chunk_size: Actions per inference
- observation_history: Past observations to include

## Performance
- Camera: 30 FPS
- Inference: 10 Hz
- Control: 200 Hz
- Action chunking bridges frequency gap

## Troubleshooting
- Check server: curl http://gpu-server:8000/health
- Monitor latency in logs
- Verify camera feeds
