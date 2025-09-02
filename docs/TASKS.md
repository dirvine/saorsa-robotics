# TASKS.md — Implementation Plan

## Phase 0 — Repo prep (Week 0)
- [ ] Create `crates/` scaffolding and `apps/brain-daemon`, `apps/sr-cli`.
- [ ] CI: macOS + Linux build, `RUSTFLAGS='-D warnings'`, unit tests, fmt/clippy.
- [ ] Add `docs/` with this plan; pre‑commit hooks.

## Phase 1 — CAN foundations (Week 1–2)
- [ ] `can-transport` with traits: `Bus`, `Frame`, `Filter`.
- [ ] Backends: `socketcan` (Linux), `gs_usb` (mac via `python-can` bridge or libusb FFI), `slcan` serial.
- [ ] Vendor FFIs (feature‑gated): Kvaser (MacCAN), PCAN (PCBUSB), Intrepid (ICS).
- [ ] CLI: `sr can list`, `sr can sniff`, `sr can send --id 0x123 --data 01 02 03`.
- [ ] Golden tests with virtual loopback; log format `.srlog`.

## Phase 2 — Device registry (Week 2–3)
- [ ] YAML schema; loaders; hot‑reload.
- [ ] Drivers: ODrive (CANSimple), T‑Motor AK (AK protocol), CiA‑402, Cyphal minimal.
- [ ] Example descriptors for 1 arm + 1 leg joint set.
- [ ] Telemetry decode + Prometheus metrics.

## Phase 3 — Vision stereo (Week 3–4)
- [ ] Backends: OAK‑D (DepthAI), ZED (SDK), RealSense.
- [ ] Calibration tool: chessboard + AprilTag board; save YAML.
- [ ] Depth → point cloud; ROI crop; latency budget tests.

## Phase 4 — Voice local (Week 4)
- [x] ASR modules: Kyutai STT with HuggingFace API integration; streaming API.
- [x] TTS modules: Chatterbox (GPU), Kokoro/Piper fallback (planned).
- [x] Wake‑word detection ("Tektra"); audio I/O abstraction.

## Phase 5 — VLA policy (Week 5–6)
- [x] Python module wrapper for **OpenVLA**; action‑head adapters.
- [x] Mini skill library: reach/pick/place; EE delta in camera frame.
- [x] Mock policy implementation for development/testing.
- [ ] Optional ARM track: integrate **MolmoAct** when weights/api available.

## Phase 6 — Safety & orchestration (Week 6–7)
- [x] Safety constraints DSL (joint, EE, workspace); clamp + block.
- [x] Watchdogs (camera heartbeat; CAN heartbeat; e‑stop GPIO).
- [x] `brain-daemon` orchestration with voice processing; basic CLI; structured logs.

## Phase 7 — Continual learning (Week 7–8)
- [x] Data taps and buffer; reward/event model.
- [x] OFT fine‑tuning job (offline first); model registry + promotion flow.
- [x] Constraint learning from interventions.

## Phase 8 — Demos & docs (Week 8)
- [x] End-to-end voice: "raise arm 15 cm" → action → CAN.
- [ ] Benchmarks: control rate, ASR latency, TTS first-chunk, depth FPS.
- [ ] Update README with BOM and quickstarts; screencast/gif.

_Optional tracks:_ ROS 2 bridge; Isaac ROS AprilTag; Cyphal native media.

_Last updated: 2025-09-03_