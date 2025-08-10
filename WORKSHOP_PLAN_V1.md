# Saorsa Labs — SO‑101 VLA Training Plan (Workshop Edition)

**Version:** 0.1 (living document)
**Owners:** Saorsa Labs Robotics Team
**Scope:** Real‑robot training of four Hugging Face **SO‑101** arms without imitation datasets, using **camera+instruction** goals, **VLM‑based rewards**, **on‑robot RL**, and **remote VLA policies** (π/OpenVLA). Macs handle I/O; an NVIDIA Linux host serves the heavy models.

---

## 0) Objectives & non‑goals

### Objectives (what success looks like)

* A reproducible pipeline to control **4× SO‑101** with **async action chunking** and **(optional) RTC** over a LAN/WAN.
* At least **3 tabletop tasks** trained **without demonstrations** (language‑specified goals only) reaching **≥80% success** over 20 trials each.
* Automated evaluation, logging, and versioned artefacts (datasets, configs, models) in a single repo layout.

### Non‑goals (for now)

* Full multi‑robot coordination policies (shared autonomy).
* Large multi‑GPU pretraining of VLAs.
* Perfect sim‑to‑real: sim is optional and used only to shorten real training.

---

## 1) System overview

**Macs (near arms)** run LeRobot drivers + cameras + safety + async client.
**GPU Server** (Linux/NVIDIA) hosts a policy server (π/OpenVLA) and optional VLM rewarder.
**RL loop** queries rewarder and updates policy (on server) while Macs execute action chunks.

```
+-----------------+     WebSocket/HTTP      +-----------------------+
|  Mac (arm01)    | <---------------------> |  GPU Policy Server    |
|  LeRobot client |                          |  π0‑FAST / OpenVLA    |
|  Cameras, safety|                          |  + RTC (optional)     |
+-----------------+                          +-----------+-----------+
       ...                                                 |
+-----------------+                                       |
|  Mac (arm04)    |                                       v
+-----------------+                          +-----------------------+
                                             |  VLM Rewarder (HTTP) |
                                             |  (e.g., Qwen‑VL)     |
                                             +-----------------------+
```

---

## 2) Environments & prerequisites

### 2.1 macOS workstations (one per arm)

* Apple Silicon, macOS 14+, Python 3.10+, stable power + **physical E‑stop**.
* **uv** for Python envs, **ffmpeg**, USB to the SO‑101 (powered hubs recommended).
* Cameras: UVC webcams (preferred) or iPhone Continuity Camera. Keep lighting stable.
* Torch on macOS uses **MPS**; heavy models run remotely.

### 2.2 NVIDIA Linux GPU host (cloud or on‑prem)

* Ubuntu 22.04+, CUDA‑capable GPU (L4/L40S/4090/A100/H100).
* NVIDIA drivers, Docker, and (optionally) nvidia‑container‑toolkit.
* Ports open to lab IPs: **policy** (default `8080`), **rewarder** (default `18080`).
* Disk for datasets/checkpoints (suggest 0.5–1 TB to start).

### 2.3 Repository layout (summary)

See **saorsa‑robotics/** scaffold: `robot/` (Mac), `policy_server/` (GPU), `rl/`, `ops/`.
Environment variables live in `.env` (copy from `.env.example`).

---

## 3) Milestones & deliverables

### M0 — Lab prep (Day 0–1)

* **Deliverables:** labelled arms (`arm01..arm04`), camera mounts, lighting fixed, E‑stop tested.
* Bench layout ensures EE motion is contained; define **EE/joint limits** file.

### M1 — SO‑101 bring‑up on macOS (Day 1)

* **Deliverables:** each arm calibrated, jogging works, camera feed verified.
* Run `lerobot-find-port`, `lerobot-setup-motors`, `lerobot-calibrate` for each arm; fill `robot/configs/so101_armNN.yaml`.

### M2 — Remote policy server online (Day 1–2)

* **Deliverables:** π0‑FAST or OpenVLA policy reachable at `POLICY_SERVER_URL`.
* Start server (OpenPI
  `serve_policy.py`) and smoke test with sample obs.

### M3 — Async closed‑loop control (Day 2)

* **Deliverables:** arm follows action chunks without stutter at 15–20 Hz.
* Run `robot/run_robot_client.sh arm01`; tune `ACTIONS_PER_CHUNK` and `CHUNK_SIZE_THRESHOLD`.

### M4 — RTC enabled if needed (Day 2–3)

* **Deliverables:** pauses at chunk boundaries eliminated; queue never under‑runs.

### M5 — No‑demo training with VLM rewards (Week 1)

* **Deliverables:** one language‑goal task achieves **≥70%** success\@20; logs uploaded.

### M6 — Scale to 4 actors + 3 tasks (Weeks 2–3)

* **Deliverables:** 3 tasks at **≥80%** success\@20; weekly evaluation dashboard.

---

## 4) Standard Operating Procedures (SOPs)

### SOP‑A: Arm bring‑up & calibration (per arm)

1. Connect SO‑101 via powered USB hub.
2. `make mac-bootstrap` on the Mac; confirm Torch/MPS availability via `scripts/check_mps.py`.
3. Identify port with `lerobot-find-port`.
4. Setup motors & calibration; assign `armID` (arm01..arm04).
5. Edit `robot/configs/so101_armID.yaml` with `port`, camera, and safety file.
6. Jog EE within **cartesian\_box**; confirm E‑stop.

### SOP‑B: Start policy server (GPU host)

1. `make gpu-bootstrap` (installs uv, clones OpenPI).
2. `make serve-pi` (π0‑FAST by default).
3. Verify service answers on `http://HOST:8080/health` (route may differ by server script).

### SOP‑C: Async client (Mac)

1. Set `.env` with `POLICY_SERVER_HOST/PORT`.
2. `ARM_ID=arm01 make run-arm` to connect and stream.
3. Inspect queue size logs; adjust `ACTIONS_PER_CHUNK` (20–60) and `CHUNK_SIZE_THRESHOLD` (0.5–0.7).
4. If queue underflows → enable **RTC** on server.

### SOP‑D: VLM rewarder

1. Start stub: `make serve-rewarder` (HTTP `/score`).
2. Replace adapter with a real VLM (e.g., Qwen‑VL) when ready.
3. Define prompt/template and post‑process to **\[0,1]** scalar; clamp and smooth.

### SOP‑E: On‑robot RL loop

1. Pick task goal text (e.g., *“Put the red cube in the blue bowl.”*).
2. Configure `rl/configs/*.yaml` (actors, control rate, rewarder URL).
3. Start run: `make train-rl` (placeholder script → your trainer).
4. Safety: human take‑over mapped to keyboard/gamepad; stop on error.
5. Log metrics/videos; push artefacts to HF Hub if desired.

### SOP‑F: Evaluation & A/Bs

1. Create eval scripts for **success\@K**, time‑to‑success, gentle‑failure rate.
2. A/B compare **VLM reward vs. heuristic** on same tasks.
3. Keep a 50‑clip validation set with human labels for rewarder agreement checks.

---

## 5) Model cookbook (controller choices)

| Model            | Size/VRAM (guide)        | Strengths                                   | When to use                       |
| ---------------- | ------------------------ | ------------------------------------------- | --------------------------------- |
| **π0‑FAST**      | >8 GB inf; >22.5 GB LoRA | Fast chunking, good for remote control; RTC | Default remote controller         |
| **OpenVLA‑7B**   | \~16 GB inf (fp16)       | Clean open baseline; PEFT‑friendly          | Fine‑tune on your tasks           |
| **SmolVLA/Tiny** | <8–12 GB                 | Compact, lower latency                      | Edge tests / constrained hardware |

**Heuristic:** start π0‑FAST for inference; if customisation required, PEFT fine‑tune OpenVLA‑7B.

---

## 6) Rewarder cookbook (no‑demo path)

1. **Noop/Heuristic** (sanity): colour/pose masks → scalar reward.
2. **VLM zero‑shot**: frame + goal → scalar; begin with binary end‑of‑episode.
3. **VLM + shaping**: add dense hints (object pose, containment).
4. **Preferences**: batch clip pairs and use preference‑based optimisation (DPO‑style).
5. **Blend**: weighted combo of VLM score + tiny in‑domain classifier to denoise.

**Calibration tips**

* Fix camera extrinsics; stabilise lighting; crop ROI around workspace.
* Use 50–100 in‑lab frames to tune prompt and thresholds.

---

## 7) Async/RTC tuning guide

* **Control rate:** 15–20 Hz initially; raise cautiously after stability.
* **Chunks:** start **40 actions/chunk**; shorter = more reactive, longer = jitter‑resistant.
* **Queue threshold:** 0.6 to begin; ensure the client never runs dry.
* **RTC on:** if you observe pauses at chunk boundaries, enable; no retraining required.
* **Monitoring:** plot queue size over time; correlate with video.

---

## 8) Networking & security

* Prefer VPN/allow‑list; open only `POLICY_SERVER_PORT` and `REWARDER_PORT`.
* Use SSH tunnels for ad‑hoc tests.
* Keep robot Macs on isolated VLAN; policy server in a secure subnet.
* Log IPs and auth tokens (if enabled) to `~/.saorsa/logs`.

---

## 9) Data management & versioning

* **Naming:** `YYYYMMDD_task_armID_runID/` for episodes; include goal text in metadata.
* **Storage:** raw videos locally on the Mac; nightly rsync to the GPU host.
* **Hub:** `make hf-login` and push curated datasets/models; never push secrets.
* **Repro:** commit YAML configs and pinned package versions; tag milestones.

---

## 10) Quality gates & KPIs

* **G0:** All arms calibrated; safety limits verified.
* **G1:** Async closed‑loop stable @ ≥15 Hz, queue underflow <1/min.
* **G2:** First task success **≥70%** over 20 trials.
* **G3:** Three tasks **≥80%**; rewarder‑human agreement **≥90%** on 50‑clip set.
* **Latency KPI:** end‑to‑end observe‑to‑action ≤ 120 ms (median) on LAN.

---

## 11) Risk register (selected)

| Risk                                        | Impact | Likelihood | Mitigation                                                                                |
| ------------------------------------------- | ------ | ---------- | ----------------------------------------------------------------------------------------- |
| VLM reward brittleness (lighting/occlusion) | Medium | High       | ROI crops; fixed lighting; blend with small classifier; periodic human preference batches |
| WAN latency spikes                          | Medium | Medium     | Async + RTC; reduce FPS; deploy policy server closer to lab                               |
| macOS camera/driver quirks                  | Medium | Medium     | Prefer UVC; limit USB chain length; keep RealSense optional                               |
| GPU VRAM shortfall for fine‑tunes           | Medium | Medium     | Start with inference/LoRA; schedule bigger jobs on L40S/A100                              |
| Safety breach (EE leaves workspace)         | High   | Low        | E‑stop, joint clamps, cartesian box limits; human supervision                             |

---

## 12) Roles & RACI

* **Technician:** hardware bring‑up, wiring, E‑stop checks (R).
* **Engineer:** Mac clients, async tuning, SOP implementation (R).
* **MLOps:** GPU host, policy server, rewarder deployment (R).
* **Research:** rewarder design, RL training, evaluation (R).
* **Lead:** sign‑off on gates G0–G3 (A).
* **All:** safety adherence (C/I).

---

## 13) Change management

* All changes via PR with a **Runbook Note** summarising the effect on: safety, latency, and metrics.
* Tag releases at each gate (G0..G3).
* Keep configs in `robot/configs/` and `rl/configs/` under version control.

---

## 14) Troubleshooting runbooks

**Arm not detected**

* Check cable/hub power; run `lerobot-find-port`; try different USB port; verify udev/permissions on macOS.

**Camera black/slow**

* Reduce resolution/FPS; confirm OpenCV sees the device; test with `ffplay -f avfoundation`.

**Queue underflow/stutter**

* Increase `ACTIONS_PER_CHUNK`; lower control rate; enable RTC; ensure server GPU is not oversubscribed.

**Rewarder noisy**

* Tighten ROI; add few‑shot in‑domain images; blend with tiny classifier; add preference batches.

**High WAN jitter**

* Prefer LAN/VPN; cap FPS to match inference time; consider deploying a nearer GPU instance.

---

## 15) Expansion options

* **Virtual Workstation (vWS):** host Omniverse/Isaac Sim for scene editing; keep headless trainers separate.
* **On‑prem GPU**: 1–2× RTX 6000 Ada/L40S with vGPU for shared access once utilisation justifies it.
* **Sim pre‑training:** Isaac Lab domain randomisation to reduce on‑robot steps.

---

## 16) Glossary

* **VLA**: Vision‑Language‑Action policy.
* **VLM**: Vision‑Language Model (used here as a rewarder).
* **RTC**: Real‑Time Chunking (inference‑time smoothing of chunked actions).
* **PEFT/LoRA**: Parameter‑efficient fine‑tuning.

---

## 17) Appendices

### A. Environment variables (excerpt)

* `POLICY_SERVER_HOST`, `POLICY_SERVER_PORT`, `POLICY_SERVER_URL`
* `REWARDER_HOST`, `REWARDER_PORT`, `REWARDER_URL`
* `ACTIONS_PER_CHUNK`, `CHUNK_SIZE_THRESHOLD`
* `LOG_DIR`

### B. Make targets (excerpt)

* `make mac-bootstrap` — install mac deps.
* `make gpu-bootstrap` — install GPU host deps + clone OpenPI.
* `make serve-pi` — run π policy server.
* `make serve-rewarder` — run reward microservice.
* `ARM_ID=arm01 make run-arm` — start async client for an arm.
* `make train-rl` — kick off an RL run (skeleton).

### C. YAML examples

* `robot/configs/so101_armNN.yaml` — per‑arm port/camera/safety.
* `robot/safety/ee_limits.yaml` — cartesian box + joint clamps.
* `rl/configs/*.yaml` — task goal, actors, async, rewarder/policy URLs.

---

## 18) References (selected)

> Keep this short and curated for the team (update as we adopt/replace components).

* Hugging Face **LeRobot** (SO‑101 docs, async client/server)
* Physical Intelligence **OpenPI** (π0/π0‑FAST)
* **OpenVLA‑7B** (open baseline)
* **Qwen‑VL** (VLM family for rewarder)
* Preference learning/DPO resources
* Isaac Lab/Sim and RTC write‑ups

