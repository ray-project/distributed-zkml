# Testing Guide

## Python Tests (pytest)

### Run All Tests
```bash
pytest tests/
```

### Run Specific Test File
```bash
pytest tests/aws/gpu_test.py
```

### Run Specific Test
```bash
pytest tests/aws/gpu_test.py::test_aws_credentials
```

### Run with Verbose Output
```bash
pytest tests/ -v
```

### Run Directly (without pytest)
```bash
python3 tests/aws/gpu_test.py
```

## Rust Tests (Cargo)

### Run All Tests in zkml
```bash
cd zkml
# Run only the test files (recommended)
cargo test --test merkle_tree_test --test chunk_execution_test

# Note: Running `cargo test` without --test flags will try to compile examples,
# some of which may have errors. Use --test flags to run specific tests.
```

### Run Specific Test File
```bash
cd zkml
cargo test --test merkle_tree_test
cargo test --test chunk_execution_test
```

### Run Tests with Output
```bash
cd zkml
cargo test -- --nocapture
```

### Run Tests for distributed-zkml Crate
```bash
# From distributed-zkml root
cargo test
```

### Check Compilation Only
```bash
cd zkml
cargo check --lib
```

## Test Files

### Python Tests
- `tests/aws/gpu_test.py` - AWS GPU tests
  - `test_aws_credentials()` - Check AWS credentials
  - `test_gpu_availability()` - Check GPU availability
  - `test_ray_setup()` - Test Ray cluster setup

### Rust Tests
- `zkml/testing/merkle_tree_test.rs` - Merkle tree tests
  - `test_merkle_single_value()` - Single value Merkle tree
  - `test_merkle_multiple_values()` - Multiple values Merkle tree
  - `test_merkle_root_verification()` - Root verification

- `zkml/testing/chunk_execution_test.rs` - Chunk execution tests
  - `test_chunk_execution_intermediate_values()` - Extract intermediate values
  - `test_chunk_execution_with_merkle()` - Chunk execution with Merkle
  - `test_multiple_chunks_consistency()` - Multiple chunks consistency

## Troubleshooting

### Python Tests Hang
- Make sure Ray is not imported at module level
- Check if Ray is installed: `pip install ray`
- Run with timeout: `pytest tests/ --timeout=10`

### Rust Tests Don't Run
- Make sure Rust is installed: `rustc --version`
- Add Cargo to PATH: `export PATH="$HOME/.cargo/bin:$PATH"`
- Use nightly Rust: `rustup override set nightly` (in zkml directory)

### Tests Fail
- Check dependencies are installed
- Verify environment variables are set (for AWS tests)
- Check GPU availability (for GPU tests)

