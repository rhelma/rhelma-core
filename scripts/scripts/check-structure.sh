#!/bin/bash

echo "🔍 Checking project structure..."

# Check for required directories
directories=("apps" "crates" "infra" "packages")
for dir in "${directories[@]}"; do
    if [ ! -d "$dir" ]; then
        echo "❌ Missing directory: $dir"
        exit 1
    fi
done

# Check Cargo.toml files
cargo_tomls=("Cargo.toml" "crates/rhelma-core/Cargo.toml" "crates/rhelma-auth/Cargo.toml")
for file in "${cargo_tomls[@]}"; do
    if [ ! -f "$file" ]; then
        echo "❌ Missing file: $file"
        exit 1
    fi
done

# Check for main applications
apps=("api-gateway" "search-service" "ai-orchestrator")
for app in "${apps[@]}"; do
    if [ ! -d "apps/$app" ]; then
        echo "❌ Missing app: $app"
        exit 1
    fi
done

echo "✅ Project structure is valid!"