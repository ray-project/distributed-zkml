# Quick Start: AWS GPU Testing

## 1. Setup AWS CLI

```bash
# Install AWS CLI (if not installed)
curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"
unzip awscliv2.zip
sudo ./aws/install

# Configure with your credentials
aws configure
# Enter: AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, default region (us-west-2), output format (json)
```

Or set environment variables:
```bash
export AWS_ACCESS_KEY_ID=your_key
export AWS_SECRET_ACCESS_KEY=your_secret
export AWS_SESSION_TOKEN=your_token
export AWS_DEFAULT_REGION=us-west-2
```

## 2. Find Deep Learning AMI

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

Or copy a different AMI ID from the output above.

## 3. Get Key Pair and Security Group

You need two things before launching an instance:

**A. Key Pair** (for SSH access):
```bash
# List your key pairs
aws ec2 describe-key-pairs --region us-west-2

# If you don't have one, create it:
aws ec2 create-key-pair --key-name my-key --region us-west-2 --query 'KeyMaterial' --output text > ~/.ssh/my-key.pem
chmod 400 ~/.ssh/my-key.pem
```

**B. Security Group** (firewall rules - must allow SSH on port 22):

```bash
# List security groups (find one allowing SSH on port 22)
aws ec2 describe-security-groups \
  --region us-west-2 \
  --query "SecurityGroups[*].[GroupId,GroupName,Description]" \
  --output table

# If you don't have one, create it:
aws ec2 create-security-group \
  --group-name distributed-zkml-test \
  --description "Security group for distributed-zkml GPU testing" \
  --region us-west-2

# Allow SSH (replace sg-xxxxx with your security group ID)
aws ec2 authorize-security-group-ingress \
  --group-id sg-xxxxx \
  --protocol tcp \
  --port 22 \
  --cidr 0.0.0.0/0 \
  --region us-west-2
```

## 4. Launch Instance

```bash
# Set variables
export AMI_ID=ami-xxxxx  # From step 2
export KEY_NAME=your-key-name  # From step 3
export SECURITY_GROUP=sg-xxxxx  # From step 3
export INSTANCE_TYPE=g5.xlarge  # A100 instance

# Launch
aws ec2 run-instances \
  --image-id "$AMI_ID" \
  --instance-type "$INSTANCE_TYPE" \
  --key-name "$KEY_NAME" \
  --security-group-ids "$SECURITY_GROUP" \
  --region us-west-2 \
  --output json | jq -r '.Instances[0].InstanceId'
```

Save the instance ID (e.g., `i-0abc123def456`)

## 5. Get Public IP and SSH

```bash
INSTANCE_ID=i-xxxxx  # From step 4

# Wait for instance to be running
aws ec2 wait instance-running --instance-ids "$INSTANCE_ID"

# Get public IP
PUBLIC_IP=$(aws ec2 describe-instances \
  --instance-ids "$INSTANCE_ID" \
  --query 'Reservations[0].Instances[0].PublicIpAddress' \
  --output text)

echo "Public IP: $PUBLIC_IP"

# Wait 1-2 minutes for SSH, then connect
ssh -i ~/.ssh/your-key.pem ubuntu@"$PUBLIC_IP"
```

## 6. On Instance: Setup and Test

```bash
# Install dependencies
sudo apt-get update
sudo apt-get install -y python3-pip git
pip3 install --user ray torch boto3

# Clone repo
git clone https://github.com/ray-project/distributed-zkml.git
cd distributed-zkml

# Set AWS credentials
export AWS_ACCESS_KEY_ID=your_key
export AWS_SECRET_ACCESS_KEY=your_secret
export AWS_SESSION_TOKEN=your_token

# Run tests
python3 tests/aws/gpu_test.py
```

## 7. Shutdown Instance

```bash
# From your local machine
aws ec2 terminate-instances --instance-ids "$INSTANCE_ID"
```

## Automated Script

Or use the automated script (defaults are set, so you only need security group):

```bash
# Defaults:
# - AMI: ami-0076e7fffffc9251d (Ubuntu 20.04, PyTorch 2.3.1)
# - Key: masoud-hybrid-par
# - Instance Type: g5.xlarge

# Only set security group:
export SECURITY_GROUP=sg-xxxxx

# Optional: override defaults
export KEY_NAME=your-key-name  # Default is masoud-hybrid-par
export INSTANCE_TYPE=g5.xlarge  # Default is already g5.xlarge
export AMI_ID=ami-xxxxx  # Only if you want a different AMI

./tests/aws/manage_aws_instance.sh
```

This will: launch → setup → test → shutdown automatically.

## Summary: Next Steps After Setting AMI

1. ✅ **AMI is set** (default: `ami-0076e7fffffc9251d`)
2. **Get/Create Key Pair** → `export KEY_NAME=your-key-name`
3. **Get/Create Security Group** → `export SECURITY_GROUP=sg-xxxxx`
4. **Launch Instance** → Run `./tests/aws/manage_aws_instance.sh` or follow steps 4-7 above
5. **SSH and Test** → Script handles this automatically
6. **Shutdown** → Script handles this automatically

