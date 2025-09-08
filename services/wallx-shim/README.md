# WALL‑X HTTP Shim (stub)

A tiny FastAPI server that mocks a WALL‑OSS style action service. It accepts a JSON Observation and returns EndEffectorDelta + Gripper actions. Replace the mock `predict()` with real calls into `wall-x` when ready.

## Quick start

```bash
python -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt
uvicorn app:app --host 0.0.0.0 --port 9009 --reload
```

## Request shape (example)
```json
{
  "image_shape": [224,224,3],
  "joint_positions": [0,0,0,0,0,0],
  "ee_pose": [0.3,0.0,0.2,0,0,0]
}
```

## Response shape
```json
{
  "actions": [
    {"action_type": "EndEffectorDelta", "values": [0.0,0.0,-0.02,0,0,0], "confidence": 0.9},
    {"action_type": "Gripper", "values": [0.0], "confidence": 0.95}
  ],
  "inference_time_ms": 5.5,
  "metadata": {}
}
```

