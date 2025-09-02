# CAN on macOS (SLCAN)

This repo includes a mac-friendly SLCAN backend that speaks the SLCAN text protocol over serial. Many USB–CAN adapters on macOS expose a serial device like `/dev/tty.usbserial-XXXX` or `/dev/tty.usbmodem-XXXX`.

## List available ports

```bash
# Optional: include a default bitrate to show planned setting
cargo run -p sr-cli -- can-list --backend slcan --bitrate 500k
```

## Sniff frames

```bash
# Replace with your serial device path
DEV=/dev/tty.usbserial-0001
# Default bitrate is 500k; override with --bitrate
cargo run -p sr-cli -- can-sniff --device $DEV --backend slcan --bitrate 250k --count 20

# Save sniff to .srlog (NDJSON)
cargo run -p sr-cli -- can-sniff --device $DEV --backend slcan --bitrate 250k --count 100 --to /tmp/capture.srlog

# Live decode using a descriptor file, print decoded JSON and write to JSONL
cargo run -p sr-cli -- can-sniff --device $DEV --backend slcan \
  --desc-file configs/devices/tmotor_ak80_9.yaml --decode --decode-to /tmp/capture_decoded.jsonl \
  --count 100

# Or decode using all descriptors in a directory
cargo run -p sr-cli -- can-sniff --device $DEV --backend slcan \
  --desc-dir configs/devices --decode --count 50
```

## Send a frame

```bash
DEV=/dev/tty.usbserial-0001
# Send standard ID 0x123 with payload 01 02 03
cargo run -p sr-cli -- can-send --device $DEV --backend slcan --bitrate 500k --id 0x123 --data 01 02 03
```

## Doctor: quick validation

Open the device, set a bitrate, send a probe frame, and optionally listen briefly for any response or background traffic:

```bash
DEV=/dev/tty.usbserial-0001
cargo run -p sr-cli -- can-doctor --device $DEV --backend slcan --bitrate 500k --id 0x123 --data 00 00 --recv-ms 300
```

Notes:
- Some adapters/firmware do not echo frames; a "no frame" receive result is not necessarily an error.
- Use `--recv-ms 0` to skip waiting.

## Replay from .srlog

Read an `.srlog` file and print frames, optionally re-sending them to an interface. With `--realtime`, approximate original timing based on timestamps.

```bash
# Print-only
cargo run -p sr-cli -- can-replay --from /tmp/capture.srlog

# Re-send to device at 500k, following timing
cargo run -p sr-cli -- can-replay --from /tmp/capture.srlog --backend slcan --device $DEV --bitrate 500k --send --realtime
```

Notes:
- Default bitrate is set to 500 kbit (S6). Adjust in code if your bus differs.
- Hardware acceptance filters are not yet supported via SLCAN.
- If you do not have an adapter connected, use the mock backend:

```bash
cargo run -p sr-cli -- can-list --backend mock
cargo run -p sr-cli -- can-sniff --device mock0 --backend mock
```
