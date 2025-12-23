# AWS GPU Instance Setup Guide

## Quick Start

### 1. Set AWS Credentials

```bash
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_SESSION_TOKEN=your_session_token
```

### 2. Find Deep Learning AMI

**Option A: Use helper script**
```bash
./tests/aws/find_ami.sh
```

**Option B: Manual command (this one works!)**
```bash
aws ec2 describe-images \
  --owners amazon \
  --filters Name=name,Values="*Deep Learning*" \
  --query "Images[*].[ImageId,Name,CreationDate]" \
  --output table \
  --region us-west-2 | grep -i pytorch | head -10
```

**Default AMI (recommended):**
```bash
export AMI_ID=ami-0076e7fffffc9251d  # Ubuntu 20.04, PyTorch 2.3.1
```

Or choose a different AMI ID from the output above.

### 3. Get Your Key Pair and Security Group

You need these two before launching:

```bash
# List key pairs
aws ec2 describe-key-pairs --region us-west-2

# List security groups (find one that allows SSH)
aws ec2 describe-security-groups \
  --region us-west-2 \
  --query "SecurityGroups[*].[GroupId,GroupName]" \
  --output table
```

### 4. Launch Instance

```bash
# Set configuration
export AMI_ID=ami-xxxxx  # From step 2
export KEY_NAME=your-key-name
export SECURITY_GROUP=sg-xxxxx
export INSTANCE_TYPE=g5.xlarge  # A100 instance

# Launch instance
aws ec2 run-instances \
  --image-id "$AMI_ID" \
  --instance-type "$INSTANCE_TYPE" \
  --key-name "$KEY_NAME" \
  --security-group-ids "$SECURITY_GROUP" \
  --region us-west-2
```

### 5. SSH into Instance

```bash
# Get public IP
INSTANCE_ID=i-xxxxx  # From step 4 output
PUBLIC_IP=$(aws ec2 describe-instances \
  --instance-ids "$INSTANCE_ID" \
  --query 'Reservations[0].Instances[0].PublicIpAddress' \
  --output text)

# SSH (wait a minute for instance to boot)
ssh -i ~/.ssh/your-key.pem ubuntu@"$PUBLIC_IP"
```

### 6. On the Instance: Setup and Run Tests

```bash
# Install dependencies
sudo apt-get update
sudo apt-get install -y python3-pip git
pip3 install --user ray torch boto3

# Clone repository
git clone https://github.com/ray-project/distributed-zkml.git
cd distributed-zkml

# Set AWS credentials
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_SESSION_TOKEN=your_session_token

# Run tests
python3 tests/aws/gpu_test.py
```

### 7. Shutdown Instance

```bash
# From your local machine
aws ec2 terminate-instances --instance-ids "$INSTANCE_ID"
```

## Automated Script

Use the provided script for automated instance management:

```bash
# Set configuration
export AMI_ID=ami-xxxxx
export KEY_NAME=your-key-name
export SECURITY_GROUP=sg-xxxxx
export INSTANCE_TYPE=g5.xlarge

# Run script (launches, tests, shuts down)
./tests/aws/manage_aws_instance.sh
```

## Instance Types

- **A100 (1x)**: `g5.xlarge` - 16 vCPU, 64 GB RAM, 1x A100 40GB
- **A100 (4x)**: `p4d.24xlarge` - 96 vCPU, 1152 GB RAM, 4x A100 40GB
- **H100 (8x)**: `p5.48xlarge` - 192 vCPU, 2048 GB RAM, 8x H100 80GB

## Cost Estimates

- `g5.xlarge`: ~$1.00/hour
- `p4d.24xlarge`: ~$32.77/hour
- `p5.48xlarge`: ~$98.32/hour

**Remember to terminate instances when done!**

## Troubleshooting

### "Instance launch failed"
- Check AMI is available in your region
- Verify security group allows SSH (port 22)
- Ensure key pair exists in the region

### "Cannot connect via SSH"
- Wait 2-3 minutes for instance to fully boot
- Check security group allows your IP
- Verify key file permissions: `chmod 400 ~/.ssh/key.pem`

### "nvidia-smi not found"
- Deep Learning AMI should have drivers pre-installed
- If not: `sudo apt-get install nvidia-driver-535`

### "Ray initialization failed"
- Update Ray: `pip3 install --upgrade ray`
- Check Ray version compatibility

