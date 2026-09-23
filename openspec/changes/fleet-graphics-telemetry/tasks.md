# Tasks

## 1. Collect graphics measurements

- [x] 1.1 Add isolated Intel Xe PMU sampling and bounded aggregate receipt, verifying measured, idle, missing, stale and reset cases with focused tests.
- [x] 1.2 Preserve valid NVIDIA and Intel source labels through metric validation, and verify malformed graphics never hides other metrics or agents.

## 2. Present and deploy graphics scope

- [x] 2.1 Show the graphics source and scope in the fleet panel, and verify accessible text for valid, missing and stale readings.
- [x] 2.2 Add opt-in Intel monitor and WSL GPU Compose overrides, document deployment and rollback, and verify the dashboard retains its base security settings while the monitor has no host process mount or network.

## 3. Acceptance

- [x] 3.1 Run Python, JavaScript, strict OpenSpec and diff checks; record requirement-to-evidence traceability.
- [ ] 3.2 Build and deploy a versioned image to both hosts, verify live scoped graphics readings and preserve the previous image for rollback.
