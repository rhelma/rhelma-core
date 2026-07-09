# Rhelma Core

## AI-Native Workspace Foundation

Rhelma Core is an open-source Rust foundation for building intelligent workspace systems.

It provides reusable primitives for developers who want to build applications where users, services, and AI agents can work together securely.

## What is Rhelma Core?

Rhelma Core focuses on foundational building blocks:

- Workspace-aware identity primitives
- Authentication foundations
- Event-driven communication
- Capability-based service design
- Policy and authorization primitives
- Health and telemetry foundations

The core project is designed to help developers create AI-enabled systems without depending on a specific hosted platform.

## Architecture

```text
Application
    |
Workspace
    |
AI Agent
    |
Capabilities
    |
Services
    |
Events
```

## Core Principles

### Workspace First

Applications can organize users and services around secure workspace contexts.

### Secure by Default

Rhelma Core provides foundations for:

- authentication
- authorization
- service boundaries
- auditable actions

### Event Driven

Shared event contracts allow services to communicate through explicit interfaces.

## Included in Rhelma Core

This repository contains public developer foundations:

- Rust libraries
- contracts
- security primitives
- event abstractions
- capability models
- documentation
- examples

## Not Included

Commercial platform features remain outside the public core:

- hosted platform services
- enterprise administration
- billing systems
- customer integrations
- managed operations
- proprietary automation workflows

## Getting Started

Requirements:

- Rust toolchain

Build:

```bash
cargo check --workspace
```

Test:

```bash
cargo test --workspace
```

## Contributing

Contributions are welcome.

Before contributing, read:

- CONTRIBUTING.md
- SECURITY.md
- OPEN_CORE_SCOPE.md

## License

See LICENSE for usage terms.
