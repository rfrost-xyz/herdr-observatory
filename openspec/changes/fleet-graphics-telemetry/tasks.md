# Tasks

## 1. Collect graphics measurements

- [ ] 1.1 Add bounded, read-only Intel Xe client counter sampling and verify measured, idle, duplicate, missing and reset cases with focused probe tests.
- [ ] 1.2 Preserve valid NVIDIA and Intel source labels through metric validation, and verify malformed graphics never hides other metrics or agents.

## 2. Present and deploy graphics scope

- [ ] 2.1 Show the graphics source and scope in the fleet panel, and verify accessible text for valid, missing and stale readings.
- [ ] 2.2 Add opt-in Intel and WSL GPU Compose overrides, document deployment and rollback, and verify rendered Compose configuration retains the base security settings.

## 3. Acceptance

- [ ] 3.1 Run Python, JavaScript, strict OpenSpec and diff checks; record requirement-to-evidence traceability.
- [ ] 3.2 Build and deploy a versioned image to both hosts, verify live scoped graphics readings and preserve the previous image for rollback.
