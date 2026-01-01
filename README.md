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
  - [How Distributed Proving Works](#how-distributed-proving-works)
  - [Structure](#structure)
- [Quick Start](#quick-start)
- [Testing and CI](#testing-and-ci)
  - [General](#general)
  - [Testing on AWS GPU Instances](#testing-on-aws-gpu-instances)
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

## Quick Start

### Option 1: Docker (Recommended)

```bash
# Build the development image
docker compose build dev

# Run interactive shell
docker compose run --rm dev

# Run tests
docker compose run --rm test
```

### Option 2: Native Build

```bash
# Ensure zkml is built first
cd zkml
rustup override set nightly
cargo build --release
cd ..

# Install Python dependencies
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

### General

#### Python Tests (pytest)

##### Run All Tests
```bash
pytest tests/
```

##### Run specific GPU and AWS tests
```bash
pytest tests/aws/gpu_test.py
pytest tests/aws/gpu_test.py::test_aws_credentials
```

#### Rust Tests (Cargo)

##### Run All Tests in zkml
```bash
cd zkml
# Run only the test files (recommended)
cargo test --test merkle_tree_test --test chunk_execution_test

# Note: Running `cargo test` without --test flags will try to compile examples,
# some of which may have errors. Use --test flags to run specific tests.
```

##### Run Specific Test File
```bash
cd zkml
cargo test --test merkle_tree_test
cargo test --test chunk_execution_test
```

##### Run Tests with Output
```bash
cd zkml
cargo test --test merkle_tree_test --test chunk_execution_test -- --nocapture
```

##### Run Tests for distributed-zkml Crate
```bash
# From distributed-zkml root
cargo test
```

##### Check Compilation Only
```bash
cd zkml
cargo check --lib
```

Broken example files are moved to `zkml/examples/broken/` to prevent compilation errors. Use `--test` flags when running tests.

#### Test Files

##### Python Tests
- `tests/aws/gpu_test.py` - AWS GPU tests
  - `test_aws_credentials()` - Check AWS credentials
  - `test_gpu_availability()` - Check GPU availability
  - `test_ray_setup()` - Test Ray cluster setup

##### Rust Tests
- `zkml/testing/merkle_tree_test.rs` - Merkle tree tests
  - `test_merkle_single_value()` - Single value Merkle tree
  - `test_merkle_multiple_values()` - Multiple values Merkle tree
  - `test_merkle_root_verification()` - Root verification

- `zkml/testing/chunk_execution_test.rs` - Chunk execution tests
  - `test_chunk_execution_intermediate_values()` - Extract intermediate values
  - `test_chunk_execution_with_merkle()` - Chunk execution with Merkle
  - `test_multiple_chunks_consistency()` - Multiple chunks consistency

### Testing on AWS GPU Instances

#### Prerequisites

##### AWS Credentials

Set the following environment variables:

```bash
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_SESSION_TOKEN=your_session_token
```

##### AWS Resource Configuration

**Option 1: Automated Setup (Recommended)**

Run the setup script to automatically get/create resources:

```bash
# Optional: Set custom resource names for auto-detection
export AWS_KEY_NAME=your-key-name              # Optional: for auto-detection
export AWS_SECURITY_GROUP_NAME=your-sg-name    # Optional: for auto-detection

# Run setup script (will prompt or create resources)
./tests/aws/setup_aws_resources.sh

# Copy the export commands it shows, then set:
export KEY_NAME=your-key-name                  # Required: from setup script
export SECURITY_GROUP=sg-xxxxx                 # Required: from setup script
```

**Option 2: Manual Configuration**

Set all required variables manually:

```bash
# Required: AWS credentials
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_SESSION_TOKEN=your_session_token

# Required: Resource identifiers
export KEY_NAME=your-key-name                  # Your EC2 key pair name
export SECURITY_GROUP=sg-xxxxx                # Your security group ID

# Optional: Override defaults
export AWS_REGION=us-west-2                    # Default: us-west-2
export INSTANCE_TYPE=g5.xlarge                 # Default: g5.xlarge
export AMI_ID=ami-0076e7fffffc9251d           # Default: Ubuntu 20.04, PyTorch 2.3.1
```

##### GPU Instance

Launch an AWS instance with GPU support:
- A100: `g5.xlarge` or larger (1x A100)
- H100: `p5.48xlarge` (8x H100)

##### Dependencies

```bash
# Install Rust (nightly)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup override set nightly

# Install Python dependencies
pip install ray torch

# Install CUDA drivers (usually pre-installed on GPU instances)
nvidia-smi  # Verify GPU is available
```

#### Test Suite

The test suite includes:

1. AWS Credentials Check: Validates required environment variables
2. GPU Availability Check: Verifies GPU is accessible via `nvidia-smi`
3. Ray Cluster Setup: Initializes Ray with GPU support
4. Basic GPU Distribution: Tests task distribution across GPU workers
5. Distributed Proving Simulation: Runs distributed proving with Merkle trees

#### Expected Output

```
============================================================
AWS GPU Tests for Distributed Proving
============================================================
INFO: AWS credentials found
INFO: GPU detected
INFO: Ray initialized with 1 GPU(s)

--- Running: Basic GPU Distribution ---
INFO: Testing GPU distribution with 2 workers
INFO: Completed 4 tasks
INFO: Task 0: Worker 0, GPU 0, Time: 2.34ms
...

--- Running: Distributed Proving Simulation ---
INFO: Testing distributed proving simulation
INFO: Distributed proving completed: 2 chunks
INFO: Chunk 0: success
INFO: Chunk 1: success

============================================================
Test Summary
============================================================
Basic GPU Distribution: PASS
Distributed Proving Simulation: PASS
```

#### Performance Notes

- A100: ~40GB VRAM, suitable for large models
- H100: ~80GB VRAM, suitable for very large models
- Ray automatically distributes tasks across available GPUs
- Monitor GPU usage: `watch -n 1 nvidia-smi`

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
