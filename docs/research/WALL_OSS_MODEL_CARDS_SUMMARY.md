# WALL‑OSS Model Cards — Paraphrased Highlights (captured 2025‑09‑08)

Sources
- HF: x-square-robot/wall-oss-flow
- HF: x-square-robot/wall-oss-fast

Notes below are short, paraphrased excerpts for offline reference. Please consult the model cards for authoritative, up‑to‑date details.

## Scope & Goals
- End‑to‑end embodied model (vision‑language‑action) targeting long‑horizon manipulation and instruction following.
- Emphasizes “cross‑level chain‑of‑thought”: language reasoning → subgoal decomposition → action synthesis.

## Variants
- “flow” and “fast” checkpoints; the cards mention a 4.22B‑parameter BF16 model and smaller variants for latency‑sensitive scenarios.

## Data & Training
- LeRobot‑style datasets; includes proprioception and (optionally) DOF masks.
- Supervised fine‑tuning (SFT) recipes in the `wall-x` repository; evaluation and inference examples provided.

## Inputs & Outputs
- Inputs: RGB image(s), text instruction, proprioception (joint positions/velocities), optional DOF masks.
- Outputs: action vectors (e.g., end‑effector deltas, gripper commands, or joint targets), plus optional logs/metadata.

## Intended Use & Limitations (abridged)
- Research purposes; not safety‑certified; users must implement safety layers and validate in their environment.
- Performance depends on correct calibration, camera viewpoint, manipulation domain, and hardware.

## Integration Tips (this repo)
- Observation now supports optional `depth_u16`, `depth_shape`, `camera_T_base`, `dof_mask`, and `dataset_name`.
- We provide a stub HTTP shim (`services/wallx-shim`) and an optional Rust client policy feature (`vla-policy/wallx-http`).

