# SPEC.md — Saorsa Robotics Brain

## 1. Scope

A modular, open-source stack that takes **vision + language** and outputs **safe CAN actuation** across heterogeneous robots. Dev on **macOS** with USB‑CAN; deploy on **Jetson AGX Thor** using native CAN.

## 2. System architecture

```
+-----------------------+     +-------------------+     +-------------------+
| Voice I/O (local)     | --> | Intent Layer      | --> | VLA/ARM Policy    |
|  ASR (fwhisper/ccpp)  |     |  parser + planner |     |  OpenVLA/MolmoAct |
|  TTS (Chatterbox/.. ) |     +---------+---------+     +---------+---------+
+-----------+-----------+               |                         |
            |                           v                         v
            |                   +---------------+        +-------------------+
            |                   | Perception    |        | Safety Guard      |
            |                   |  Stereo, tags |        | limits/override   |
            |                   +-------+-------+        +---------+---------+
            |                           |                         |
            v                           v                         v
+-----------+-----------+     +---------+---------+     +---------+---------+
| Brain Orchestrator    | --> | Device Registry  | --> | CAN Transport      |
|  (Rust)               |     |  (ODrive/TMotor/ |     |  Linux: SocketCAN  |
|  gRPC/IPC, logging    |     |   CANopen/Cyphal)|     |  macOS: gs_usb,    |
+-----------------------+     +------------------+     |  slcan, vendor FFI  |
                                                      +---------------------+
```

## 3. Core crates (Rust)

- **can-transport**  
  - `SocketCAN` (Linux/Jetson).  
  - `gs-usb`/`slcan` (macOS via libusb/serial).  
  - Optional FFI shims for Kvaser (MacCAN), PCAN (PCBUSB), Intrepid.

- **device-registry**  
  - Parsers: CAN DBC, CANopen EDS; Cyphal types.  
  - Drivers: ODrive (CANSimple), T‑Motor AK series, generic CiA‑402, Cyphal/UAVCAN.  
  - Device descriptors (YAML) → **joint abstractions** (position/velocity/torque).

- **vision-stereo**  
  - Backends: DepthAI (OAK‑D), ZED SDK, RealSense, OpenCV.  
  - Utilities: stereo rectification & depth, AprilTag pose, camera calibration.

- **voice-local**  
  - ASR: faster‑whisper / whisper.cpp / Vosk (configurable).  
  - TTS: Chatterbox (GPU), Kokoro/Piper (CPU‑friendly).

- **vla-policy** (Python module + Rust FFI)  
  - OpenVLA / OpenVLA‑OFT inference; MolmoAct (ARM) when available.  
  - Action‑head adapters (EE delta / joint targets / gripper).

- **safety-guard**  
  - Constraint engine (joint/EE/workspace).  
  - Watchdogs (CAN heartbeat, camera timeout).  
  - Human override + E‑stop link.  
  - Audit log (events, blocked actions, overrides).

- **brain**  
  - Orchestration state machine; skill runner; teleop; logging.  
  - gRPC/IPC service; CLI integration.

## 4. Device Description (YAML)

```yaml
id: t_motor_ak80_9
bus: can0
protocol: tmotor_ak
node_id: 0x01
joints:
  - name: hip_yaw
    limits: {pos_deg: [-60,60], vel_dps: 300, torque_nm: 25}
    map:
      mode: torque
      scale: {k_t: 0.12}
      frames:
        - id: 0x140
          fmt: tmotor_cmd
telemetry:
  - id: 0x240
    fmt: tmotor_state
heartbeat: {id: 0x700, period_ms: 20}
```

## 5. Calibration

- **Stereo**: chessboard stereo calibrate; store `configs/calib/*.yaml`.  
- **Tag frame**: AprilTag board to align robot base ↔ camera.  
- **Joint zeroing**: per‑device homing or manual set + safety check.

## 6. Action mapping

- VLA output → **robot‑agnostic action** (EE delta / task‑space).  
- Device registry resolves to per‑joint commands; rate‑limit; blend with feedback.  
- Support discrete **skills** (reach, grasp, place, walk step) as graph nodes.

## 7. Continual learning

- **Data taps**: {(obs frames), action, reward, safety_flags}.  
- **OFT** fine‑tuning hook (parallel decoding, action chunking).  
- **Constraint learning**: intervention → negative reward and/or CPO‑style constraint.  
- **Versioning**: model registry + rollout gating (shadow tests before promotion).

## 8. Safety

- **Compile‑time**: forbid `unwrap()/panic!` in daemon; error budgets.  
- **Run‑time**: bounded outputs; EWMA jerk clamps; stale sensor guards.  
- **Human factors**: audible confirmations, voice “are you sure?” before risky actions.

## 9. Telemetry & DevEx

- Binary log (zstd) + JSON summary; replay into sim.  
- `sr-cli` helpers: `sr can sniff`, `sr can map`, `sr vision calib`, `sr run skill`.

## 10. Performance targets

- Control loop: 100–200 Hz.  
- Perception: 30–60 FPS stereo.  
- ASR:<200 ms streaming; TTS first‑chunk <500 ms (Chatterbox GPU).

## 11. Compatibility

- macOS (Apple Silicon) for dev; Linux/Jetson for deploy.  
- ROS 2 bridge optional (ros2_canopen, Isaac ROS AprilTag).

_Last updated: 2025-09-01_