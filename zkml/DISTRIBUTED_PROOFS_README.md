# Distributed Proofs with Ray, Layer Partitioning, and Merkle Trees

## Overview

This implementation adds support for distributed proving of ML models using:
- **Ray** for distributed execution
- **Layer-wise partitioning** to split models into chunks
- **Merkle trees** to preserve privacy of intermediate values

## Quick Start

### Run the Example

```bash
cd zkml
python3 examples/simple_distributed.py \
    --model examples/mnist/model.msgpack \
    --input examples/mnist/inp.msgpack \
    --layers 4 \
    --workers 2
```

## Architecture

### Core Concept

Instead of revealing intermediate values between chunks, we:
1. Hash intermediate values into a Merkle tree
2. Reveal only the Merkle root (public)
3. Next chunk proves it knows values that hash to that root (without revealing them)

This preserves privacy while maintaining consistency.

### Implementation Structure

```
src/
├── commitments/
│   └── merkle.rs              # Merkle tree gadget using Poseidon
├── distributed/
│   ├── mod.rs                 # Distributed proving module
│   ├── chunk.rs               # ModelChunk structure
│   └── merkle_proof.rs        # Merkle proof serialization
└── ray/                       # Ray integration (batch inference)
    ├── mod.rs
    ├── batch.rs
    ├── shared.rs
    └── worker.rs

examples/
├── simple_distributed.py      # Working Python example
└── distributed_proof_simple.rs # Rust structure (needs completion)
```

## Key Components

### 1. Merkle Tree (`src/commitments/merkle.rs`)

Simple Merkle tree implementation using Poseidon hashing:
- `hash_single()` - Hash individual values
- `build_simple_tree()` - Build Merkle tree from values
- `verify_root()` - Verify values hash to expected root

### 2. Model Chunks (`src/distributed/chunk.rs`)

`ModelChunk` represents a portion of the model:
- Layer indices this chunk covers
- Input/output tensor indices
- Merkle root of intermediate values

### 3. Distributed Execution (`examples/simple_distributed.py`)

Python example demonstrating:
- Model partitioning into chunks
- Ray workers for parallel execution
- Merkle root concept for privacy

## Current Status

✅ **Working**:
- Python example runs and demonstrates concept
- Code structure in place
- Merkle tree basic implementation

⚠️ **Needs Completion**:
- Full Merkle tree integration in circuits
- Chunk execution logic
- Proof generation with Merkle roots
- Ray-Rust integration

## How It Works

### Layer Partitioning

```
Original Model: Layers 0-3
  ↓
Chunk 0: Layers 0-1 → Merkle Root₁
Chunk 1: Layers 2-3 (uses Root₁)
```

### Privacy Preservation

```
Chunk 0: Intermediate Values → Merkle Tree → Root₁ (PUBLIC)
                                 ↓
Chunk 1: Root₁ → Verify → Intermediate (PRIVATE, proven)
```

Only the Merkle root is public; intermediate values stay private.

## Next Steps

1. Complete Merkle tree in circuits (build proper binary tree)
2. Implement chunk execution (execute layers in chunk)
3. Integrate with proof generation (handle Merkle roots)
4. Connect Python to Rust (Ray-Rust integration)

See implementation files for details.

## References

- Architecture: `ARCHITECTURE.md`
- Component diagrams: `COMPONENT_DIAGRAM.md`
- Original zkml README: `README.md`

