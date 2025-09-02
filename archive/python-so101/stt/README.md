# Kyutai STT (Moshi) — Integration Scaffold

This folder provides scripts and Makefile targets to install and run a Kyutai Moshi STT server locally, and to configure the CLI to stream mic audio to it.

## Quick Start

```bash
# 1) Bootstrap (fill in repo/image details inside the script as needed)
make stt-moshi-bootstrap

# 2) Run server
make stt-moshi-serve

# 3) Update your .env with the Moshi URL (if needed)
# KYUTAI_MOSHI_URL=ws://localhost:8000/stream

# 4) Capture mic and stream to ASR backend (mock fallback if moshi not wired yet)
cargo run -p sr-cli --features voice-local/audio -- \
  voice-asr-capture --backend kyutai-moshi --language en --duration 10
```

## Notes
- These scripts are placeholders: please plug in the actual repo or Docker image for Kyutai Moshi used by your team.
- The CLI currently provides a plugin hook for `kyutai-moshi` but returns a “not integrated” message unless that backend is implemented or the hook is swapped to connect to your server.
- Recommended runtime: WebSocket streaming with PCM16 mono chunks at 16kHz or 24kHz.

