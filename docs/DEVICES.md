# Device Registry

YAML-based device descriptors define buses, protocols, joint limits, and CAN frame mappings. Load a directory of descriptors into the registry and validate with `sr-cli`.

## Schema (informal)

```yaml
id: string               # unique device id
bus: string              # logical bus name (e.g., can0)
protocol: string         # tmotor_ak | odrive | cia402 | cyphal | custom
node_id: int|hex         # optional node id
joints:
  - name: string
    limits:
      pos_deg: [min, max]
      vel_dps: number
      torque_nm: number
    map:
      mode: torque|velocity|position
      scale:
        k_t: number
      frames:
        - id: hex|string
          fmt: string
      pd:                 # Optional default PD gains (used for position/velocity commands)
        kp: number
        kd: number
telemetry:
  - id: hex|string
    fmt: string
heartbeat:
  id: hex|string
  period_ms: number
```

## Examples

See `configs/devices/`:
- `tmotor_ak80_9.yaml` — T-Motor AK
- `odrive_axis.yaml` — ODrive axis

## CLI

```bash
# Validate one file
cargo run -p sr-cli -- device-validate --file configs/devices/tmotor_ak80_9.yaml

# Validate a directory and print JSON
cargo run -p sr-cli -- device-validate --dir configs/devices --json

# List devices in a directory
cargo run -p sr-cli -- device-list --dir configs/devices
```

## Next
- Drivers for `tmotor_ak`, `odrive`, `cia402`, `cyphal` mapping to CAN frames.
- Telemetry decode tables.
- Prometheus export from daemon.
