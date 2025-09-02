# Saorsa Robotics Brain — Vision·Language·Action over CAN

> **Goal:** Plug a **camera** + **CAN bus** into our **Saorsa brain**, and control **any** CAN‑addressable robot (arms, quadrupeds, humanoids, AMRs) with stereo perception, local voice I/O, and a continually‑learning VLA/ARM policy. First dev target is **macOS (Apple Silicon)**; deployment target is **NVIDIA Jetson AGX Thor**.

---

## What you can build **today**

- **See**: Stereo depth via ZED‑2i / Luxonis OAK‑D / RealSense.
- **Understand**: Open‑source **VLA** policy (OpenVLA; MolmoAct ARM) parses vision + language into actions.
- **Act**: Cross‑platform CAN backends (Mac dev adapters; SocketCAN on Linux/Jetson Thor with native CAN).
- **Talk**: Fully local **ASR** (faster‑whisper / whisper.cpp / Vosk) + **TTS** (Chatterbox, Kokoro, Piper).
- **Learn**: Online fine‑tuning (OFT for VLA) + reward/constraint‑aware continual learning.
- **Safety**: Human override, e‑stop, hard constraints, and audit trails.

---

## Hardware bill of materials (development → deployment)

### A. macOS development (Apple Silicon MacBook Pro)

**CAN adapters (pick one):**
- **Intrepid ValueCAN 4‑1/4‑2 (USB, isolated, macOS driver)** — robust, CAN FD ready.  
- **Kvaser Leaf Light v2 / U100P (USB)** — use **MacCAN** Kvaser driver for macOS.  
- **PEAK PCAN‑USB / PCAN‑USB FD (USB)** — use **MacCAN** PCAN (PCBUSB) driver for macOS.  
- **Open adapters**: **CANable/CANtact** flashed with **candleLight** (gs_usb). Works on macOS via `python-can[gs-usb]` (libusb).  

**Stereo cameras (USB‑C):**
- **Stereolabs ZED‑2i** (SDK on Jetson/macOS via Docker).  
- **Luxonis OAK‑D** (DepthAI pipeline on CPU/NPU; simple USB).  
- **Intel RealSense D455/D555** (active IR stereo; new RealSense spin‑out continues support).  

**Audio I/O:** USB mic (or MacBook mic) + speakers/headphones.

**Misc:** USB‑C hubs, 120Ω CAN terminators, DE‑9 to JST/Phoenix breakouts, DB9 pinout adapters.

### B. Robot brain deployment (NVIDIA Jetson AGX Thor)

- **Jetson AGX Thor dev kit** (B100/Blackwell GPU; **2070 FP4 TFLOPS**, **128 GB LPDDR5X**, **2× CAN** on carrier).  
- **Stereo camera**: ZED‑2i / OAK‑D / RealSense mounted on the head/torso.  
- **CAN wiring**: direct to Thor carrier CAN or via industrial transceivers; 120Ω termination.  
- **Audio**: USB mic + powered speakers.  
- **E‑stop**: hardware NC loop that cuts actuator power; GPIO‑read for software interlock.

> Thor’s native CAN means **no USB adapter** in production; use SocketCAN (`can0`, `can1`) with the Rust `socketcan` crate.

---

## Repo layout (proposed)

```
saorsa-robotics/
├─ crates/
│  ├─ can-transport/         # Rust: unified CAN API (Linux SocketCAN, macOS gs_usb/slcan, optional Kvaser/PCAN FFI)
│  ├─ device-registry/       # YAML/DBC/EDS parsers; ODrive, T‑Motor AK, CANopen, Cyphal drivers
│  ├─ vision-stereo/         # camera capture (OpenCV/DepthAI/ZED), stereo depth, AprilTag utilities
│  ├─ voice-local/           # ASR (faster‑whisper/whisper.cpp/Vosk) + TTS (Chatterbox/Kokoro/Piper)
│  ├─ vla-policy/            # Python FFI for OpenVLA / MolmoAct; action heads; OFT fine‑tuning hook
│  ├─ safety-guard/          # hard limits, watchdogs, e‑stop, human override
│  └─ brain/                 # orchestrator; planners; voice→intent→VLA→actuation pipeline
├─ apps/
│  ├─ brain-daemon/          # runs on Mac/Jetson; gRPC/IPC control
│  └─ sr-cli/                # CLI to list devices, sniff CAN, run skills, calibrate cameras, etc.
├─ models/                   # model cards, conversion scripts, quantised weights pointers
├─ configs/                  # CAN maps, device descriptors, camera calib, safety rules
└─ docs/                     # README.md, SPEC.md, TASKS.md, RESEARCH.md
```

---

## Quickstart — macOS (Apple Silicon)

1) **CAN (open adapters via gs_usb)**  
```bash
brew install libusb python@3.11
python -m venv .venv && source .venv/bin/activate
pip install "python-can[gs-usb]" pyusb
python - <<'PY'
import can, usb
# auto-detect candleLight/CANable/CANtact
bus = can.interface.Bus(bustype="gs_usb", channel=0, bitrate=500000)
bus.send(can.Message(arbitration_id=0x123, data=b"\x01\x02\x03", is_extended_id=False))
print(bus.recv(1.0))
PY
```

2) **CAN (Kvaser/PCAN/Intrepid)**  
- Install vendor/macOS driver (MacCAN for Kvaser/PCAN; Intrepid’s macOS driver).  
- Configure `python-can` (e.g., `bustype="kvaser"` or `bustype="pcan"` or vendor SDK).

3) **Stereo camera**  
- **OAK‑D**: `pip install depthai` → run `python3 examples/color_mono.py`.  
- **ZED‑2i**: install ZED SDK; test `ZED Explorer`.  
- **RealSense**: `librealsense` + `pyrealsense2`.

4) **ASR/TTS (fully local)**  
```bash
pip install faster-whisper  # ASR
pip install kokoro==1.*     # light TTS (Apache-2.0)
# Chatterbox (higher quality; GPU recommended)
pip install torch torchaudio soundfile
pip install git+https://github.com/resemble-ai/chatterbox.git
```

5) **VLA policy**  
```bash
pip install -U openvla  # if published; otherwise clone github.com/openvla/openvla
```

---

## Action pipeline (voice → intent → VLA → CAN)

1. **Wake‑word** → **ASR** transcript.  
2. **Intent parser** (small local LLM or rules) → task graph (“raise left arm 15 cm”).  
3. **Perception**: stereo depth + object/pose; optional AprilTags for calibration.  
4. **VLA/ARM policy** (OpenVLA / MolmoAct) → **action chunk** (joint/EE deltas).  
5. **Device registry** maps action space → **CAN commands** (ODrive/T‑Motor/CANopen/Cyphal).  
6. **Safety guard** clamps/blocks; human override / e‑stop; full telemetry logging.  
7. **Continual learning** captures (obs, action, reward, overrides) → OFT/RL updates.

---

## Safety & overrides

- **Hard limits** per joint (pos/vel/torque), workspace fences, collision‑check on depth map.  
- **E‑stop** (HW) cuts actuator power; SW watches GPIO + CAN heartbeat.  
- **Interventions** become negative reward/constraints; policies re‑trained to avoid repeats.  

---

## Licensing

All code MIT/Apache‑2.0 where possible; model weights follow upstream licenses.

---

## References

See `docs/RESEARCH.md` for detailed links and notes.

_Last updated: 2025-09-01_