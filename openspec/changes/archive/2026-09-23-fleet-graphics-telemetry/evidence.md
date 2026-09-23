# Evidence

| Requirement and scenario | Implementation | Verification |
| --- | --- | --- |
| Honest telemetry: WSL NVIDIA | `observatory/probe.py`, `deploy/compose.gpu-wsl.yaml` | Non-root ephemeral Observatory container queried ws-255 WSL `nvidia-smi`; source and malformed-output tests in `test_probe.py` |
| Honest telemetry: remote fleet view of WSL graphics | `observatory/probe.py`, `observatory/core.py` | Host-bound, fresh, bounded loopback receipt tests in `test_probe.py`, including redirects, slow bodies and malformed JSON; direct read from ws-255 omaterm shell and live Personal display both returned NVIDIA aggregate |
| Honest telemetry: Intel Xe device graphics | `observatory/gpu_probe.py`, `observatory/probe.py`, `deploy/compose.gpu-intel.yaml` | Ephemeral no-network, `CAP_PERFMON` container published a real Intel aggregate; PMU calculation, fresh-sample, bounds and reset tests in `test_gpu_probe.py` and `test_probe.py` |
| Honest telemetry: invalid counters and source isolation | `observatory/probe.py`, `observatory/core.py` | Malformed NVIDIA and Intel values remain unavailable; other metrics and Herdr agents persist in `test_probe.py` and `test_core.py` |
| Reproducible deployment: restart/update and optional graphics | `deploy/compose.gpu-intel.yaml`, `deploy/compose.gpu-wsl.yaml`, `README.md` | `docker compose config --format json` confirms dashboard UID 1000, dropped capabilities and host network; Intel monitor has no network or host mount, only `CAP_PERFMON`; WSL exposes only `/dev/dxg` and read-only `/usr/lib/wsl` |
| Readable fleet: missing values and measured source | `web/app.js` | `test_ui.cjs` covers source-aware accessible description, unavailable and stale values |

## Gates

The complete 116 Python and 194 JavaScript tests, JavaScript syntax, strict OpenSpec validation and `git diff --check` passed after adding remote graphics receipt. Independent review found and resolved the initial Intel `/proc` exposure, Xe engine-capacity calculation, malformed aggregate, and remote redirect/deadline/parser paths; its renewed verdict has no actionable findings. An ephemeral monitor produced a real Intel sample with no network or host mount.

Versioned image `ce2c713` runs healthy on both hosts. Both Personal and Work APIs show iapetus with `intel-xe-pmu` graphics and ws-255 with `nvidia-visible` graphics while both hosts remain online. The existing Chrome Personal display accessibility tree shows nonblank Graphics values and source-labelled descriptions for both machines. The displayed ws-255 zero is a measured NVIDIA utilisation, distinct from the previous unavailable value. The `e1aaf41` and original `f12d43c` images remain on both hosts for rollback. Previous deployment settings were preserved, as was the original private Personal config before adding ws-255's loopback GPU port. No private agent payloads are included here.
