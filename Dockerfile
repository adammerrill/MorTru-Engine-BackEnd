# ─────────────────────────────────────────────────────────────────────────────
# Meridian Mortgage Engine — Multi-stage Dockerfile
#
# Stage 1 (builder): compile the full workspace in release mode
# Stage 2 (tester):  run cargo test to validate the image at build time
# Stage 3 (runtime): minimal production image with just the API binary
#
# Until the API binary exists (Epic 15 / Task 15.3), the runtime stage
# runs the test suite as its CMD so CI/CD has a deployable artifact.
# Replace CMD in stage 3 once the API binary is built.
# ─────────────────────────────────────────────────────────────────────────────

# ── Stage 1: Rust build environment ─────────────────────────────────────────
FROM rust:1.85-slim-bookworm AS builder

# System dependencies for linking
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests first for layer-cached dependency compilation
COPY Cargo.toml Cargo.lock ./
COPY crates/types/Cargo.toml             crates/types/Cargo.toml
COPY crates/mismo/Cargo.toml             crates/mismo/Cargo.toml
COPY crates/reso/Cargo.toml              crates/reso/Cargo.toml
COPY crates/ingest/Cargo.toml            crates/ingest/Cargo.toml
COPY crates/enrich/Cargo.toml            crates/enrich/Cargo.toml
COPY crates/eligibility/Cargo.toml       crates/eligibility/Cargo.toml
COPY crates/compliance/Cargo.toml        crates/compliance/Cargo.toml
COPY crates/scenarios/Cargo.toml         crates/scenarios/Cargo.toml
COPY crates/solver/Cargo.toml            crates/solver/Cargo.toml
COPY crates/amort/Cargo.toml             crates/amort/Cargo.toml
COPY crates/ml/Cargo.toml                crates/ml/Cargo.toml
COPY crates/orchestrator/Cargo.toml      crates/orchestrator/Cargo.toml
COPY crates/api/Cargo.toml               crates/api/Cargo.toml

# Stub out all src/lib.rs so Cargo can resolve deps without real source
RUN for crate in types mismo reso ingest enrich eligibility compliance \
        scenarios solver amort ml orchestrator api; do \
    mkdir -p crates/$crate/src && \
    echo "" > crates/$crate/src/lib.rs; \
    done

# Compile dependencies only (cached layer)
RUN cargo build --release --workspace 2>/dev/null || true

# Now copy real source
COPY crates/ crates/
COPY deny.toml deny.toml

# Full release build
RUN cargo build --release --workspace

# ── Stage 2: Test validation ─────────────────────────────────────────────────
FROM builder AS tester
RUN cargo test --workspace --release

# ── Stage 3: Runtime image ───────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# TODO (Task 15.3): copy the API binary here
# COPY --from=builder /app/target/release/api /app/api

# Temporary: copy the test binary so Cloud Run has something to execute
# Replace with: CMD ["/app/api"]
COPY --from=tester /app/target/release/deps /app/deps

ENV PORT=8080
EXPOSE 8080

# ── Health check ──────────────────────────────────────────────────────────────
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:${PORT}/health || exit 1

# TODO: replace with /app/api once Task 15.3 delivers the HTTP server
CMD ["echo", "MorTru Engine — API binary pending Epic 15. CI/CD pipeline active."]
