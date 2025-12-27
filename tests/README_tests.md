# Tests

This directory contains tests for distributed zkml.

## Test Files

### `simple_distributed.py`
Minimal simulation of distributed proving with Ray, layer partitioning, and Merkle trees. This is a test/simulation file with placeholder implementations - it does not perform actual proof generation.

**Usage:**
```bash
python3 tests/simple_distributed.py \
    --model zkml/examples/mnist/model.msgpack \
    --input zkml/examples/mnist/inp.msgpack \
    --layers 4 \
    --workers 2
```

### AWS GPU Tests (`aws/`)
Tests for distributed proving on AWS GPU instances (A100/H100) using Ray.

**Quick Start:**
```bash
# Set AWS credentials
export AWS_ACCESS_KEY_ID=your_key
export AWS_SECRET_ACCESS_KEY=your_secret
export AWS_SESSION_TOKEN=your_token

# Launch instance and run tests
./tests/aws/manage_aws_instance.sh
```

**Files:**
- `gpu_test.py` - Python test suite for GPU testing
- `setup_aws_resources.sh` - Creates/gets key pair and security group
- `manage_aws_instance.sh` - Launches instance, runs tests, shuts down
- `find_ami.sh` - Lists available Deep Learning AMIs
- `README.md` - AWS testing documentation (includes quick start guide)
- `aws_setup_guide.md` - Comprehensive AWS setup guide

See `aws/README.md` for detailed documentation.

## Running Tests

### Python Tests (pytest)
```bash
pytest tests/
```

### Run Specific Test
```bash
pytest tests/aws/gpu_test.py
```

### Run Directly (without pytest)
```bash
python3 tests/aws/gpu_test.py
```

