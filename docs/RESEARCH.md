# RESEARCH.md — Evidence & Pointers

## Jetson Thor (deployment)
- NVIDIA page: 2070 FP4 TFLOPS, 128 GB, 40–130 W; **Jetson AGX Thor**; native **2× CAN** on carrier.  
- Dev‑kit carrier spec PDF (Aug 2025).  
- Coverage (TechRadar/WindowsCentral/Barron’s/ServeTheHome).

## VLA / ARM models
- **OpenVLA** — open‑source VLA (7B), pretrained on 970k episodes (Open‑X Embodiment), code & checkpoints; **OpenVLA‑OFT** shows 25–50× faster inference and ↑success on LIBERO.  
- **MolmoAct (AI2, 2025)** — first fully open **Action Reasoning Model (ARM)** “thinks in 3D”, plans waypoints; blog & coverage.  
- Related: **Octo** (Stanford), **RDT‑1B** (Tsinghua), **Open X‑Embodiment** dataset.

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

_Last updated: 2025-09-01_