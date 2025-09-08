# X Square Robot — WALL‑OSS Summary (captured 2025‑09‑08)

Sources
- Research page: https://www.x2robot.com/en/research
- GitHub org: https://github.com/X-Square-Robot
- Repo: https://github.com/X-Square-Robot/wall-x
- Models: https://huggingface.co/x-square-robot/wall-oss-flow, https://huggingface.co/x-square-robot/wall-oss-fast

## What They Released
- “WALL‑OSS” is presented as an end‑to‑end embodied foundation model aiming for: (1) embodiment‑aware vision‑language understanding, (2) strong language‑action association, and (3) robust manipulation. Two model variants are linked on HuggingFace (“flow” and “fast”).
- The public repo (wall‑x) provides training/inference code, using LeRobot datasets, a Qwen2.5‑based VLM‑MoE action head, and scripts for SFT and inference.
- Model cards show a 4.22B‑parameter BF16 checkpoint and reference a whitepaper “WALL‑OSS: Igniting VLMs toward the Embodied Space”.

## High‑Level Method (from public materials)
- Training stack: Python repo + LeRobot format; SFT scripts and evaluation utilities are included. Inference examples use proprioception, DOF masks, dataset names, and a validation “mode” flag.
- Architectural notes: Qwen2.5‑VLMoE‑for‑Action backbone; unified cross‑level chain‑of‑thought (instruction reasoning → subgoal decomposition → fine‑grained action synthesis) claimed by the model cards.
- Objectives/claims: strong long‑horizon manipulation and instruction‑following; improved generalization vs. baselines (details in linked whitepaper and repo).

## Useful Artifacts
- Code: `X-Square-Robot/wall-x` (Python). Direct Torch API for loading action models, example scripts `scripts/fake_inference.py`, `scripts/draw_openloop_plot.py`.
- Checkpoints: `x-square-robot/wall-oss-flow`, `x-square-robot/wall-oss-fast` (HuggingFace).

## Mapping to This Repository
- Inputs: Our `Observation` already carries image, joints, timestamp; we recently added optional `depth_u16`, `depth_shape`, `camera_T_base`. To mirror their examples, we could optionally add `dof_mask` and `dataset_name` fields (kept optional) if we later call their Python model.
- Actions: Our policy types already include `EndEffectorDelta` + `Gripper` and joint‑space commands; demo now converts EE deltas to joint targets via an IK stub and runs safety checks.
- Integration path (future): expose a small gRPC/HTTP inference shim in Python (wall‑x) and call from our Rust `vla-policy` crate; start with mocked outputs, then swap to real model.

## Notes
- This is a paraphrased summary of public pages captured on 2025‑09‑08. See links above for the latest.

