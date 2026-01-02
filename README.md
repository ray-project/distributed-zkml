# distributed-zkml

Extension of [zkml](https://github.com/uiuc-kang-lab/zkml) for distributed proving using Ray, layer-wise partitioning, and Merkle trees.

> **Note:** This project is under active development. See [Next Steps](#next-steps) for current progress.

## Next Steps

1. ~~**Make Merkle root public**: Add root to public values so next chunk can verify it~~ Done
2. ~~**Complete proof generation**: Connect chunk execution to actual proof generation ([#8](https://github.com/ray-project/distributed-zkml/issues/8))~~ Done
3. ~~**Ray-Rust integration**: Connect Python Ray workers to Rust proof generation ([#9](https://github.com/ray-project/distributed-zkml/issues/9))~~ Done
4. **GPU acceleration**: Current implementation is CPU-based. GPU acceleration for proof generation requires additional work ([#10](https://github.com/ray-project/distributed-zkml/issues/10))

---

## Table of Contents

- [Overview](#overview)
- [Implementation](#implementation)
- [Requirements](#requirements)
- [Quick Start](#quick-start)
- [Testing and CI](#testing-and-ci)
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

- **Rust** (nightly) - Install via [rustup](https://rustup.rs/)
- **Python** (>=3.10) - 3.11 recommended for macOS x86_64
- **uv** or **pip** - Python package manager

Optional:
- **Docker** - For containerized builds (CI uses this)
- **CUDA** - For GPU-accelerated proving

---

## Quick Start

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

### Docker Alternative

```bash
docker compose build dev
docker compose run --rm dev
# Inside container: cd zkml && cargo test --test merkle_tree_test -- --nocapture
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
