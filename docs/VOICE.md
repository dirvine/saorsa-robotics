Kyutai STT (Local Scaffold)

- App: `apps/kyutai-stt-app` provides a hotkey-triggered recorder that downloads Kyutai `kyutai/stt-2.6b-en` assets via `hf-hub` and transcribes.
- Current status: Real transcripts via optional Hugging Face Inference API fallback while local Candle Mimi+decoder wiring lands.

Usage
- Build: `cargo build -p kyutai-stt-app`
- Run via CLI: `sr voice-kyutai --config config.json --hotkey F12 --duration 5`
- Or direct: `./target/debug/kyutai-stt-app --config config.json --duration 5`
- Inspect weights: `./target/debug/kyutai-stt-app --dump-weights` (prints Mimi and decoder tensor keys/shapes)

Hugging Face Inference Fallback
- Set `HUGGINGFACEHUB_API_TOKEN` in your environment (see `.env.example`).
- Optionally set `HF_INFERENCE_URL` (defaults to Kyutai STT 2.6B endpoint).
- The app records 24 kHz mono WAV chunks and posts them to the HF endpoint, returning the `text` field when present.

Local Inference (WIP)
- Model assets fetched: `config.json`, `model.safetensors`, Mimi weights, tokenizer model.
- Candle scaffolding for Mimi encoder and Transformer decoder is in place; forward paths to be implemented.
- Mimi skeleton includes a named-parameter loader stub: set `KYUTAI_MIMI_STRICT_LOAD=1` to make the app error out if Mimi weights are not mapped yet (useful while wiring real layers).
- Tokenization: integration planned; current build does not require tokenizer to produce HF-backed transcripts.

Troubleshooting
- If you see "Model not loaded", ensure the app printed "Kyutai STT model ready" before recording.
- If HF fallback fails, the app prints the HTTP status; verify your token and endpoint.
