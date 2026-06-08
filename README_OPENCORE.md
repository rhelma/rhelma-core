# 🚀 Rhelma: Open Core Platform

**A distributed, AI-native platform for building scalable systems.**

Rhelma is a production-ready Rust framework for:
- 🏗️ **Distributed systems** — Multi-tenant, multi-region architecture
- 🔒 **Zero-trust security** — Defense-in-depth by design
- 🤖 **AI-native workflows** — AI orchestration and attestation
- 📊 **Observability-first** — Logs, traces, metrics built in
- ⚡ **Event-driven operations** — Kafka/NATS ready

**This is the open-source core.** Use it to build, self-host, and learn. For managed operations and enterprise features, see [COMMERCIAL_FEATURES.md](COMMERCIAL_FEATURES.md).

---

## ⚡ Quick Start

### Prerequisites
- Rust 1.70+ (`rustc --version`)
- Docker & Docker Compose
- PostgreSQL 14+
- Redis 7+

### 1. Clone & Setup
```bash
git clone https://github.com/rhelma/rhelma-open-core.git
cd rhelma-open-core

# Copy default env
cp .env.example .env

# Bootstrap dependencies
bash scripts/bootstrap.sh
```

### 2. Start Local Dev Stack
```bash
# Spin up postgres, redis, local services
bash scripts/run-world.sh
```

### 3. Run a Service
```bash
# API Gateway
cargo run -p api-gateway

# Social Service
cargo run -p social-service

# Search Service
cargo run -p search-service
```

### 4. Test It
```bash
# List available crates
cargo workspaces list

# Run full test suite
cargo test --all

# Run clippy linter
cargo clippy --all -- -D warnings
```

📖 **Full setup guide:** [`docs/getting-started/LOCAL_DEV_STACK.md`](docs/getting-started/LOCAL_DEV_STACK.md)

---

## 📚 What's Included

### Foundation Crates (16)
Reusable, production-grade Rust libraries:
- `rhelma-core` — Domain models and abstractions
- `rhelma-auth` — Authentication layer
- `rhelma-db` — Database abstraction (PostgreSQL)
- `rhelma-cache` — Caching layer (Redis)
- `rhelma-event` — Event system (Kafka, NATS)
- `rhelma-tracing` — Observability (OTEL)
- `rhelma-attestation` — AI model attestation
- And 8 more core libraries

**Use as dependencies:**
```toml
[dependencies]
rhelma-core = "0.1"
rhelma-auth = "0.1"
rhelma-db = "0.1"
```

Published on [crates.io](https://crates.io/search?q=rhelma)

### Reference Services (8)
Production-ready microservices:
- **api-gateway** — REST API entry point
- **social-service** — Posts, follows, comments
- **search-service** — Full-text search
- **realtime-service** — WebSocket, pub/sub
- **file-storage** — Upload, download, CDN
- **node-registry** — Service discovery
- **rhelma-attestation-verifier** — AI model verification
- **sandbox-runner** — Safe code execution

Run locally with Docker Compose or deploy anywhere.

### SDKs (3)
- **JavaScript/TypeScript** — Node.js, browser
- **Python** — Sync & async APIs
- **Go** — Standard Go idioms

```python
from rhelma import Client

client = Client(url="http://localhost:8000")
posts = client.social.list_posts()
```

### Documentation
- 📖 **[docs/README.md](docs/README.md)** — Full documentation index
- 🏗️ **[docs/architecture/](docs/architecture/)** — Design decisions
- 🛠️ **[docs/getting-started/](docs/getting-started/)** — Setup guides
- 🔌 **[docs/contract/v6.0/](docs/contract/v6.0/)** — API contracts
- 📝 **[ROADMAP.md](ROADMAP.md)** — Future milestones

---

## 🎯 Architecture Highlights

### Multi-Tenant, Multi-Region
```
┌─────────────────────────────────────┐
│      API Gateway                    │
│  (Request routing, auth, rate limit)│
└──────────┬──────────────────────────┘
           │
      ┌────┼────┐
      │         │
┌─────┴────┐ ┌─┴──────────┐
│ Social   │ │ Search     │
│ Service  │ │ Service    │
└──────────┘ └────────────┘
      │         │
   ┌──┴─────────┴──┐
   │  Event Bus    │
   │  (Kafka/NATS) │
   └───────────────┘
      │
   ┌──┴──────────────┐
   │  Data Layer     │
   │ (DB + Cache)    │
   └─────────────────┘
```

### Zero-Trust Security
- All services authenticate via JWT
- Mutual TLS (mTLS) for service-to-service
- API tokens for external clients
- Audit logging of all operations

### Observability Built-In
- Structured logging (JSON)
- Distributed tracing (OpenTelemetry)
- Prometheus metrics
- Real-time dashboard integration

### Event-Driven
- Service decoupling via events
- Event sourcing support
- Kafka / NATS integration
- Webhook support

---

## 🧪 Testing

```bash
# Run all tests
cargo test --all

# Run tests in a specific crate
cargo test -p rhelma-core

# Run tests with logs
RUST_LOG=debug cargo test -- --nocapture

# Integration tests
cargo test --test '*' --all

# Bench
cargo bench --all
```

---

## 📦 Using as a Library

Add Rhelma crates to your `Cargo.toml`:

```toml
[dependencies]
rhelma-core = "0.1"
rhelma-auth = "0.1"
rhelma-db = "0.1"
tokio = { version = "1", features = ["full"] }
```

Example:
```rust
use rhelma_core::prelude::*;
use rhelma_db::pool::create_pool;

#[tokio::main]
async fn main() -> Result<()> {
    let pool = create_pool("postgres://...").await?;
    
    // Use pool for queries
    println!("Connected!");
    Ok(())
}
```

---

## 🤝 Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for:
- Code style and conventions
- Pull request process
- Testing requirements
- Documentation standards

**Quick start for contributors:**
```bash
git checkout -b fix/my-issue
# Make your changes
cargo test --all
cargo fmt
cargo clippy

git push origin fix/my-issue
# Open a PR
```

---

## 🔒 Security

For security concerns:
- 📋 See [SECURITY.md](SECURITY.md)
- 📧 Email: security@rhelma.ir
- 🐛 **Do not** open public security issues

---

## 📋 License

**MIT OR Apache-2.0**

Choose whichever license works best for your project.

---

## 💬 Community

- 💻 **GitHub:** [github.com/rhelma/rhelma-open-core](https://github.com/rhelma/rhelma-open-core)
- 📖 **Docs:** [docs.rhelma.ir](https://docs.rhelma.ir)
- 💬 **Discussions:** GitHub Discussions
- 📧 **Email:** hello@rhelma.ir
- 🐦 **Twitter:** [@rhelmaio](https://twitter.com/rhelmaio)

---

## 🎁 What About Enterprise?

Need:
- ☁️ Hosted SaaS with 99.99% SLA?
- 👥 Advanced admin dashboard?
- 🔗 Enterprise integrations (SAML, LDAP, custom)?
- ✅ Compliance tools (SOC2, GDPR, HIPAA)?
- 📞 24/7 priority support?

👉 Check out [COMMERCIAL_FEATURES.md](COMMERCIAL_FEATURES.md) or [contact sales](mailto:enterprise@rhelma.ir)

---

## 🚀 Roadmap

**v0.1** (Current)
- ✅ Foundation crates
- ✅ Core services
- ✅ SDKs (JS, Python, Go)

**v0.2** (Next)
- 📅 GraphQL API
- 📅 Streaming webhooks
- 📅 Plugin system

**v1.0** (Future)
- 📅 Complete module stability
- 📅 Production SLA
- 📅 Commercial Rhelma Enterprise

See [ROADMAP.md](ROADMAP.md) for details.

---

## 📊 Project Status

| Component | Status | Notes |
|-----------|--------|-------|
| Core crates | ✅ Stable | Production-ready |
| Services | ✅ Stable | Reference implementations |
| SDKs | ✅ Stable | Full API coverage |
| Documentation | ✅ Complete | Architecture & guides |
| Security | ✅ Audited | Annual security review |
| Compliance | ⏳ Partial | SOC2 in progress for Enterprise |

---

**Ready to build?** Start with [`docs/getting-started/QUICKSTART_MVP.md`](docs/getting-started/QUICKSTART_MVP.md)

**Questions?** Check [`docs/INDEX.md`](docs/INDEX.md) or open an issue.

---

**Made with ❤️ by the Rhelma community**

Open source core. Enterprise capabilities available.
