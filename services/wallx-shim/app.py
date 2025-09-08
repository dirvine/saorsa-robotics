from fastapi import FastAPI
from pydantic import BaseModel
from typing import List, Optional, Dict, Any
import time

app = FastAPI(title="WALL-X HTTP Shim", version="0.1.0")


class Observation(BaseModel):
    image_shape: Optional[List[int]] = None
    joint_positions: Optional[List[float]] = None
    joint_velocities: Optional[List[float]] = None
    ee_pose: Optional[List[float]] = None
    depth_shape: Optional[List[int]] = None
    dof_mask: Optional[List[int]] = None
    dataset_name: Optional[str] = None


class Action(BaseModel):
    action_type: str
    values: List[float]
    confidence: float
    timestamp: Optional[float] = None


class PolicyResult(BaseModel):
    actions: List[Action]
    inference_time_ms: float
    metadata: Dict[str, Any]


@app.get("/")
def root() -> Dict[str, str]:
    return {"status": "ok", "service": "wallx-shim"}


@app.post("/infer", response_model=PolicyResult)
def infer(obs: Observation) -> PolicyResult:
    t0 = time.time()
    # Mock policy: move slightly down in Z, keep gripper open
    ee_delta = Action(
        action_type="EndEffectorDelta",
        values=[0.0, 0.0, -0.02, 0.0, 0.0, 0.0],
        confidence=0.9,
        timestamp=time.time(),
    )
    gripper = Action(
        action_type="Gripper",
        values=[0.0],
        confidence=0.95,
        timestamp=time.time(),
    )
    dt = (time.time() - t0) * 1000.0
    return PolicyResult(actions=[ee_delta, gripper], inference_time_ms=dt, metadata={})

