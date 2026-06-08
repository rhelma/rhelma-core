# Rhelma6 — Annex A1: P2P Overlay Protocol (Sketch)

**Status:** Draft

This annex outlines a pragmatic P2P layer that can evolve.

## Goals

- Peer discovery without central dependency (after bootstrap)
- Secure message transport (signed envelopes, optional encryption)
- Minimal message taxonomy

## Transport options

- QUIC (preferred for NAT traversal + multiplexing)
- WebSocket fallback (easy dev)

## Node-to-node messages (initial)

- `peer.hello` — announce node id + capabilities (signed)
- `peer.peers` — share peer list (signed)
- `peer.challenge` / `peer.challenge.result` — anti-sybil / verification (phase 4+)
- `task.request` / `task.result` — routing work (only with policy permission)
- `policy.bundle` — signed policy distribution
- `governance.log.append` — append-only governance log entries (phase 5+)

## Envelope

Reuse the Rhelma EventEnvelope concepts (id, timestamp, source, trace context),
but keep it light for P2P. Later we can converge them fully.

## Safety rule

If verification is uncertain, nodes must default to:
- deny privileged requests
- accept only low-risk tasks
