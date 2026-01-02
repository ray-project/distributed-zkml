# distributed-zkml

Extension of [zkml](https://github.com/uiuc-kang-lab/zkml) for distributed proving using Ray, layer-wise partitioning, and Merkle trees.

> **Note:** This project is under active development. See [Next Steps](#next-steps) for current progress.

## Next Steps

1. ~~**Make Merkle root public**: Add root to public values so next chunk can verify it~~ Done
2. ~~**Complete proof generation**: Connect chunk execution to actual proof generation ([#8](https://github.com/ray-project/distributed-zkml/issues/8))~~ Done
3. ~~**Ray-Rust integration**: Connect Python Ray workers to Rust proof generation ([#9](https://github.com/ray-project/distributed-zkml/issues/9))~~ Done
4. **GPU acceleration**: ICICLE GPU backend integrated for MSM operations. See [GPU Acceleration](#gpu-acceleration) for setup. ([#10](https://github.com/ray-project/distributed-zkml/issues/10))

---

## Table of Contents

- [Overview](#overview)
- [Implementation](#implementation)
  - [How Distributed Proving Works](#how-distributed-proving-works)
  - [Structure](#structure)
- [Requirements](#requirements)
- [GPU Acceleration](#gpu-acceleration)
- [Quick Start](#quick-start)
- [Testing and CI](#testing-and-ci)
  - [CI](#ci)
- [References](#references)

---

## Overview

This repository extends zkml (see [ZKML: An Optimizing System for ML Inference in Zero-Knowledge Proofs](https://ddkang.github.io/papers/2024/zkml-eurosys.pdf)) with distributed proving capabilities. The zkml repository is included as a git submodule in the `zkml/` directory and modified to support Merkle tree commitments for intermediate layer outputs required in a distributed setting. zkml provides an optimizing compiler from TensorFlow to halo2 ZK-SNARK circuits for single-machine proof generation. High-stakes AI applications in biology or robotics are more practical with trustless verification using [ZKPs (Zero Knowledge Proofs](https://toc.csail.mit.edu/node/218), [SNARKs (Succient Non-interactive Arguments of Knowledge), and zk-SNARKs](https://cs251.stanford.edu/lectures/lecture15.pdf).

distributed-zkml adds:
- Layer-wise partitioning: Split ML models into chunks for parallel proving across multiple GPUs
- Merkle trees: Privacy-preserving commitments to intermediate values using Poseidon hashing
- Ray integration: Distributed execution across GPU workers for scalable proving

### Comparison to zkml

| Feature | zkml | distributed-zkml |
|---------|------|------------------|
| Architecture | Single-machine proving | Distributed proving across multiple GPUs |
| Scalability | Limited by single GPU memory | Horizontal scaling with multiple GPUs |
| Privacy | Model weights private, outputs public | Intermediate values also private via Merkle trees |
| Use Case | Small to medium models | Large models requiring distributed proving |
| Optimization | Circuit layout optimization | Layer partitioning + Merkle tree optimization |

The key difference: zkml optimizes circuit layout for a single proving instance, while distributed-zkml enables parallel proving of model chunks with privacy-preserving commitments to intermediate values.

## Implementation

### How Distributed Proving Works

#### Architecture

1. Model Layer Partitioning: Partition the ML model into chunks at the layer level (e.g., layers 0-2, 3-5, 6-8). Each chunk can execute on a separate GPU.

2. Parallel Chunk Execution: Each chunk executes its assigned layers on a GPU. Multiple chunks run in parallel across different GPUs using Ray for task distribution.

3. Merkle Tree for Privacy: Hash each chunk's intermediate outputs using Poseidon (efficient for ZK circuits). These hashes form a Merkle tree. Only the Merkle root is committed on-chain. Individual intermediate values remain private.

4. On-Chain Commitment: Publish only the Merkle root (a single hash) on-chain. This proves intermediate values were computed correctly without revealing their actual values.

#### Example Flow

```
Model: 9 layers total
Partition into 3 chunks:
  Chunk 1: Layers 0-2  → GPU 1 → Output A → Hash A
  Chunk 2: Layers 3-5  → GPU 2 → Output B → Hash B  
  Chunk 3: Layers 6-8  → GPU 3 → Output C → Hash C

Merkle Tree:
        Root (on-chain)
       /    \
    Hash(AB) Hash C
    /    \
 Hash A  Hash B

On-chain: Only the Root hash
Private: Outputs A, B, C (never revealed)
```

#### Why Merkle Trees?

Without Merkle trees, all intermediate values must be public for the next chunk to verify them—**O(n) public values**, which is expensive in ZK circuits.

With Merkle trees, only the root is public—**O(1) public values**. The next chunk verifies specific inputs with O(log n) Merkle proofs inside the circuit.

| Approach | Public Values | Verification Cost | Privacy |
|----------|---------------|-------------------|---------|
| No Merkle | O(n) | O(1) per value | All intermediate values exposed |
| Merkle | O(1) | O(log n) per value | Only root exposed |

### Structure

```
distributed-zkml/
├── python/                 # Python wrappers for Rust prover
│   └── rust_prover.py      # Python interface to prove_chunk CLI
├── tests/                  # Tests
│   ├── simple_distributed.py  # Distributed proving with Ray
│   └── aws/                # AWS GPU tests
└── zkml/                   # zkml (modified with Merkle tree + chunk proving)
    ├── src/bin/prove_chunk.rs  # CLI for chunk proof generation
    └── testing/            # Rust test suites
```

## Requirements

### Docker Setup (Recommended)

- **Docker** and **Docker Compose** only
- All other dependencies are included in the container image

### Native Build

**Required:**
- **Rust** (nightly toolchain) - Install via [rustup](https://rustup.rs/)
- **Python** (>=3.10, recommended 3.11-3.12)
  - macOS x86_64: Use Python 3.11 for Ray compatibility
- **uv** (recommended) or **pip** - Python package manager
- **System build tools**:
  - Linux: `build-essential`, `pkg-config`, `libssl-dev`
  - macOS: Xcode Command Line Tools (`xcode-select --install`)

**Python dependencies** (auto-installed via `uv sync` or `pip install -e .`):
- `ray[default]>=2.9.0,<2.11.0` - Constrained for macOS x86_64 compatibility
- `msgpack`, `numpy`

**Optional:**
- `pytest` - For running tests (dev dependencies)
- NVIDIA GPU + CUDA 12.x - For GPU-accelerated proving ops
- ICICLE backend - GPU MSM/NTT acceleration (see [GPU Acceleration](#gpu-acceleration))

### Quick Reference

| Tool | Docker | Native | Notes |
|------|--------|--------|-------|
| Docker | Required | - | Only for containerized workflow |
| Rust (nightly) | Included | Required | Builds zkml |
| Python (>=3.10) | Included | Required | 3.11 recommended on macOS x86_64 |
| uv/pip | Included | Required | Python package manager |
| Ray | Included | Required | <2.11.0 for macOS x86_64 |
| Build tools | Included | Required | System-specific |

---

---

## GPU Acceleration

GPU acceleration uses [ICICLE](https://github.com/ingonyama-zk/icicle) for GPU-accelerated MSM (Multi-Scalar Multiplication) operations.

### GPU Requirements

- NVIDIA GPU (tested on A10G, compatible with A100/H100)
- CUDA 12.x drivers
- Ubuntu 20.04+ (Ubuntu 22.04 recommended)

### GPU Setup

1. **Download ICICLE backend** (match your Ubuntu version):

```bash
# Ubuntu 22.04
curl -L -o /tmp/icicle.tar.gz \\
  https://github.com/ingonyama-zk/icicle/releases/download/v3.1.0/icicle_3_1_0-ubuntu22-cuda122.tar.gz

# Ubuntu 20.04
curl -L -o /tmp/icicle.tar.gz \\
  https://github.com/ingonyama-zk/icicle/releases/download/v3.1.0/icicle_3_1_0-ubuntu20-cuda122.tar.gz
```

2. **Install backend**:

```bash
mkdir -p ~/.icicle
tar -xzf /tmp/icicle.tar.gz -C /tmp
cp -r /tmp/icicle/lib/backend ~/.icicle/
```

3. **Set environment variable** (add to ~/.bashrc):

```bash
export ICICLE_BACKEND_INSTALL_DIR=~/.icicle/backend
```

4. **Build with GPU support**:

```bash
cd zkml
cargo build --release --features gpu
```

5. **Verify GPU detection**:

```bash
ICICLE_BACKEND_INSTALL_DIR=~/.icicle/backend \\
  cargo test --test gpu_benchmark_test --release --features gpu -- --nocapture
```

Expected output:
```
Registered devices: ["CUDA", "CPU"]
Successfully set CUDA device 0
```

### Benchmark Results

Tested on 4x NVIDIA A10G (23GB each):

| Operation | Size | Time | Throughput |
|-----------|------|------|------------|
| GPU MSM | 2^12 (4K points) | 15ms | 260K pts/sec |
| GPU MSM | 2^14 (16K points) | 6.5ms | 2.5M pts/sec |
| GPU MSM | 2^16 (65K points) | 7.9ms | 8.3M pts/sec |
| GPU MSM | 2^18 (262K points) | 13ms | 19.5M pts/sec |


### FFT / NTT (how it’s used here)

Halo2 proving does a lot of polynomial work, and that uses FFTs. Over a finite field it’s usually called an NTT, but it’s the same “fast polynomial transform” idea. In this repo, a big chunk of proving time is from these FFT/NTT calls.

- **Measure it**: set `HALO2_FFT_STATS=1` (our proof test prints totals + call counts).
- **GPU NTT (experimental)**: `HALO2_USE_GPU_NTT=1` turns on an ICICLE NTT path for BN256 `Fr`. It’s currently not faster due to conversion overhead, so it stays opt-in.

---

## Quick Start

### Option 1: Docker (Recommended)

```bash
# Build the development image
docker compose build dev

# Run interactive shell
docker compose run --rm dev

# Inside container: run tests
cd zkml && cargo test --test merkle_tree_test --test chunk_execution_test -- --nocapture
```

### Option 2: Native Build

```bash
# 1. Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 2. Build zkml
cd zkml
rustup override set nightly
cargo build --release
cd ..

# 3. Install Python dependencies
uv sync  # or: pip install -e .
```

### Run Distributed Proving

```bash
# Simulation mode (fast, no actual proofs)
python3 tests/simple_distributed.py \
    --model zkml/examples/mnist/model.msgpack \
    --input zkml/examples/mnist/inp.msgpack \
    --layers 4 \
    --workers 2

# Real mode (generates actual ZK proofs, ~2-3s per chunk)
python3 tests/simple_distributed.py \
    --model zkml/examples/mnist/model.msgpack \
    --input zkml/examples/mnist/inp.msgpack \
    --layers 4 \
    --workers 2 \
    --real
```

## Testing and CI

### Distributed Proving Test

Test the distributed proving pipeline:

```bash
# Simulation mode (fast, no real proofs)
python tests/simple_distributed.py \
    --model zkml/examples/mnist/model.msgpack \
    --input zkml/examples/mnist/inp.msgpack \
    --layers 4 --workers 2

# Real mode (generates actual ZK proofs)
python tests/simple_distributed.py \
    --model zkml/examples/mnist/model.msgpack \
    --input zkml/examples/mnist/inp.msgpack \
    --layers 4 --workers 2 --real
```

### Rust Tests

```bash
cd zkml

# Run all tests
cargo test --test merkle_tree_test --test chunk_execution_test --test test_merkle_root_public -- --nocapture

# Run specific test
cargo test --test merkle_tree_test -- --nocapture
```

### CI

Lightweight CI runs on every PR to `main` and `dev`:
- Builds zkml library (nightly Rust)
- Runs `zkml/testing/` tests (~3-4 min total)
- AWS/GPU tests excluded to save costs

## References

- zkml Paper: [ZKML: An Optimizing System for ML Inference in Zero-Knowledge Proofs](https://ddkang.github.io/papers/2024/zkml-eurosys.pdf) (EuroSys '24)
  - Original zkml framework for single-machine ZK-SNARK generation
  - Circuit layout optimization and gadget design
  - Supports realistic ML models including vision models and DistillGPT-2

- zkml Repository: [uiuc-kang-lab/zkml](https://github.com/uiuc-kang-lab/zkml)
  - Source code for the original zkml framework
  - TensorFlow to halo2 compiler

## License

See LICENSE files in zkml subdirectory.
