# Holds Replication E2E (Phase 50)

This guide validates that deposit holds propagate across VLF federation.

## Steps (example)
1) Start VLF instance A and B (federated peers).
2) On A: create hold for subject `node:<node_id>` with reason `admission.deposit`.
3) Force snapshot push A -> B.
4) On B: attempt withdraw tx (delta < 0) for that subject; expect rejection.
5) Clear hold on A, push snapshot again.
6) On B: withdraw should now succeed.

Hook scripts can be added once the exact endpoints in your VLF implementation are finalized.
