set shell := ["bash", "-cu"]

default:
  @just --list

fmt:
  cargo fmt --all

lint:
  cargo clippy --workspace --all-targets -- -D warnings

test:
  cargo test --workspace

build:
  cargo build --workspace

build-release:
  cargo build --workspace --release

# Static Linux x86_64 CLI. On Ubuntu/Debian: `sudo apt install musl-tools`, then `rustup target add x86_64-unknown-linux-musl`.
build-musl:
  CC=x86_64-linux-musl-gcc cargo build --release --target x86_64-unknown-linux-musl -p nexus-cli

build-musl-debug:
  CC=x86_64-linux-musl-gcc cargo build --target x86_64-unknown-linux-musl -p nexus-cli

# Linux hosts with musl-tools + the musl rustup target only.
test-musl:
  CC=x86_64-linux-musl-gcc CXX=x86_64-linux-musl-g++ \
    cargo test --workspace --target x86_64-unknown-linux-musl

nexus_bin := "./target/debug/nexus"

run-scan *args:
  cargo build -p nexus-cli
  {{nexus_bin}} scan {{args}}

run-plan *args:
  cargo build -p nexus-cli
  {{nexus_bin}} plan {{args}}

run-report *args:
  cargo build -p nexus-cli
  {{nexus_bin}} report {{args}}

doctor:
  cargo build -p nexus-cli
  {{nexus_bin}} doctor

apply-dry-run:
  cargo build -p nexus-cli
  {{nexus_bin}} apply --dry-run

serve port="3030":
  cargo build -p nexus-cli
  {{nexus_bin}} serve --port {{port}}

tools:
  cargo build -p nexus-cli
  {{nexus_bin}} tools
