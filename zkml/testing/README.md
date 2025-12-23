# Testing Merkle Tree Integration

## Overview

This directory contains tests for the Merkle tree integration in zkml.

## Running Tests

### Prerequisites

Make sure Rust is installed and in your PATH:

```bash
# Check if Rust is installed
rustc --version

# If not in PATH, add it:
export PATH="$HOME/.cargo/bin:$PATH"

# Or add to your shell profile (~/.zshrc or ~/.bashrc):
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
```

### Run Tests

```bash
cd zkml

# Run all tests
cargo test

# Run specific test file
cargo test --test merkle_tree_test

# Run with output
cargo test -- --nocapture
```

## Test Files

- `merkle_tree_test.rs` - Basic Merkle tree tests
- `chunk_execution_test.rs` - Tests for chunk execution with Merkle trees

## What's Tested

1. **Merkle Tree Building**
   - Single value
   - Multiple values
   - Empty values
   - Odd number of values

2. **Chunk Execution**
   - Executing layers in a range
   - Extracting intermediate values
   - Building Merkle trees from intermediate values

3. **Root Verification**
   - Verifying Merkle roots match expected values
   - Consistency checks between chunks

## Current Status

⚠️ **Tests are placeholders** - They need proper circuit setup to work.

The Merkle tree implementation compiles successfully, but tests need:
- Proper PoseidonCommitChip configuration
- Circuit setup with MockProver
- Test data preparation

## Next Steps

1. Create proper test circuits
2. Set up MockProver tests
3. Add integration tests with real models

