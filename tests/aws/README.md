# AWS GPU Testing

## Quick Start

1. **Set AWS credentials:**
   ```bash
   export AWS_ACCESS_KEY_ID=your_key
   export AWS_SECRET_ACCESS_KEY=your_secret
   export AWS_SESSION_TOKEN=your_token
   ```

2. **Optional: Verify/override defaults:**
   ```bash
   ./tests/aws/setup_aws_resources.sh
   ```
   This will show you the defaults being used. All defaults are set automatically:
   - Key: `masoud-hybrid-par`
   - Security Group: `anyscale-security-group` (if exists)
   
   You can skip this step if defaults work for you!

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

## Defaults

- **AMI**: `ami-0076e7fffffc9251d` (Ubuntu 20.04, PyTorch 2.3.1) - set automatically
- **Instance Type**: `g5.xlarge` (1x A100 40GB) - set automatically
- **Key Name**: `masoud-hybrid-par` - set automatically (default key pair)
- **Security Group**: `anyscale-security-group` - set automatically (if exists)
- **Subnet**: Auto-detected from security group's VPC
- **Region**: `us-west-2` - set automatically

All defaults are set automatically! Just run `./tests/aws/manage_aws_instance.sh` and it will use the defaults, or run `./tests/aws/setup_aws_resources.sh` to verify/override them.

## Files

- `setup_aws_resources.sh` - Creates/gets key pair and security group
- `manage_aws_instance.sh` - Launches instance, runs tests, shuts down
- `find_ami.sh` - Lists available Deep Learning AMIs
- `gpu_test.py` - Python test suite for GPU testing
- `QUICK_START.md` - Detailed step-by-step guide
- `NEXT_STEPS.md` - What to do after setting AMI
- `aws_setup_guide.md` - Comprehensive AWS setup guide

## Troubleshooting

**"Missing required parameters"**
- Run `./tests/aws/setup_aws_resources.sh` first to get KEY_NAME and SECURITY_GROUP

**"AWS credentials not set"**
- Set: `export AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`

**"Identity file not accessible" or "Permission denied" on SSH**
- The key pair exists in AWS but the private key file is missing locally
- If you have the key file elsewhere, copy it to: `~/.ssh/masoud-hybrid-par.pem`
- If you lost the key file, you'll need to create a new key pair or retrieve the key from where it was originally saved
- Set correct permissions: `chmod 400 ~/.ssh/masoud-hybrid-par.pem`
- Make sure your security group allows SSH (port 22) from your IP

**"nvidia-smi not found"**
- Make sure you're using a Deep Learning AMI (default AMI has it)
- Or install drivers: `sudo apt-get install nvidia-driver-535`

