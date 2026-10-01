# Evidence

Synthetic fixtures only. No private transcripts, ids, paths or account data. The corpus check records aggregate counts only.

## Baseline

The unchanged runtime at `3ed6c11` is byte-identical in source to `2d2be90`, whose baseline was recorded in the archived research change:

- release binary sha256 `74f50d695c89e4cc01198c77df9d31ab9b1230e8e5948a6a73dceb569d76931d`;
- runtime CPU 0.031 s over 30 s;
- peak RSS 4,100 KiB;
- mean snapshot 27,674.6 bytes;
- all suites green.

Intermittent test: in CI, `allowances::tests::account_rpc_flood_is_bounded_and_cancellation_prevents_spawn` failed at `allowances.rs:749` (`create_dir` AlreadyExists) on 2 of 12 runs on the research branch. The cause is fixture directories named by pid and timestamp only. `0b9669e` adds a per-site sequence, and five consecutive local full runs passed. A deterministic regression test is not possible, because the collision needs two equal clock reads.

## Traceability

| Requirement / scenario | Implementation | Verification | Commit |
|---|---|---|---|
