class NoopAdapter:
    """Always returns 0.5; useful as a wiring sanity check."""

    def score(self, frame, goal: str) -> float:
        return 0.5
