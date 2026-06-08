# Service README standard

To keep `apps/*/README.md` consistent and easy to operate, each service README should include:

1) **Overview** (1 paragraph)
2) **Run (local)** (`cargo run -p <name>` or node command)
3) **Configuration** (link to `.env.example` + `docs/reference/ENVIRONMENT_VARIABLES.md`)
4) **Endpoints** (health, metrics, main API)
5) **Observability** (metrics/tracing/logs)
6) **Security / policy notes**
7) **Verification** (which scripts/tests enforce correctness)

Template: `docs/_templates/app_readme_template.md`
