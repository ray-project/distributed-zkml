# distributed-zkml

Extension of [zkml](https://github.com/uiuc-kang-lab/zkml) for distributed proving using Ray, layer-wise partitioning, and Merkle trees.

> **⚠️ Status Note:** This is an experimental research project. For production zkml, consider [zk-torch](https://github.com/uiuc-kang-lab/zk-torch) which uses proof folding for parallelization. See [Status and Limitations](#status-and-limitations) for details.

## Completed Milestones

1. ~~**Make Merkle root public**: Add root to public values so next chunk can verify it~~ Done
2. ~~**Complete proof generation**: Connect chunk execution to actual proof generation ([#8](https://github.com/ray-project/distributed-zkml/issues/8))~~ Done
3. ~~**Ray-Rust integration**: Connect Python Ray workers to Rust proof generation ([#9](https://github.com/ray-project/distributed-zkml/issues/9))~~ Done
4. ~~**GPU acceleration**: ICICLE GPU backend for MSM operations ([#10](https://github.com/ray-project/distributed-zkml/issues/10))~~ Done - see [GPU Acceleration](#gpu-acceleration)

---

## Table of Contents

- [Status and Limitations](#status-and-limitations)
- [Overview](#overview)
- [Implementation](#implementation)
- [Requirements](#requirements)
- [Quick Start](#quick-start)
- [GPU Acceleration](#gpu-acceleration)
- [Testing](#testing)
- [References](#references)

---

## Status and Limitations

### Project Status

This project implements a **Ray-based distributed proving approach** for zkml. It is experimental research code and should be considered useful for studying alternative approaches to zkML parallelization. The current status lacks formal security analysis and proof composition.

### Known Limitations

**Proof Composition**: This implementation generates separate proofs per chunk. It does not implement recursive proof composition or aggregation. Verifiers must check O(n) proofs rather than O(1), limiting succinctness.

**Security Assumptions**: The distributed trust model (Ray workers) is not formally analyzed. It does not address malicious worker resistance, collusion resistance, and Byzantine fault tolerance.

### When to Use This

**Consider this project if:**
- Researching alternative zkml parallelization approaches
- Need examples of Ray integration for cryptographic workloads
- Studying Merkle-based privacy for intermediate computations
- Building distributed halo2 proving (not zkML-specific)

---

## Overview

This repository extends zkml (see [ZKML paper](https://ddkang.github.io/papers/2024/zkml-eurosys.pdf)) with distributed proving capabilities. zkml provides an optimizing compiler from TensorFlow to halo2 ZK-SNARK circuits.

distributed-zkml adds:
- **Layer-wise partitioning**: Split ML models into chunks for parallel proving
- **Merkle trees**: Privacy-preserving commitments to intermediate values using Poseidon hashing
- **Ray integration**: Distributed execution across GPU workers

### Comparison to zkml

| Feature | zkml | distributed-zkml |
|---------|------|------------------|
| Architecture | Single-machine | Distributed across GPUs |
| Scalability | Single GPU memory | Horizontal scaling |
| Privacy | Outputs public | Intermediate values private via Merkle trees |

## Implementation

### How Distributed Proving Works

1. **Model Partitioning**: Split model into chunks at layer boundaries
2. **Parallel Execution**: Each chunk runs on a separate GPU via Ray
3. **Merkle Commitments**: Hash intermediate outputs with Poseidon, only root is public
4. **On-Chain**: Publish only the Merkle root (O(1) public values vs O(n) without)

**Note**: Each chunk produces a separate proof. This implementation does not aggregate proofs into a single succinct proof. Verifiers must check all chunk proofs individually (O(n) verification time). For single-proof aggregation, see [zk-orch](hhttps://github.com/uiuc-kang-lab/zk-torch)'s accumulation-based approach.

\`\`\`
Model: 9 layers -> 3 chunks
  Chunk 1: Layers 0-2 -> GPU 1 -> Hash A
  Chunk 2: Layers 3-5 -> GPU 2 -> Hash B
  Chunk 3: Layers 6-8 -> GPU 3 -> Hash C

Merkle Tree:
        Root (public)
       /    \\
    Hash(AB) Hash C
    /    \\
 Hash A  Hash B
\`\`\`

### Structure

\`\`\`
distributed-zkml/
├── python/                 # Python wrappers for Rust prover
├── tests/                  # Distributed proving tests
└── zkml/                   # zkml (modified for Merkle + chunking)
    ├── src/bin/prove_chunk.rs
    └── testing/
\`\`\`

## Requirements

### Docker (Recommended)

Just Docker and Docker Compose. Everything else is in the container.

### Native Build

| Dependency | Notes |
|------------|-------|
| Rust (nightly) | Install via [rustup](https://rustup.rs/) |
| Python >=3.10 | |
| pip | \`pip install -e .\` |
| Build tools | Linux: \`build-essential pkg-config libssl-dev\`; macOS: Xcode CLI |

**Python deps** (installed via \`pip install -e .\`):
- \`ray[default]>=2.31.0\`
- \`msgpack\`, \`numpy\`

**Optional**: NVIDIA GPU + CUDA 12.x + ICICLE backend for GPU acceleration

---

## Quick Start

### Docker

\`\`\`bash
docker compose build dev
docker compose run --rm dev
# Inside container:
cd zkml && cargo test --test merkle_tree_test -- --nocapture
\`\`\`

### Native

\`\`\`bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build
cd zkml && rustup override set nightly && cargo build --release && cd ..

# Python deps
pip install -e .
\`\`\`

---

## GPU Acceleration

Uses [ICICLE](https://github.com/ingonyama-zk/icicle) for GPU-accelerated MSM (Multi-Scalar Multiplication).

### Requirements

- NVIDIA GPU (tested on A10G/T4, compatible with A100/H100)
- CUDA 12.x
- Ubuntu 20.04+

### Setup

\`\`\`bash
# 1. Download ICICLE backend (Ubuntu 22.04 - use ubuntu20 for 20.04)
curl -L -o /tmp/icicle.tar.gz \\
  https://github.com/ingonyama-zk/icicle/releases/download/v3.1.0/icicle_3_1_0-ubuntu22-cuda122.tar.gz

# 2. Install
mkdir -p ~/.icicle && tar -xzf /tmp/icicle.tar.gz -C /tmp && cp -r /tmp/icicle/lib/backend ~/.icicle/

# 3. Set env var (add to ~/.bashrc)
export ICICLE_BACKEND_INSTALL_DIR=~/.icicle/backend

# 4. Build with GPU
cd zkml && cargo build --release --features gpu

# 5. Verify
cargo test --test gpu_benchmark_test --release --features gpu -- --nocapture
\`\`\`

Expected output:
\`\`\`
Registered devices: ["CUDA", "CPU"]
Successfully set CUDA device 0
\`\`\`

### Benchmarks (T4)

| Size | GPU MSM Time | Throughput |
|------|--------------|------------|
| 2^14 (16K) | 6.5ms | 2.5M pts/sec |
| 2^16 (65K) | 7.9ms | 8.3M pts/sec |
| 2^18 (262K) | 13ms | 19.5M pts/sec |

### FFT/NTT Notes

- **Measure FFT time**: \`HALO2_FFT_STATS=1\`
- **GPU NTT (experimental)**: \`HALO2_USE_GPU_NTT=1\` - currently slower due to conversion overhead

---

## Testing

### Distributed Proving

\`\`\`bash
# Simulation (fast)
python tests/simple_distributed.py \\
    --model zkml/examples/mnist/model.msgpack \\
    --input zkml/examples/mnist/inp.msgpack \\
    --layers 4 --workers 2

# Real proofs
python tests/simple_distributed.py ... --real
\`\`\`

### Rust Tests

\`\`\`bash
cd zkml
cargo test --test merkle_tree_test --test chunk_execution_test -- --nocapture
\`\`\`

### CI

Runs on PRs to \`main\`/\`dev\`: builds zkml, runs tests (~3-4 min). GPU tests excluded to save costs.

---

## References

- [ZKML Paper](https://ddkang.github.io/papers/2024/zkml-eurosys.pdf) (EuroSys '24) - Original zkml framework
- [zkml Repository](https://github.com/uiuc-kang-lab/zkml) - Base framework this project extends
- [zk-torch](https://github.com/uiuc-kang-lab/zk-torch) - Alternative approach using proof accumulation/folding (from same research group)
