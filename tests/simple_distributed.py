#!/usr/bin/env python3
"""
Distributed proving with Ray, layer partitioning, and ZK proofs.

This script demonstrates distributed proof generation using Ray workers
that call the Rust zkml prover.

Usage:
    # Simulation mode (placeholder proofs, fast)
    python tests/simple_distributed.py --model zkml/examples/mnist/model.msgpack \
        --input zkml/examples/mnist/inp.msgpack --layers 4 --workers 2
    
    # Real proof generation (requires built Rust binary)
    python tests/simple_distributed.py --model zkml/examples/mnist/model.msgpack \
        --input zkml/examples/mnist/inp.msgpack --layers 4 --workers 1 --real
"""

import ray
import json
import os
import sys
import logging
import tempfile
from dataclasses import asdict
from typing import Dict, List, Tuple, Optional

# Add parent dir to path for imports
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Suppress Ray dashboard startup message
logging.getLogger("ray").setLevel(logging.WARNING)


@ray.remote
class ChunkWorker:
    """Ray worker for proving a model chunk"""
    
    def __init__(self, chunk_id: int, use_real_prover: bool = False):
        self.chunk_id = chunk_id
        self.use_real_prover = use_real_prover
        self._prover = None
        
    def _get_prover(self):
        """Lazy load the prover module (only when real proving)"""
        if self._prover is None and self.use_real_prover:
            from python.rust_prover import prove_chunk
            self._prover = prove_chunk
        return self._prover
        
    def prove_chunk(
        self,
        model_config: str,
        input_path: str,
        layer_start: int,
        layer_end: int,
        params_dir: str = "./params_kzg",
    ) -> Dict:
        """
        Prove a chunk of the model.
        
        Args:
            model_config: Path to model config/weights (msgpack)
            input_path: Path to input data (msgpack)
            layer_start: Start layer index (inclusive)
            layer_end: End layer index (exclusive)
            params_dir: Directory for KZG params
        
        Returns:
            Dict with proof result metadata
        """
        print(f"Worker {self.chunk_id}: Proving layers {layer_start}-{layer_end}")
        
        if self.use_real_prover:
            # Real proof generation using Rust binary
            prove_chunk = self._get_prover()
            
            try:
                result = prove_chunk(
                    config_path=model_config,
                    input_path=input_path,
                    chunk_start=layer_start,
                    chunk_end=layer_end,
                    use_merkle=False,  # Merkle has known issue (#12)
                    params_dir=params_dir,
                )
                
                return {
                    "chunk_id": self.chunk_id,
                    "layers": f"{layer_start}-{layer_end}",
                    "merkle_root": result.merkle_root,
                    "proof_size": result.proof_size_bytes,
                    "proving_time_ms": result.proving_time_ms,
                    "verify_time_ms": result.verify_time_ms,
                    "public_vals_count": result.public_vals_count,
                    "proof_path": result.proof_path,
                    "output_dir": result.output_dir,
                    "status": "success",
                    "mode": "real",
                }
            except Exception as e:
                return {
                    "chunk_id": self.chunk_id,
                    "layers": f"{layer_start}-{layer_end}",
                    "status": "error",
                    "error": str(e),
                    "mode": "real",
                }
        else:
            # Simulation mode (placeholder)
            return {
                "chunk_id": self.chunk_id,
                "layers": f"{layer_start}-{layer_end}",
                "merkle_root": f"0x{'a' * 64}",  # Placeholder
                "proof_size": 3360,  # Typical size
                "status": "success",
                "mode": "simulation",
            }


def partition_model(num_layers: int, num_chunks: int) -> List[Tuple[int, int]]:
    """Partition model into chunks by layer ranges."""
    layers_per_chunk = num_layers // num_chunks
    chunks = []
    for i in range(num_chunks):
        start = i * layers_per_chunk
        end = start + layers_per_chunk if i < num_chunks - 1 else num_layers
        chunks.append((start, end))
    return chunks


def distributed_prove(
    model_config: str,
    input_path: str,
    num_layers: int = 4,
    num_workers: int = 2,
    use_real_prover: bool = False,
    params_dir: str = "./params_kzg",
    sequential: bool = True,
) -> List[Dict]:
    """
    Distributed proof generation with Ray.
    
    Args:
        model_config: Path to model configuration
        input_path: Path to input data
        num_layers: Total number of layers in model
        num_workers: Number of Ray workers (chunks)
        use_real_prover: If True, use Rust prover; otherwise simulation
        params_dir: Directory for KZG params (real mode only)
        sequential: If True, run chunks sequentially (required for real proofs
                   since each chunk needs outputs from previous layers)
    
    Returns:
        List of chunk results
    """
    # Partition model into chunks
    chunks = partition_model(num_layers, num_workers)
    
    # Create workers
    workers = [
        ChunkWorker.remote(i, use_real_prover=use_real_prover) 
        for i in range(num_workers)
    ]
    
    results = []
    
    if sequential and use_real_prover:
        # Sequential execution: each chunk proves from layer 0 to its end
        # This ensures all intermediate values are computed
        # Each proof covers [0, chunk_end) but we track which layers each "owns"
        print("\nNote: Running sequentially (each chunk proves [0, end) range)")
        
        for i, (start, end) in enumerate(chunks):
            # For real proofs, always start from 0 to ensure intermediate values exist
            # Each chunk "owns" layers [start, end) but proves [0, end)
            prove_start = 0
            prove_end = end
            
            print(f"  Chunk {i}: proving layers [0, {end}) (owns layers [{start}, {end}))")
            
            future = workers[i].prove_chunk.remote(
                model_config,
                input_path,
                prove_start,
                prove_end,
                params_dir,
            )
            result = ray.get(future)
            result["owned_layers"] = f"{start}-{end}"
            result["proved_layers"] = f"{prove_start}-{prove_end}"
            results.append(result)
    else:
        # Parallel execution (simulation mode or when sequential=False)
        futures = []
        for i, (start, end) in enumerate(chunks):
            future = workers[i].prove_chunk.remote(
                model_config,
                input_path,
                start,
                end,
                params_dir,
            )
            futures.append(future)
        
        results = ray.get(futures)
    
    return results


if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(
        description="Distributed ZK proving with Ray",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
    # Simulation mode (fast, no real proofs)
    python tests/simple_distributed.py --model zkml/examples/mnist/model.msgpack \\
        --input zkml/examples/mnist/inp.msgpack --layers 4 --workers 2
    
    # Real proof generation (slow, generates actual ZK proofs)
    python tests/simple_distributed.py --model zkml/examples/mnist/model.msgpack \\
        --input zkml/examples/mnist/inp.msgpack --layers 4 --workers 1 --real
        """
    )
    parser.add_argument("--model", required=True, help="Path to model.msgpack")
    parser.add_argument("--input", required=True, help="Path to input.msgpack")
    parser.add_argument("--layers", type=int, default=4, help="Number of layers")
    parser.add_argument("--workers", type=int, default=2, help="Number of workers")
    parser.add_argument("--real", action="store_true", 
                        help="Use real Rust prover (slower, generates actual proofs)")
    parser.add_argument("--params-dir", default="./params_kzg",
                        help="Directory for KZG params")
    parser.add_argument("--parallel", action="store_true",
                        help="Force parallel execution (only works in simulation mode)")
    
    args = parser.parse_args()
    
    # Initialize Ray
    ray.init(ignore_reinit_error=True)
    
    mode = "REAL" if args.real else "SIMULATION"
    exec_mode = "PARALLEL" if args.parallel else "SEQUENTIAL"
    print(f"\n=== Distributed Proving ({mode} mode, {exec_mode}) ===")
    print(f"Model: {args.model}")
    print(f"Input: {args.input}")
    print(f"Layers: {args.layers}, Workers: {args.workers}")
    
    if args.real:
        print("\nNote: Real proving takes ~2-3 seconds per chunk")
        if args.parallel:
            print("WARNING: --parallel with --real may fail for chunks > 0")
            print("         (chunks need intermediate values from previous layers)")
    
    results = distributed_prove(
        args.model,
        args.input,
        args.layers,
        args.workers,
        use_real_prover=args.real,
        params_dir=args.params_dir,
        sequential=not args.parallel,
    )
    
    print("\n=== Results ===")
    for result in results:
        print(json.dumps(result, indent=2))
    
    # Summary
    success_count = sum(1 for r in results if r.get("status") == "success")
    print(f"\n=== Summary ===")
    print(f"Total chunks: {len(results)}")
    print(f"Successful: {success_count}")
    print(f"Failed: {len(results) - success_count}")
    
    if args.real and success_count > 0:
        total_proving_time = sum(r.get("proving_time_ms", 0) for r in results)
        total_verify_time = sum(r.get("verify_time_ms", 0) for r in results)
        print(f"Total proving time: {total_proving_time}ms")
        print(f"Total verify time: {total_verify_time}ms")
