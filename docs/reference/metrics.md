# Metrics Reference (parsed from tegrastats)

## Memory
- RAM: used/total bytes; Largest Free Block (blocks or size).
- SWAP: used/total/cached bytes.
- IRAM: used/total/lfb bytes.

## CPU
- Per-core load percent, frequency MHz.
- Governor (read; set via control).

## Engines (usage/freq/raw as available)
- EMC, MC, AXI
- GR3D (GPU)
- NVENC, NVDEC, NVJPG, NVJPG1
- VIC, OFA, ISP, NVCSI
- PCIE, NVLINK, APE
- UTIL-only tokens (e.g., ISP_UTIL, NVCSI_UTIL) are mapped to base engines.
- “off” engines are reported with 0% usage.

## Temperatures
- All sensors reported by tegrastats (e.g., CPU, GPU, Tboard, AO, PLL, etc.).

## Power
- Rails current/avg mW (e.g., VDD_IN, VDD_CPU, VDD_GPU, VDD_SOC, VDD_DDR, etc.).

## MTS
- Foreground/background percent if present.

## Telemetry export
- Prometheus metrics under `jetsonscope_*` (see docs/telemetry.md).
- `/debug/snapshot` returns JSON with stats + control status.
