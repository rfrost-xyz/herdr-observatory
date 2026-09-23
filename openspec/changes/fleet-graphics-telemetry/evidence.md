# Evidence

| Requirement and scenario | Implementation | Verification |
| --- | --- | --- |
| Honest telemetry: WSL NVIDIA | `observatory/probe.py`, `deploy/compose.gpu-wsl.yaml` | Non-root ephemeral Observatory container queried ws-255 WSL `nvidia-smi`; source and malformed-output tests in `test_probe.py` |
| Honest telemetry: Intel Xe device graphics | `observatory/gpu_probe.py`, `observatory/probe.py`, `deploy/compose.gpu-intel.yaml` | Ephemeral no-network, `CAP_PERFMON` container published a real Intel aggregate; PMU calculation, fresh-sample, bounds and reset tests in `test_gpu_probe.py` and `test_probe.py` |
| Honest telemetry: invalid counters and source isolation | `observatory/probe.py`, `observatory/core.py` | Malformed NVIDIA and Intel values remain unavailable; other metrics and Herdr agents persist in `test_probe.py` and `test_core.py` |
| Reproducible deployment: restart/update and optional graphics | `deploy/compose.gpu-intel.yaml`, `deploy/compose.gpu-wsl.yaml`, `README.md` | `docker compose config --format json` confirms dashboard UID 1000, dropped capabilities and host network; Intel monitor has no network or host mount, only `CAP_PERFMON`; WSL exposes only `/dev/dxg` and read-only `/usr/lib/wsl` |
| Readable fleet: missing values and measured source | `web/app.js` | `test_ui.cjs` covers source-aware accessible description, unavailable and stale values |

## Gates

The complete Python and JavaScript suites, JavaScript syntax, strict OpenSpec validation and `git diff --check` passed after the Intel design change. The previous `/proc` mount was removed following independent review because process links could reach host files. An ephemeral monitor produced a real Intel sample with no network or host mount. Live deployment and final reviewed-head gates are recorded after release validation.
