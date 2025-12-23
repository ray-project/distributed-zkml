# Next Steps After Setting AMI

## Quick Checklist

1. ✅ **AMI is set** (default: `ami-0076e7fffffc9251d` - Ubuntu 20.04, PyTorch 2.3.1)
2. ✅ **Key Pair is set** (default: `masoud-hybrid-par`)
3. ⬜ **Get/Create Security Group** → Run `./tests/aws/setup_aws_resources.sh`
4. ⬜ **Launch and Test** → Run `./tests/aws/manage_aws_instance.sh`

## Easiest Way: Automated Setup

**Run this one command to get security group set up:**
```bash
./tests/aws/setup_aws_resources.sh
```

This will:
- Use default key pair `masoud-hybrid-par` (or let you choose/create one)
- Check for existing security groups with SSH (or create one)
- Show you the export commands to run

Then copy the export commands it shows and run them.

**Note**: The default key pair `masoud-hybrid-par` is used automatically. If you want to use a different key, set `export KEY_NAME=your-key-name`.

## Step-by-Step

### Step 2: Key Pair

**Check if you have one:**
```bash
aws ec2 describe-key-pairs --region us-west-2
```

**If you don't have one, create it:**
```bash
aws ec2 create-key-pair \
  --key-name distributed-zkml-key \
  --region us-west-2 \
  --query 'KeyMaterial' \
  --output text > ~/.ssh/distributed-zkml-key.pem

chmod 400 ~/.ssh/distributed-zkml-key.pem
```

**Set it:**
```bash
export KEY_NAME=distributed-zkml-key
```

### Step 3: Security Group

**Check if you have one that allows SSH:**
```bash
aws ec2 describe-security-groups \
  --region us-west-2 \
  --query "SecurityGroups[*].[GroupId,GroupName,Description]" \
  --output table
```

**If you don't have one, create it:**
```bash
# Create security group
SG_OUTPUT=$(aws ec2 create-security-group \
  --group-name distributed-zkml-test \
  --description "Security group for distributed-zkml GPU testing" \
  --region us-west-2)

SG_ID=$(echo $SG_OUTPUT | jq -r '.GroupId')
echo "Created security group: $SG_ID"

# Allow SSH (port 22)
aws ec2 authorize-security-group-ingress \
  --group-id "$SG_ID" \
  --protocol tcp \
  --port 22 \
  --cidr 0.0.0.0/0 \
  --region us-west-2

export SECURITY_GROUP="$SG_ID"
```

**Or if you already have one:**
```bash
export SECURITY_GROUP=sg-xxxxx  # Replace with your security group ID
```

### Step 4: Launch and Test

**Option A: Automated (recommended)**
```bash
# Make sure these are set:
export KEY_NAME=your-key-name
export SECURITY_GROUP=sg-xxxxx

# Launch, test, and shutdown automatically
./tests/aws/manage_aws_instance.sh
```

**Option B: Manual**
```bash
# Launch instance
aws ec2 run-instances \
  --image-id ami-0076e7fffffc9251d \
  --instance-type g5.xlarge \
  --key-name "$KEY_NAME" \
  --security-group-ids "$SECURITY_GROUP" \
  --region us-west-2 \
  --output json | jq -r '.Instances[0].InstanceId'

# Save the instance ID, then SSH and run tests
# (See QUICK_START.md for full manual steps)
```

## Default Values

- **AMI**: `ami-0076e7fffffc9251d` (Ubuntu 20.04, PyTorch 2.3.1)
- **Instance Type**: `g5.xlarge` (1x A100 40GB)
- **Region**: `us-west-2`

These are set as defaults in `manage_aws_instance.sh`, so you only need to provide `KEY_NAME` and `SECURITY_GROUP`.

