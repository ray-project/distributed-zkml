#!/bin/bash
# Ready-to-run git commit script for distributed proofs work
# Run this from the zkml directory

set -e

echo "=== Preparing Git Commit for Distributed Proofs ==="
echo ""

# Check we're in the zkml directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Must run from zkml directory"
    exit 1
fi

echo "1. Removing redundant documentation files..."
# Remove redundant docs (info is consolidated in DISTRIBUTED_PROOFS_README.md)
rm -f CLARIFICATIONS.md \
      DISTRIBUTED_PROVING.md \
      DISTRIBUTED_PROVING_DIAGRAMS.md \
      MERKLE_TREE_APPROACH.md \
      MERKLE_IMPLEMENTATION.md \
      RAY_INTEGRATION.md \
      RAY_IMPLEMENTATION_SUMMARY.md \
      MINIMAL_EXAMPLE_SUMMARY.md \
      QUICKSTART_DISTRIBUTED.md \
      HOW_TO_RUN.md \
      IMPLEMENTATION_STATUS.md \
      PUBLIC_VALUES_EXPLAINED.md \
      GIT_COMMIT_GUIDE.md \
      COMMIT_CHECKLIST.md \
      FILES_TO_KEEP.md \
      README_COMMIT.md

echo "   ✓ Removed redundant documentation files"

echo ""
echo "2. Adding core implementation files..."
git add src/commitments/merkle.rs
git add src/distributed/
git add src/ray/
git add src/lib.rs
git add src/commitments.rs

echo "3. Adding examples..."
git add -f examples/simple_distributed.py
git add -f examples/distributed_proof_simple.rs
git add -f examples/README_DISTRIBUTED.md

echo "4. Adding main documentation..."
git add DISTRIBUTED_PROOFS_README.md

# Keep ARCHITECTURE.md and COMPONENT_DIAGRAM.md (useful reference)
if [ -f "ARCHITECTURE.md" ]; then
    git add ARCHITECTURE.md
fi
if [ -f "COMPONENT_DIAGRAM.md" ]; then
    git add COMPONENT_DIAGRAM.md
fi

echo ""
echo "5. Showing what will be committed..."
git status --short

echo ""
echo "=== Summary ==="
echo "Essential files added:"
echo "  ✓ Core implementation (merkle.rs, distributed/, ray/)"
echo "  ✓ Examples (simple_distributed.py, etc.)"
echo "  ✓ Main documentation (DISTRIBUTED_PROOFS_README.md)"
echo ""
echo "=== Ready to commit! ==="
echo ""
echo "Run this to commit:"
echo ""
echo 'git commit -m "feat: Add distributed proving with Ray, layer partitioning, and Merkle trees

This commit adds initial support for distributed proving of ML models:

- Merkle tree implementation for privacy-preserving intermediate values
- Model chunking structure for layer-wise partitioning
- Ray integration framework for distributed execution
- Working Python example demonstrating the concept

Implementation:
- src/commitments/merkle.rs: Merkle tree gadget using Poseidon
- src/distributed/: Model chunking and distributed proof structures
- src/ray/: Ray integration for batch inference
- examples/simple_distributed.py: Working demonstration

Status: Conceptual implementation complete. Circuit integration pending.

See DISTRIBUTED_PROOFS_README.md for details."'
echo ""
