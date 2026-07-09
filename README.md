# Rhelma Core

## Open Foundation for AI-Native Systems

Rhelma Core is an open-source Rust foundation for building secure, intelligent, and event-driven applications.

The project provides reusable infrastructure primitives that help developers create systems with authentication, capabilities, policies, events, and reliable AI integrations.

Rhelma Core is designed as a foundation layer, not as a hosted product.

---

## Why Rhelma Core Exists

Modern applications are becoming distributed systems where software services, automation, and AI agents need to communicate safely.

Rhelma Core provides common building blocks for:

- secure service communication
- AI agent foundations
- capability-driven actions
- policy enforcement
- event-based architectures
- operational reliability primitives

---

## Core Architecture

```text
Application
    |
Core Libraries
    |
Capabilities
    |
Policies
    |
Events
    |
Services
```

The goal is to provide stable foundations that developers can compose into their own applications.

---

## Public Core Principles

### Security by Design

Rhelma Core focuses on reusable security foundations:

- authentication primitives
- authorization models
- explicit service boundaries
- auditable actions

### Capability-Based Systems

Services can describe supported actions through explicit capability definitions.

This enables safer automation and AI-assisted workflows.

### Event-Driven Architecture

Shared event contracts allow independent components to communicate through clear interfaces.

---

## AI Agent Foundations

Rhelma Core provides primitives for building AI-enabled applications.

The public core focuses on:

- agent interfaces
- action contracts
- capability discovery
- policy-controlled execution models

Commercial AI products and managed agent experiences are built separately on top of these foundations.

---

## What Is Included

This repository contains public developer foundations:

- Rust libraries
- authentication components
- event contracts
- capability models
- policy primitives
- health and telemetry foundations
- documentation
- examples

---

## What Is Not Included

Rhelma follows an open-core model.

The following remain outside the public core:

- hosted platform services
- commercial applications
- enterprise administration
- managed deployments
- billing systems
- customer integrations
- proprietary automation workflows
- production operations tooling

Products built on top of Rhelma Core may provide additional capabilities.

---

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

Explore:

- `crates/rhelma-core`
- `crates/rhelma-auth`
- `crates/rhelma-event`
- `crates/rhelma-capabilities`

---

## Contributing

Contributions are welcome.

Please read:

- CONTRIBUTING.md
- SECURITY.md
- OPEN_CORE_SCOPE.md

---

## License

See LICENSE for details.
