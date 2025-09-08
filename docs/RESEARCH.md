# 🔬 RESEARCH.md — Evidence & Pointers

> **Active research into robotics platforms, VLA models, and integration strategies**

## Jetson Thor (deployment)
- NVIDIA page: 2070 FP4 TFLOPS, 128 GB, 40–130 W; **Jetson AGX Thor**; native **2× CAN** on carrier.  
- Dev‑kit carrier spec PDF (Aug 2025).  
- Coverage (TechRadar/WindowsCentral/Barron’s/ServeTheHome).

## Robot Platforms & Runtimes

### OpenMind OM1 (NEW - Dec 2024 Investigation)
- **Repository**: https://github.com/OpenMind/OM1
- **Architecture**: Modular AI runtime for multimodal agents across digital & physical robots
- **Key Features**:
  - Multi-platform: Humanoids, quadrupeds, educational robots, apps, websites
  - Multimodal I/O: Web data, social media, cameras, LIDAR → motion, navigation, conversation
  - Middleware support: ROS2, Zenoh, CycloneDDS
  - Hardware: Jetson AGX Orin, Mac Studio M2 Ultra, Mac Mini M4 Pro, RPi 5
  - Configuration: JSON5-based, plugin architecture
  - License: MIT (permissive)
- **Integration with Saorsa**:
  - Complementary: OM1 for high-level orchestration, Saorsa for real-time safety-critical control
  - Bridge via ROS2/gRPC for sensor sharing
  - Potential FFI integration for Rust↔Python interop
  - Challenges: Python GIL vs real-time guarantees
  - Opportunity: Use OM1's cloud AI with our local inference

## VLA / ARM models
- **OpenVLA** — open‑source VLA (7B), pretrained on 970k episodes (Open‑X Embodiment), code & checkpoints; **OpenVLA‑OFT** shows 25–50× faster inference and ↑success on LIBERO.  
- **MolmoAct (AI2, 2025)** — first fully open **Action Reasoning Model (ARM)** “thinks in 3D”, plans waypoints; blog & coverage.  
- Related: **Octo** (Stanford), **RDT‑1B** (Tsinghua), **Open X‑Embodiment** dataset.
 - **X2Robot depth‑aware manipulation (2025)** — see draft integration note: `docs/research/X2ROBOT_DEPTH_MANIPULATION.md` (source link currently unreachable from our environment).

## Stereo cameras (Jetson/macOS)
- **ZED SDK** (Jetson install guide).  
- **Luxonis OAK‑D / DepthAI** (USB; onboard NPU).  
- **Intel RealSense** — D455/D555; spin‑out from Intel secured funding (2025), ecosystem active.

## Voice — fully local
- **ASR**: faster‑whisper (CTranslate2, Metal/GPU), whisper.cpp (Core ML/ANE), Vosk (lightweight).  
- **TTS**: **Chatterbox** (Resemble AI, MIT, 0.5B, emotion control; trending 2025), **Kokoro‑82M** (Apache‑2.0), **Piper** (fast CPU, many voices).  
- **NVIDIA Riva** runs on Jetson for ASR/TTS (licensing note; optional).

## CAN on macOS
- **python‑can gs_usb backend** supports Windows/Linux/**Mac** for candleLight/CANable/CANtact via libusb.  
- **MacCAN** drivers: **Kvaser** and **PCAN** user‑space drivers for macOS (Apple Silicon supported), PCANBasic‑compatible API.  
- **Intrepid ValueCAN 4** — macOS drivers available; CAN FD; isolated.  
- **SLCAN** serial‑line option and libraries (libSLCAN, SerialCAN).

## Protocols & actuators
- **ODrive** CANSimple protocol docs; ROS 2 CAN bridge.  
- **T‑Motor AK** CAN protocol manuals; community control libs.  
- **CANopen (ros2_canopen)**; **Cyphal (UAVCAN v1)** for real‑time distributed nodes (pycyphal).

## Calibration & tags
- **AprilTag** (C, ROS 2, Rust bindings) for fiducials and pose.  
- **OpenCV** stereo calibration tutorial.

---

## Links

- Jetson Thor: NVIDIA page; dev‑kit PDF; ServeTheHome, TechRadar, Barron’s, WindowsCentral.  
- VLA/ARM: OpenVLA (GitHub/site/OFT), AI2 MolmoAct, Octo, RDT‑1B, Open X‑Embodiment.  
- Cameras: ZED SDK for Jetson; OAK‑D (DepthAI); RealSense news.  
- Voice: faster‑whisper; whisper.cpp Core ML; Vosk; NVIDIA Riva docs.  
- TTS: Chatterbox repo & demo; Kokoro (HF & GitHub); Piper.  
- CAN (macOS): python‑can gs_usb docs; MacCAN Kvaser/PCAN; Intrepid ValueCAN; SLCAN libs.  
- Actuators/Protocols: ODrive CANSimple; T‑Motor AK manuals; ros2_canopen; pycyphal.  
- Tags/Calib: AprilTag ROS/Rust; OpenCV calibration.

> See the chat message for inline citations and dates.

## Similar Projects Aligned with Local-First Robotics (2024 Research)

### Edge AI and Local Training Frameworks

#### **LeRobot by Hugging Face**
- **Focus**: Real-world robotics with local training capabilities
- **Features**: Train robots "in minutes on your laptop", imitation learning, reinforcement learning
- **Hardware**: Runs on consumer hardware, supports SO-101 and other platforms
- **Alignment**: Strong focus on accessibility and local deployment

#### **Isaac Lab (formerly ORBIT)**
- **Focus**: GPU-accelerated local training using NVIDIA Isaac Sim
- **Performance**: 100k FPS policy training with RSL-RL
- **Hardware**: Leverages local GPU parallelization
- **Robots**: 16+ models including manipulators, quadrupeds, humanoids

#### **Copper (Rust)**
- **Repository**: copper-project/copper-rs
- **Focus**: Memory-safe, deterministic runtime for production robots
- **Features**: Leverages Rust's ownership model for thread safety
- **Alignment**: Production-ready with formal safety guarantees

#### **OpenRR (Open Rust Robotics)**
- **Focus**: "World's first robotics platform made by Rust and for Rust"
- **Safety**: Formal specifications, no segmentation faults
- **Integration**: ROS2 bindings with safe_drive library

### Privacy-Preserving and Federated Learning

#### **Advanced Privacy-Preserving Federated Learning (APPFL)**
- **Developers**: Argonne National Lab, University of Illinois, ASU (2024)
- **Features**: 40% reduction in communication, 30% faster training
- **Privacy**: Robust against data reconstruction attacks
- **Deployment**: Works across healthcare, finance, robotics

#### **Multi-Agent Federated Reinforcement Learning (MARL-FL)**
- **Focus**: Human-robot collaboration without cloud dependencies
- **Application**: Smart manufacturing, Industry 4.0
- **Innovation**: Eliminates centralized data aggregation vulnerabilities

### TinyML and Neuromorphic Edge Deployment

#### **Neuromorphic Platforms**
- **BrainChip Akida**: Always-on inference on MCU-scale devices
- **Intel Loihi 2**: Spiking neural networks for ultra-low power
- **Applications**: Continuous monitoring without cloud connection

#### **Edge-Native Foundation Models**
- **Gemini Robotics On-Device**: Brings AI to local robotic devices
- **SenseCAP Watcher**: World's first physical LLM agent for space monitoring
- **TinyML Models**: Deploy to devices with MHz processors and mW power

### Emerging 2024-2025 Initiatives

#### **Data Flywheel Mechanisms**
- Focus on large-scale local data collection
- Continuous adaptation without cloud upload
- Lifelong learning enabling robots to evolve through interactions

#### **Vision-Language-Action Models for Edge**
- **GraspVLA**: Pre-trained on billion-scale synthetic action data
- **UniVLA**: Unified model for multiple robot types
- **Deployment**: Optimized for local inference on edge hardware

## Integration Architecture Patterns

### Hybrid Python-Rust Architecture
```
┌─────────────────────────────────────┐
│      High-Level Planning (Python)    │
│  OM1 Runtime / Cloud VLA Models      │
└──────────────┬──────────────────────┘
               │ ROS2/gRPC
┌──────────────┴──────────────────────┐
│    Mid-Level Coordination (Rust)     │
│  Saorsa Brain Daemon / Safety Guard  │
└──────────────┬──────────────────────┘
               │ CAN/Serial
┌──────────────┴──────────────────────┐
│      Real-Time Control (Rust)        │
│    Motor Control / Sensor Fusion     │
└─────────────────────────────────────┘
```

### Communication Strategies
- **Shared Memory IPC**: Same-machine Python↔Rust
- **ROS2 Topics**: Sensor data, high-level commands
- **gRPC**: Model inference requests
- **CAN Bus**: Real-time motor control

## Key Differentiators of Saorsa Robotics

### What Makes Saorsa Unique

1. **Rust-First Safety-Critical Design**
   - Unlike Python-based frameworks (OM1, LeRobot), we guarantee memory safety
   - Formal constraint verification in production
   - Zero panic guarantee in production code
   
2. **True Local-First Architecture**
   - Complete on-device training and inference
   - No cloud dependencies for core functionality
   - Privacy by design, not as an afterthought

3. **Integrated Safety System**
   - Expression-based constraint DSL
   - Real-time watchdog monitoring
   - Hardware E-stop integration

4. **Unified Rust Ecosystem**
   - Single language from high-level planning to real-time control
   - No Python GIL limitations
   - Deterministic timing guarantees

### Competitive Analysis

| Feature | Saorsa | OM1 | LeRobot | Isaac Lab | Copper |
|---------|--------|-----|---------|-----------|---------|
| Language | Rust | Python | Python | Python | Rust |
| Memory Safety | ✅ Guaranteed | ❌ | ❌ | ❌ | ✅ |
| Real-time | ✅ Hard RT | ❌ Soft RT | ❌ | ❌ | ✅ |
| Local Training | ✅ | Partial | ✅ | ✅ | N/A |
| Privacy-First | ✅ | ❌ | Partial | ❌ | N/A |
| Safety Constraints | ✅ Formal | ❌ | ❌ | ❌ | ✅ |
| Production Ready | ✅ | Beta | Research | Research | ✅ |

## Future Research Areas

### Near-Term (Q1 2025)
1. **Unified Robot Description Format**: Extend URDF with VLA metadata
2. **Local-Cloud Hybrid Inference**: Intelligent routing based on latency/complexity
3. **Cross-Platform Skill Transfer**: Abstract skills to capability representations

### Mid-Term (Q2-Q3 2025)
1. **Formal Verification of VLA Behaviors**: Symbolic abstraction of learned policies
2. **Distributed Multi-Robot Coordination**: Blockchain-inspired consensus
3. **Edge-Optimized Models**: 1B parameter models on Jetson Orin

### Long-Term (2026+)
1. **Self-Supervised Robot Learning**: Curiosity-driven exploration
2. **Neuromorphic Computing**: Intel Loihi, IBM TrueNorth integration
3. **Quantum-Enhanced Planning**: Quantum annealing for multi-robot coordination

## Collaboration Opportunities

### Potential Integrations

1. **LeRobot + Saorsa**
   - Use LeRobot's datasets and pre-trained models
   - Provide Rust runtime for safety-critical deployment
   - Bridge: ONNX export → Candle inference

2. **OM1 + Saorsa**
   - OM1 for high-level task planning
   - Saorsa for real-time execution
   - Bridge: ROS2 topics or gRPC

3. **APPFL + Saorsa**
   - Federated learning for multi-robot fleets
   - Privacy-preserving model updates
   - Local training with global knowledge sharing

4. **Isaac Lab + Saorsa**
   - Sim-to-real transfer pipeline
   - GPU-accelerated training → Rust deployment
   - Synthetic data generation for edge models

### Research Partnerships

- **Academic**: CMU RI, Stanford AI Lab, MIT CSAIL, UC Berkeley BAIR
- **Industry**: Hugging Face, Physical Intelligence, NVIDIA
- **Open Source**: RustRobotics community, ROS2 WG

## References & Resources

### Key Papers (2024-2025)
1. "Foundation Models in Robotics: Applications, Challenges, and the Future" (2025)
2. "Advanced Privacy-Preserving Federated Learning (APPFL)" (2024)
3. "From Tiny Machine Learning to Tiny Deep Learning: A Survey" (2024)
4. "Federated Learning for Privacy-Preserving AI in HRC" (2024)

### Critical Repositories
- https://github.com/huggingface/lerobot
- https://github.com/OpenMind/OM1
- https://github.com/copper-project/copper-rs
- https://github.com/openrr/openrr
- https://github.com/isaac-sim/IsaacLab
- https://github.com/RustRobotics

### Communities & Events
- tinyML Foundation Austin 2025
- CoRL 2024 Workshops on Lifelong Learning
- Rust Robotics Discord
- Edge AI Foundation

_Last updated: December 2024_
_Next comprehensive review: January 2025_
