# distributed-zkml

Distributed proving for zkml using Ray, layer-wise partitioning, and Merkle trees.

## Overview

This repository extends [zkml](https://github.com/uiuc-kang-lab/zkml) with distributed proving capabilities:
- **Layer-wise partitioning**: Split ML models into chunks for parallel proving
- **Merkle trees**: Preserve privacy of intermediate values between chunks
- **Ray integration**: Distributed execution across GPU workers

## Structure

```
distributed-zkml/
├── src/                    # Distributed proving code (this repo)
│   ├── distributed/        # Chunk execution and Merkle proofs
│   └── ray/                # Ray integration for batch inference
├── examples/               # Example scripts
│   └── simple_distributed.py
├── tests/                  # Tests
│   └── aws/                # AWS GPU tests
└── zkml/                   # zkml dependency (path dependency, includes our Merkle tree modifications)
```

**Note**: This is a separate Rust crate that extends zkml. The `zkml/` directory contains a modified version of zkml with our Merkle tree support.

## What's Been Done

### Merkle Tree Integration

- **Binary Merkle tree implementation** (`zkml/src/commitments/merkle.rs`)
  - Builds proper binary tree from intermediate values
  - Uses Poseidon hashing for efficient circuit operations
  
- **Chunk execution** (`zkml/src/layers/dag.rs`)
  - `forward_chunk()` - Execute layers in a range
  - `forward_chunk_with_merkle()` - Execute chunk and build Merkle tree
  
- **Tests** (`zkml/testing/`)
  - Merkle tree tests (3/3 pass)
  - Chunk execution tests (3/3 pass)

Status: ✅ Code compiles, all tests pass. Ready for integration with proof generation.

## Quick Start

### Build

```bash
# Ensure zkml is built first
cd zkml
rustup override set nightly
cargo build --release
cd ..

# Build distributed-zkml
cargo build
```

### Run Example

```bash
python3 examples/simple_distributed.py \
    --model zkml/examples/mnist/model.msgpack \
    --input zkml/examples/mnist/inp.msgpack \
    --layers 4 \
    --workers 2
```

## Testing on AWS GPUs (A100/H100)

### Prerequisites

Set AWS credentials (required):
```bash
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_SESSION_TOKEN=your_session_token
```

### Run Tests

```bash
python3 tests/aws/gpu_test.py
```

The test suite will:
1. Validate AWS credentials (warns if missing)
2. Check GPU availability via `nvidia-smi`
3. Setup Ray cluster with GPU support
4. Test GPU task distribution across workers
5. Run distributed proving simulation with Merkle trees

See `tests/aws/README.md` for detailed documentation.

## Testing

### Python Tests (pytest)
```bash
# Run all tests
pytest tests/

# Run specific test
pytest tests/aws/gpu_test.py::test_aws_credentials

# Run directly (without pytest)
python3 tests/aws/gpu_test.py
```

### Rust Tests (Cargo)
```bash
# Run Merkle tree tests (recommended)
cd zkml
cargo test --test merkle_tree_test

# Run chunk execution tests (recommended)
cargo test --test chunk_execution_test

# Run both test files
cargo test --test merkle_tree_test --test chunk_execution_test

# Run tests with output
cargo test --test merkle_tree_test --test chunk_execution_test -- --nocapture

# Check compilation only
cargo check --lib
```

**Note**: Broken example files have been moved to `zkml/examples/broken/` to prevent compilation errors. Always use `--test` flags when running tests.

**Test Files:**
- `zkml/testing/merkle_tree_test.rs` - Merkle tree tests (3 tests)
- `zkml/testing/chunk_execution_test.rs` - Chunk execution tests (3 tests)

## Next Steps

1. **Make Merkle root public**: Add root to public values so next chunk can verify it
2. **Complete proof generation**: Connect chunk execution to actual proof generation
3. **Ray-Rust integration**: Connect Python Ray workers to Rust proof generation
4. **GPU acceleration**: Current implementation is CPU-based. GPU acceleration for proof generation requires additional work (Halo2 GPU support or custom GPU kernels)

### GPU Performance Benchmarking

- Compare CPU vs GPU proving times
- Test with larger models (A100: 40GB, H100: 80GB)
- Measure throughput scaling with multiple GPUs

## Requirements

- Rust (nightly)
- Ray (Python)
- CUDA drivers (for GPU testing)
- Model files in msgpack format

## License

See LICENSE files in zkml subdirectory.
