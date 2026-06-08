#!/usr/bin/env bash
# scripts/dev.sh
# Rhelma Development Environment – One command to rule them all

set -euo pipefail

# رنگ‌های قشنگ برای خروجی
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() { echo -e "${BLUE}➤${NC} $1"; }
success() { echo -e "${GREEN}✓ $1${NC}"; }
warn() { echo -e "${YELLOW}⚠ $1${NC}"; }
error() { echo -e "${RED}✗ $1${NC}"; exit 1; }

log "Rhelma is waking up..."

# Best-effort: load repo-level .env so the central identity contract is always satisfied.
if [[ -f ".env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source ".env"
  set +a
fi

# 1. مطمئن شو که همه ابزارها نصب هستن
command -v docker >/dev/null || error "Docker is required but not installed"
command -v pnpm >/dev/null || error "pnpm is required → pnpm install -g pnpm"
command -v turbo >/dev/null || error "turbo is required → pnpm add -g turbo"
command -v wrangler >/dev/null || warn "wrangler not found → edge dev will be slower"

# 2. محیط رو تشخیص بده (git branch یا RHELMA_ENV)
export RHELMA_ENV=${RHELMA_ENV:-$(git rev-parse --abbrev-ref HEAD 2>/dev/null | sed 's|/.*||' || echo "development")}
export RHELMA_ENV=${RHELMA_ENV#"heads/"}
[[ "$RHELMA_ENV" == "main" || "$RHELMA_ENV" == "master" ]] && RHELMA_ENV="production"
export RHELMA_ENV=$(echo "$RHELMA_ENV" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9]/-/g')

# CentralEnv strict fields (required by most Rust services). Keep defaults safe for local dev.
export RHELMA_ENVIRONMENT="${RHELMA_ENVIRONMENT:-$RHELMA_ENV}"
export RHELMA_REGION="${RHELMA_REGION:-local}"
export RHELMA_SERVICE_VERSION="${RHELMA_SERVICE_VERSION:-0.0.0-dev}"

log "Environment: ${GREEN}$RHELMA_ENV${NC}"

# 3. دیتابیس و سرویس‌های زیرساختی رو بالا بیار (با docker-compose)
if [[ -f "docker-compose.dev.yml" ]] || [[ -f "docker-compose.yaml" ]]; then
  log "Starting infrastructure (PostgreSQL, Redis, Redpanda, Qdrant, MinIO...)"
  docker compose -f docker-compose.dev.yml up -d --remove-orphans || error "Failed to start infra"
  success "Infrastructure is up"
else
  warn "No docker-compose.dev.yml found → skipping infra"
fi

# 4. پروتوباف‌ها رو generate کن (فقط اگر تغییر کردن)
if [[ -d "packages/contracts" ]]; then
  log "Generating protobuf types..."
  pnpm --filter contracts proto:generate || error "Proto generation failed"
  success "Protobuf types generated"
fi

# 5. مهاجرت دیتابیس (sqlx migrate) - best-effort
#
# Note: api-gateway also supports auto-migrate in dev via RHELMA_DB__AUTO_MIGRATE=1.
if command -v sqlx >/dev/null; then
  if [[ -d "apps/api-gateway/migrations" ]]; then
    log "Running database migrations (apps/api-gateway)..."
    (cd apps/api-gateway && sqlx migrate run) || warn "Migration failed (continuing anyway)"
  elif [[ -d "crates/rhelma-db/migrations" ]]; then
    log "Running database migrations (crates/rhelma-db)..."
    sqlx migrate run --source crates/rhelma-db/migrations || warn "Migration failed (continuing anyway)"
  fi
fi

# 6. همه سرویس‌ها رو با turbo بالا بیار (parallel + hot reload)
log "Starting all services in development mode..."
log "Open http://localhost:3000 (API) | http://localhost:5173 (Web) | http://localhost:8080 (Search)"

# ترفند جادویی: اگر wrangler نصب باشه → edge worker رو local preview می‌کنه
if command -v wrangler >/dev/null; then
  turbo run dev --parallel &
  TURBO_PID=$!
  (cd apps/edge-worker && wrangler dev --port 8787 --local) &
  WRANGLER_PID=$!
  wait $TURBO_PID $WRANGLER_PID
else
  turbo run dev --parallel
fi

success "Rhelma is running! Happy coding!"
echo -e "${GREEN}
   ___  ___ ___ ___ 
  |  _|/ __/ __/ __|
  | |__| (__\__ \__ \\
  |___|\___|___/___/ ${YELLOW}dev mode${NC}

  Web:      http://localhost:5173
  API:      http://localhost:3000
  Edge:     http://localhost:8787 (if wrangler)
  Docs:     http://localhost:3000/docs
${NC}"
