# syntax=docker/dockerfile:1
# Multi-stage Dockerfile for distributed-zkml
# Supports both CPU and GPU builds

# =============================================================================
# Stage 1: Base image with system dependencies
# =============================================================================
FROM nvidia/cuda:12.2.0-devel-ubuntu22.04 AS base

# Prevent interactive prompts
ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    curl \
    git \
    pkg-config \
    libssl-dev \
    ca-certificates \
    python3.11 \
    python3.11-venv \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

# Make python3.11 the default
RUN update-alternatives --install /usr/bin/python3 python3 /usr/bin/python3.11 1 \
    && update-alternatives --install /usr/bin/python python /usr/bin/python3.11 1

# =============================================================================
# Stage 2: Rust toolchain
# =============================================================================
FROM base AS rust-builder

# Install Rust (version pinned via rust-toolchain.toml)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app

# Copy rust-toolchain.toml first to install correct version
COPY rust-toolchain.toml ./
RUN rustup show

# Copy Cargo files for dependency caching
COPY zkml/Cargo.toml zkml/Cargo.lock ./zkml/
COPY zkml/halo2 ./zkml/halo2/

# Create dummy src to cache dependencies
RUN mkdir -p zkml/src && echo "fn main() {}" > zkml/src/main.rs \
    && mkdir -p zkml/src/bin && echo "fn main() {}" > zkml/src/bin/prove_chunk.rs
WORKDIR /app/zkml
RUN cargo build --release || true
RUN rm -rf src

# Copy actual source
WORKDIR /app
COPY zkml ./zkml/

# Build release binary
WORKDIR /app/zkml
RUN cargo build --release

# =============================================================================
# Stage 3: Python environment
# =============================================================================
FROM base AS python-builder

# Install uv for fast Python package management (installs to ~/.local/bin)
RUN curl -LsSf https://astral.sh/uv/install.sh | sh
ENV PATH="/root/.local/bin:${PATH}"

WORKDIR /app

# Copy Python project files
COPY pyproject.toml uv.lock* ./

# Install Python dependencies
RUN uv sync --frozen || uv sync

# =============================================================================
# Stage 4: Final runtime image
# =============================================================================
FROM nvidia/cuda:12.2.0-runtime-ubuntu22.04 AS runtime

ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3.11 \
    python3.11-venv \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && update-alternatives --install /usr/bin/python3 python3 /usr/bin/python3.11 1 \
    && update-alternatives --install /usr/bin/python python /usr/bin/python3.11 1

WORKDIR /app

# Copy Rust binary from builder
COPY --from=rust-builder /app/zkml/target/release/prove_chunk /usr/local/bin/

# Copy Python environment from builder
COPY --from=python-builder /app/.venv /app/.venv
ENV PATH="/app/.venv/bin:${PATH}"
ENV VIRTUAL_ENV="/app/.venv"

# Copy application code
COPY python ./python/
COPY tests ./tests/
COPY zkml/examples ./zkml/examples/

# Set default command
CMD ["python", "-c", "print('distributed-zkml ready. Run: python tests/simple_distributed.py --help')"]

# =============================================================================
# Stage 5: Development image (includes full toolchain)
# =============================================================================
FROM rust-builder AS dev

# Install uv (installs to ~/.local/bin)
RUN curl -LsSf https://astral.sh/uv/install.sh | sh
ENV PATH="/root/.local/bin:${PATH}"

WORKDIR /app

# Set venv location outside /app so it's not overwritten by volume mounts
ENV UV_PROJECT_ENVIRONMENT="/opt/venv"
ENV VIRTUAL_ENV="/opt/venv"
ENV PATH="/opt/venv/bin:/root/.cargo/bin:/root/.local/bin:${PATH}"

# Copy Python project files and install
COPY pyproject.toml uv.lock* README.md ./
RUN uv sync --frozen || uv sync

# Copy all source
COPY . .

CMD ["/bin/bash"]

