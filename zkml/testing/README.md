# zkml Testing

## Overview

This directory contains unit and integration tests for zkml's core functionality.

**These tests run in CI on every PR** - they are fast (~3-4 seconds) and don't require external resources.

## Test Files

| File | Status | Description |
|------|--------|-------------|
| `test_merkle_root_public.rs` | ✅ Working | Verifies Merkle root is added to public values |
| `merkle_tree_test.rs` | ⚠️ Placeholder | Basic Merkle tree operations |
| `chunk_execution_test.rs` | ⚠️ Placeholder | Chunk execution tests |

## Running Tests

```bash
cd zkml

# Run all testing/ tests
cargo test --test test_merkle_root_public
cargo test --test merkle_tree_test
cargo test --test chunk_execution_test

# Run with output
cargo test --test test_merkle_root_public -- --nocapture

# Run all tests at once
cargo test
```

## What's Tested

### `test_merkle_root_public.rs` (Main Tests)

1. **`test_merkle_root_in_public_values`**
   - Compares public values WITH and WITHOUT Merkle enabled
   - Verifies Merkle root adds exactly one public value
   - Validates circuit with MockProver

2. **`test_full_model_execution_still_works`**
   - Regression test for full model execution
   - Ensures new chunk/Merkle code doesn't break existing functionality

3. **`test_chunk_execution_without_merkle`**
   - Tests chunk execution independently of Merkle trees
   - Verifies partial DAG execution works

## CI Integration

These tests run automatically on every PR via GitHub Actions (`.github/workflows/ci.yml`).

**Excluded from CI:**
- `tests/aws/` - AWS/GPU tests (run manually to save costs)
- Benchmarks - too slow for PR checks

## Adding New Tests

1. Create a new `.rs` file in this directory
2. Add to `zkml/Cargo.toml`:
   ```toml
   [[test]]
   name = "my_new_test"
   path = "testing/my_new_test.rs"
   ```
3. Tests will automatically run in CI
