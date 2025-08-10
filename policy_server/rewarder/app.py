import os

import cv2
import numpy as np
import uvicorn
from adapters.noop import NoopAdapter
from fastapi import FastAPI, File, Form, UploadFile
from pydantic import BaseModel

try:
    from adapters.qwen_vl_stub import QwenVLStub as DefaultAdapter
except Exception:
    DefaultAdapter = NoopAdapter

app = FastAPI(title="Saorsa Rewarder", version="0.1.0")

# Choose adapter via env
ADAPTER = os.environ.get("REWARD_ADAPTER", "qwen_vl_stub").lower()
if ADAPTER == "noop":
    adapter = NoopAdapter()
else:
    adapter = DefaultAdapter()


class ScoreResponse(BaseModel):
    score: float  # 0..1


@app.get("/health")
def health():
    return {"ok": True, "adapter": ADAPTER}


@app.post("/score", response_model=ScoreResponse)
async def score(file: UploadFile = File(...), goal: str = Form(...)):
    data = await file.read()
    # Decode image
    img_array = np.frombuffer(data, np.uint8)
    frame = cv2.imdecode(img_array, cv2.IMREAD_COLOR)
    if frame is None:
        return ScoreResponse(score=0.0)
    s = float(adapter.score(frame, goal))
    # clamp
    s = max(0.0, min(1.0, s))
    return ScoreResponse(score=s)


if __name__ == "__main__":
    uvicorn.run(
        app,
        host=os.environ.get("REWARDER_HOST", "0.0.0.0"),
        port=int(os.environ.get("REWARDER_PORT", 18080)),
    )
