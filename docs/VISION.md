# Vision — Stereo Utilities

This repo includes a basic vision scaffold with a mock backend and an optional OpenCV backend. You can test cameras, capture stereo image pairs, and run stereo chessboard calibration to produce YAML.

## Build with OpenCV

OpenCV is feature-gated to keep CI portable. Enable the feature when running:

```bash
cargo run -p sr-cli --features vision-stereo/opencv -- vision-list
```

## Test a camera

```bash
# Index 0 using OpenCV
cargo run -p sr-cli --features vision-stereo/opencv -- vision-test --device 0 --opencv

# Fallback to mock backend
cargo run -p sr-cli -- vision-test --device 0
```

## Capture stereo image pairs

Save N synchronized frames from two devices to left/right directories:

```bash
LEFT=0
RIGHT=1
cargo run -p sr-cli --features vision-stereo/opencv -- \
  vision-capture-stereo --left-device $LEFT --right-device $RIGHT \
  --count 50 --left-dir data/left --right-dir data/right
```

Pairs are written with matching filenames (e.g., `00000.png`), so they can be fed directly to the calibration tool.

## Chessboard stereo calibration

Given two folders of paired chessboard images, estimate intrinsics/extrinsics and rectification matrices, then write YAML:

```bash
cargo run -p sr-cli --features vision-stereo/opencv -- \
  vision-calib-stereo --left-dir data/left --right-dir data/right \
  --grid 9x6 --square-mm 25 --out configs/calib/stereo.yaml

## Depth and point cloud (OpenCV)

Given rectified stereo images and a stereo YAML with `Q`, compute disparity and export a point cloud:

```bash
cargo run -p sr-cli --features vision-stereo/opencv -- \
  vision-depth --left data/left/00010.png --right data/right/00010.png \
  --calib configs/calib/stereo.yaml --out-depth out/disp_00010.png --out-ply out/cloud_00010.ply \
  --roi 200,120,800,480
```

Notes:
- Uses OpenCV SGBM with conservative defaults. Tune as needed.
- ROI reduces output size and can help avoid invalid regions.
```

Notes:
- Requires at least 5 valid pairs where chessboard corners are found.
- Grid is `COLSxROWS`, matching OpenCV’s convention.
- YAML stores `K1/D1/K2/D2/R/T` and `R1/R2/P1/P2/Q` matrices.
