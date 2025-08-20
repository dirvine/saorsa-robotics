# Camera Setup Guide for SO-101 Robotic Arms

## Overview
This guide covers camera integration for SO-101 robotic arms with Vision-Language-Action (VLA) model training support.

## Hardware Setup

### Recommended Cameras

#### For Wrist Mounting
- **Logitech C920/C922**: Good balance of quality and cost
- **Intel RealSense D435**: When depth is needed
- **Arducam USB cameras**: Compact options

#### For Overhead/Side Views
- **Intel RealSense D455**: Longer range depth
- **Logitech Brio**: 4K resolution
- **Industrial cameras**: Basler ace series

### Mounting Solutions

#### 3D Printed Mounts
STL files are provided in `hardware/camera_mounts/`:
- `wrist_mount_so101.stl`: Direct wrist attachment
- `overhead_clamp.stl`: Table/desk clamp
- `magnetic_base.stl`: Quick-release magnetic mount

Print settings:
- Material: PETG or ABS
- Layer height: 0.2mm
- Infill: 40%
- Support: Yes for overhangs

#### Commercial Mounting Options
1. **RAM Mounts**: Flexible ball joints
2. **SmallRig**: Camera cages and arms
3. **Manfrotto Magic Arms**: Professional positioning

### Wiring and USB Management

For 4-arm setup with cameras:
cat > docs/CAMERA_SETUP.md << 'EOF'
# Camera Setup Guide for SO-101 Robotic Arms

## Overview
This guide covers camera integration for SO-101 robotic arms with Vision-Language-Action (VLA) model training support.

## Hardware Setup

### Recommended Cameras

#### For Wrist Mounting
- **Logitech C920/C922**: Good balance of quality and cost
- **Intel RealSense D435**: When depth is needed
- **Arducam USB cameras**: Compact options

#### For Overhead/Side Views
- **Intel RealSense D455**: Longer range depth
- **Logitech Brio**: 4K resolution
- **Industrial cameras**: Basler ace series

### Mounting Solutions

#### 3D Printed Mounts
STL files are provided in hardware/camera_mounts/:
- wrist_mount_so101.stl: Direct wrist attachment
- overhead_clamp.stl: Table/desk clamp
- magnetic_base.stl: Quick-release magnetic mount

#### Commercial Mounting Options
1. **RAM Mounts**: Flexible ball joints
2. **SmallRig**: Camera cages and arms
3. **Manfrotto Magic Arms**: Professional positioning

## Software Installation

### 1. Install Dependencies
make install-camera-deps

### 2. Test Camera Detection
make test-camera

### 3. Camera Calibration
make calibrate-camera

## Configuration

Edit robot/configs/camera_config.yaml for your setup.

## Data Collection

### Starting Collection
make collect-demos

## Troubleshooting

### Camera Not Found
- Check USB connections
- Verify camera permissions
- Test with cv2.VideoCapture()

### Low Frame Rate
- Reduce resolution
- Check USB bandwidth
- Use USB 3.0 ports

## Support

For issues, check example configurations in robot/configs/
