
# Rhelma Architecture

## Core Components
1. **rhelma-core**: The central part of the system, handling low-level operations.
2. **ai-orchestrator**: Manages AI improvement cycles and orchestrates self-improvement loops.
3. **node-registry**: Responsible for node registration, security, and network discovery.
4. **value-ledger**: Maintains a ledger of transactions, ensuring consistency across the system.

## Design Decisions
- **Distributed architecture**: The system is designed to scale horizontally, with nodes being added dynamically.
- **Security**: We use Proof of Work (PoW) and rate limiting for node admission to prevent Sybil attacks.

## Data Flow
1. Nodes register and verify themselves through the admission process.
2. Once verified, nodes can interact with the system and submit events for processing.
3. Events are processed through NATS and Redis dispatchers.
4. Results are logged and tracked through the **value-ledger**.
