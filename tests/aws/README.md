# AWS GPU Testing

## Quick Start

1. **Set AWS credentials:**
   ```bash
   export AWS_ACCESS_KEY_ID=your_key
   export AWS_SECRET_ACCESS_KEY=your_secret
   export AWS_SESSION_TOKEN=your_token
   ```

2. **Set up resources (choose one method):**

   **Method A: Automated Setup (Recommended)**
   ```bash
   # Optional: Set custom resource names for auto-detection
   export AWS_KEY_NAME=your-key-name              # Optional: for auto-detection
   export AWS_SECURITY_GROUP_NAME=your-sg-name    # Optional: for auto-detection
   
   # Run setup script (will prompt or create resources)
   ./tests/aws/setup_aws_resources.sh
   
   # Copy the export commands it shows, then set:
   export KEY_NAME=your-key-name                  # Required: from setup script
   export SECURITY_GROUP=sg-xxxxx                 # Required: from setup script
   ```

   **Method B: Manual Configuration**
   ```bash
   # Set required resource identifiers manually
   export KEY_NAME=your-key-name                  # Your EC2 key pair name
   export SECURITY_GROUP=sg-xxxxx                # Your security group ID
   
   # Optional: Override defaults
   export AWS_REGION=us-west-2                    # Default: us-west-2
   export INSTANCE_TYPE=g5.xlarge                 # Default: g5.xlarge
   export AMI_ID=ami-0076e7fffffc9251d           # Default: Ubuntu 20.04, PyTorch 2.3.1
   ```

3. **Launch instance and run tests:**
   ```bash
   ./tests/aws/manage_aws_instance.sh
   ```
   This will automatically:
   - Launch a `g5.xlarge` instance (A100 GPU)
   - Wait for SSH
   - Install dependencies
   - Run GPU tests
   - Shutdown the instance

## Configuration

All configuration is done via environment variables:

### Required Variables

```bash
# AWS Credentials (required)
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_SESSION_TOKEN=your_session_token

# Resource Identifiers (required - get from setup script or set manually)
export KEY_NAME=your-key-name                  # EC2 key pair name
export SECURITY_GROUP=sg-xxxxx                # Security group ID
```

### Optional Variables

```bash
# For auto-detection in setup script
export AWS_KEY_NAME=your-key-name              # Optional: key name for auto-detection
export AWS_SECURITY_GROUP_NAME=your-sg-name    # Optional: security group name for auto-detection

# Override defaults
export AWS_REGION=us-west-2                    # Default: us-west-2
export INSTANCE_TYPE=g5.xlarge                 # Default: g5.xlarge (1x A100 40GB)
export AMI_ID=ami-0076e7fffffc9251d           # Default: Ubuntu 20.04, PyTorch 2.3.1
export SUBNET_ID=subnet-xxxxx                  # Auto-detected from security group's VPC if not set
```

**Quick Start**: Run `./tests/aws/setup_aws_resources.sh` first to get `KEY_NAME` and `SECURITY_GROUP`, then run `./tests/aws/manage_aws_instance.sh`.

## Files

- `setup_aws_resources.sh` - Creates/gets key pair and security group
- `manage_aws_instance.sh` - Launches instance, runs tests, shuts down
- `find_ami.sh` - Lists available Deep Learning AMIs
- `gpu_test.py` - Python test suite for GPU testing
- `aws_setup_guide.md` - Comprehensive AWS setup guide

## Troubleshooting

**"Missing required parameters"**
- Run `./tests/aws/setup_aws_resources.sh` first to get KEY_NAME and SECURITY_GROUP

**"AWS credentials not set"**
- Set: `export AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`

**"Identity file not accessible" or "Permission denied" on SSH**
- The key pair exists in AWS but the private key file is missing locally
- If you have the key file elsewhere, copy it to: `~/.ssh/$KEY_NAME.pem` (where `$KEY_NAME` is your key name)
- If you lost the key file, you'll need to create a new key pair or retrieve the key from where it was originally saved
- Set correct permissions: `chmod 400 ~/.ssh/$KEY_NAME.pem`
- Make sure your security group allows SSH (port 22) from your IP

**"nvidia-smi not found"**
- Make sure you're using a Deep Learning AMI (default AMI has it)
- Or install drivers: `sudo apt-get install nvidia-driver-535`

