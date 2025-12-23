# AWS GPU Tests

Tests for distributed proving on AWS GPU instances (A100/H100).

## Prerequisites

### AWS Credentials

Set the following environment variables:

```bash
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_SESSION_TOKEN=your_session_token
```

### GPU Instance

Launch an AWS instance with GPU support:
- **A100**: `g5.xlarge` or larger (1x A100)
- **H100**: `p5.48xlarge` (8x H100)

### Dependencies

```bash
# Install Rust (nightly)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup override set nightly

# Install Python dependencies
pip install ray torch

# Install CUDA drivers (usually pre-installed on GPU instances)
nvidia-smi  # Verify GPU is available
```

## Running Tests

```bash
# From distributed-zkml root
python3 tests/aws/gpu_test.py
```

## Test Suite

The test suite includes:

1. **AWS Credentials Check**: Validates required environment variables
2. **GPU Availability Check**: Verifies GPU is accessible via `nvidia-smi`
3. **Ray Cluster Setup**: Initializes Ray with GPU support
4. **Basic GPU Distribution**: Tests task distribution across GPU workers
5. **Distributed Proving Simulation**: Runs distributed proving with Merkle trees

## Expected Output

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

## Troubleshooting

### "Missing AWS credentials"
- Ensure all three environment variables are set
- Check credentials are valid: `aws sts get-caller-identity`

### "nvidia-smi not found"
- GPU instance may not have CUDA drivers installed
- Install NVIDIA drivers: `sudo apt-get install nvidia-driver-535`

### "Ray initialization failed"
- Check GPU availability: `nvidia-smi`
- Verify Ray installation: `pip install --upgrade ray`

### "PyTorch CUDA not available"
- Install PyTorch with CUDA: `pip install torch --index-url https://download.pytorch.org/whl/cu118`

## Performance Notes

- **A100**: ~40GB VRAM, good for large models
- **H100**: ~80GB VRAM, excellent for very large models
- Ray will automatically distribute tasks across available GPUs
- Monitor GPU usage: `watch -n 1 nvidia-smi`
