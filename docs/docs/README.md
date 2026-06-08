
# Rhelma Project

## Overview
Rhelma is a comprehensive system that provides a scalable and secure platform for building AI-driven applications.
The project is composed of several services and components that work together to provide a reliable, self-improving AI orchestration environment.

## Services
- **rhelma-core**: Core infrastructure.
- **ai-orchestrator**: Orchestrates AI improvement cycles.
- **node-registry**: Manages node registration and security.
- **value-ledger**: Keeps track of transactions and balances.

## Public Release Guides

- Open-source manifest: `../OPEN_SOURCE_MANIFEST.md`
- Commercial boundary: `../COMMERCIAL_BOUNDARY.md`
- Social manifest: `../SOCIAL_MANIFEST.md`
- Public repository plan: `release/PUBLIC_REPO_PLAN.md`
- Private repository plan: `release/PRIVATE_REPO_PLAN.md`
- Social release plan: `release/SOCIAL_RELEASE_PLAN.md`
- Open-source strategy: `open-source/README.md`
- Open-source release checklist: `open-source/RELEASE_CHECKLIST.md`
- Commercial boundary: `commercial/README.md`
- `rhelma.ir` site structure: `sites/rhelma-ir.md`
- `asrnegar.ir` demo structure: `sites/asrnegar-ir.md`

## How to Run Locally
1. Clone the repository: `git clone https://github.com/your/repo.git`
2. Install dependencies: `cargo build`
3. Run the system: `cargo run --release`

## Environment Variables
- `DATABASE_URL`: URL for the PostgreSQL database.
- `REDIS_URL`: URL for Redis connection.
- `NATS_URL`: URL for NATS messaging system.

## API Endpoints
- `/register`: Register a new node.
- `/heartbeat`: Send a heartbeat from a node.
- `/improvement-cycle`: Submit an improvement proposal.

## Common Issues
- Ensure Redis and NATS are running before starting the system.
