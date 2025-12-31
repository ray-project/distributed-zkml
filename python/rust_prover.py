"""
Python wrapper for the Rust prove_chunk CLI binary.

This module provides a Python interface to the zkml proof generation system,
allowing Ray workers to generate ZK proofs for model chunks.
"""

import json
import os
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Optional


@dataclass
class ProofResult:
    """Result from proof generation."""
    chunk_start: int
    chunk_end: int
    use_merkle: bool
    prev_merkle_root: Optional[str]
    merkle_root: Optional[str]
    proving_time_ms: int
    verify_time_ms: int
    proof_size_bytes: int
    public_vals_count: int
    proof_path: str
    public_vals_path: str
    output_dir: str


def find_prove_chunk_binary() -> str:
    """Find the prove_chunk binary, checking common locations."""
    # Check common locations relative to this file
    this_dir = Path(__file__).parent
    candidates = [
        this_dir.parent / "zkml" / "target" / "release" / "prove_chunk",
        this_dir.parent / "zkml" / "target" / "debug" / "prove_chunk",
        Path("zkml") / "target" / "release" / "prove_chunk",
        Path("zkml") / "target" / "debug" / "prove_chunk",
    ]
    
    for candidate in candidates:
        if candidate.exists():
            return str(candidate.resolve())
    
    # Try PATH
    try:
        result = subprocess.run(
            ["which", "prove_chunk"],
            capture_output=True,
            text=True,
            check=True
        )
        return result.stdout.strip()
    except subprocess.CalledProcessError:
        pass
    
    raise FileNotFoundError(
        "Could not find prove_chunk binary. "
        "Build it with: cd zkml && cargo build --bin prove_chunk --release"
    )


def prove_chunk(
    config_path: str,
    input_path: str,
    chunk_start: int,
    chunk_end: int,
    use_merkle: bool = False,
    prev_merkle_root: Optional[str] = None,
    params_dir: str = "./params_kzg",
    output_dir: Optional[str] = None,
    binary_path: Optional[str] = None,
) -> ProofResult:
    """
    Generate a ZK proof for a model chunk using the Rust prover.
    
    Args:
        config_path: Path to model config/weights (msgpack)
        input_path: Path to input data (msgpack)
        chunk_start: Start layer index (inclusive)
        chunk_end: End layer index (exclusive)
        use_merkle: Enable Merkle tree for intermediate values
        prev_merkle_root: Previous chunk's Merkle root (hex string)
        params_dir: Directory for KZG params
        output_dir: Output directory for proof files (temp dir if None)
        binary_path: Path to prove_chunk binary (auto-detect if None)
    
    Returns:
        ProofResult with proof metadata and file paths
    
    Raises:
        FileNotFoundError: If binary or input files not found
        subprocess.CalledProcessError: If proof generation fails
        ValueError: If result.json is invalid
    """
    # Find binary
    if binary_path is None:
        binary_path = find_prove_chunk_binary()
    
    # Validate inputs exist
    if not os.path.exists(config_path):
        raise FileNotFoundError(f"Config file not found: {config_path}")
    if not os.path.exists(input_path):
        raise FileNotFoundError(f"Input file not found: {input_path}")
    
    # Create output directory
    if output_dir is None:
        output_dir = tempfile.mkdtemp(prefix="zkml_proof_")
    else:
        os.makedirs(output_dir, exist_ok=True)
    
    # Ensure params directory exists
    os.makedirs(params_dir, exist_ok=True)
    
    # Build command
    cmd = [
        binary_path,
        "--config", config_path,
        "--input", input_path,
        "--start", str(chunk_start),
        "--end", str(chunk_end),
        "--params-dir", params_dir,
        "--output-dir", output_dir,
    ]
    
    if use_merkle:
        cmd.append("--use-merkle")
    
    if prev_merkle_root is not None:
        cmd.extend(["--prev-root", prev_merkle_root])
    
    # Run prover
    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
    )
    
    if result.returncode != 0:
        raise subprocess.CalledProcessError(
            result.returncode,
            cmd,
            output=result.stdout,
            stderr=result.stderr,
        )
    
    # Parse result
    result_path = os.path.join(output_dir, "result.json")
    if not os.path.exists(result_path):
        raise ValueError(f"result.json not found in {output_dir}")
    
    with open(result_path, "r") as f:
        data = json.load(f)
    
    return ProofResult(
        chunk_start=data["chunk_start"],
        chunk_end=data["chunk_end"],
        use_merkle=data["use_merkle"],
        prev_merkle_root=data.get("prev_merkle_root"),
        merkle_root=data.get("merkle_root"),
        proving_time_ms=data["proving_time_ms"],
        verify_time_ms=data["verify_time_ms"],
        proof_size_bytes=data["proof_size_bytes"],
        public_vals_count=data["public_vals_count"],
        proof_path=os.path.join(output_dir, "proof.bin"),
        public_vals_path=os.path.join(output_dir, "public_vals.bin"),
        output_dir=output_dir,
    )


if __name__ == "__main__":
    # Simple test
    import sys
    
    if len(sys.argv) < 3:
        print("Usage: python rust_prover.py <config.msgpack> <input.msgpack>")
        print("\nExample:")
        print("  python rust_prover.py zkml/examples/mnist/model.msgpack zkml/examples/mnist/inp.msgpack")
        sys.exit(1)
    
    config = sys.argv[1]
    inp = sys.argv[2]
    
    print(f"Testing prove_chunk with config={config}, input={inp}")
    
    try:
        result = prove_chunk(
            config_path=config,
            input_path=inp,
            chunk_start=0,
            chunk_end=2,
            use_merkle=False,
        )
        print(f"\nSuccess!")
        print(f"  Chunk: [{result.chunk_start}, {result.chunk_end})")
        print(f"  Proving time: {result.proving_time_ms}ms")
        print(f"  Verify time: {result.verify_time_ms}ms")
        print(f"  Proof size: {result.proof_size_bytes} bytes")
        print(f"  Public values: {result.public_vals_count}")
        print(f"  Output dir: {result.output_dir}")
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)

