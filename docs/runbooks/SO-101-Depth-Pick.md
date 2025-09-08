# SO‑101 Depth Pick — Runbook (SO‑101)

Status: draft. This runbook wires RGB(+D) → grasp → policy → safety → CAN.

Prereqs
- Build workspace: `cargo build --workspace --all-targets`
- Camera calibrated (stereo) or depth device available
- Device descriptors in `configs/devices/*.yaml` (e.g., `odrive_axis.yaml`)

Quick Start (Mock Data)
- Demo with synthetic depth and mock CAN:
  - `cargo run -p vla-policy-demo -- --roi 92,92,40,40 --send`
  - AprilTag grasp (requires OpenCV+AprilTag features):
    - `cargo run -p vla-policy-demo --features vla-policy-demo/vision-opencv,vla-policy-demo/apriltag -- \
       --tag-image data/tag.png --intr 600,600,112,112 --tag-size-m 0.05`

Stereo Calibration
- Capture chessboard images from each camera:
  - `cargo run -p sr-cli -- vision-capture-stereo --left /dev/video2 --right /dev/video3 --out-left data/left --out-right data/right --count 50`
- Calibrate:
  - `cargo run -p sr-cli -- vision-calib-stereo --left-dir data/left --right-dir data/right --grid 9x6 --square-mm 25 --out configs/calib/stereo.yaml`

Depth / Point Cloud
- Compute disparity/depth and optionally a point cloud (OpenCV backend):
  - `cargo run -p sr-cli --features vision-stereo/opencv -- vision-depth --left data/L.png --right data/R.png --calib configs/calib/stereo.yaml --out-ply out/scene.ply --roi 100,100,80,80`

ROI → Grasp (Prototype)
- Use the helper in `vision-stereo` to estimate a surface normal + grasp at the ROI center. The demo calls this automatically when `--roi` is provided.
- AprilTag mode: `--tag-image <path>` with `--intr fx,fy,cx,cy` and optional `--tag-size-m` computes a tag-frame grasp.

Policy Step
- The demo requests two heads: `EndEffectorDelta` and `Gripper` (plus a `JointPositions` fallback in the mock). Replace the mock with your model once ready.
- IK stub converts `EndEffectorDelta` into joint targets with basic clamping; see demo source for `DefaultKinematics`.

Safety Guard
- Actions pass through default limits (`safety-guard`). Violations block sending and print diagnostics.

CAN Send (Mock by default)
- The demo loads the first descriptor from `configs/devices` and prints/sends frames via `mock0`. Replace with `slcan`/`socketcan` when hardware is attached (see docs/CAN.md).

Notes
- Observation accepts optional depth (`depth_u16`, `depth_shape`) and `camera_T_base` (row‑major 4×4) for frame transforms.
- For AprilTag grasping, build `sr-cli` with `vision-stereo/opencv,vision-stereo/apriltag` features and use its tag tools.
