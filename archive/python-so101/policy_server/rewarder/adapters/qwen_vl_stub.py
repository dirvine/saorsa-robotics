# Placeholder illustrating where a real VLM would be called.
# Swap this for an actual Transformers pipeline when ready.


class QwenVLStub:
    def __init__(self):
        pass

    def score(self, frame, goal: str) -> float:
        # TODO: implement real prompt + model call, for now a tiny heuristic:
        # if goal mentions "red" and the frame has a lot of red-ish pixels in the lower half, boost score.
        import cv2
        import numpy as np

        hsv = cv2.cvtColor(frame, cv2.COLOR_BGR2HSV)
        # naive red mask
        lower1 = np.array([0, 120, 70])
        upper1 = np.array([10, 255, 255])
        lower2 = np.array([170, 120, 70])
        upper2 = np.array([180, 255, 255])
        mask = cv2.inRange(hsv, lower1, upper1) | cv2.inRange(hsv, lower2, upper2)
        h, w = mask.shape
        lower_half = mask[h // 2 :, :]
        frac_red = float((lower_half > 0).sum()) / float(lower_half.size)
        base = 0.3 + 0.7 * min(1.0, frac_red * 4.0)
        if "red" in goal.lower():
            return base
        return 0.3 + 0.5 * base
