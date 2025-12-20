# distributed-zkml

Distributed proving for zkml using Ray, layer-wise partitioning, and Merkle trees.

## What's Been Done

### Merkle Tree Integration in Rust Circuits

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

## Next Steps: Testing on A100/H100 GPUs

### 1. Set Up Ray Cluster with GPU Workers

```bash
# On head node (A100/H100)
ray start --head --num-gpus=1

# On worker nodes
ray start --address=<head-node-ip>:10001 --num-gpus=1
```

### 2. Update Python Example for GPU Workers

Modify `zkml/examples/simple_distributed.py`:

```python
@ray.remote(num_gpus=1)  # Request 1 GPU per worker
class ChunkWorker:
    ...
```

### 3. Test Distributed Proving on GPUs

```bash
cd zkml
python3 examples/simple_distributed.py \
    --model examples/mnist/model.msgpack \
    --input examples/mnist/inp.msgpack \
    --layers 4 \
    --workers 2
```

### 4. Benchmark GPU Performance

- Compare CPU vs GPU proving times
- Test with larger models (A100: 40GB, H100: 80GB)
- Measure throughput scaling with multiple GPUs

### Requirements

- Ray cluster with GPU nodes (A100/H100)
- CUDA drivers and toolkit
- Rust/Cargo for proof generation
- Model files in msgpack format

**Note**: Current implementation is CPU-based. GPU acceleration for proof generation requires additional work (Halo2 GPU support or custom GPU kernels).

See `zkml/DISTRIBUTED_PROOFS_README.md` for architecture details.
