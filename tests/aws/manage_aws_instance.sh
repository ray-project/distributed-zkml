#!/bin/bash
# Script to launch AWS GPU instance, run tests, and shutdown

set -e

# Configuration
INSTANCE_TYPE="${INSTANCE_TYPE:-g5.xlarge}"  # A100 instance
AMI_ID="${AMI_ID:-ami-0076e7fffffc9251d}"  # Default: Deep Learning OSS Nvidia Driver AMI GPU PyTorch 2.3.1 (Ubuntu 20.04)
KEY_NAME="${KEY_NAME:-}"  # Set via KEY_NAME env var (required)
SECURITY_GROUP="${SECURITY_GROUP:-}"  # Set via SECURITY_GROUP env var (required)
REGION="${AWS_REGION:-us-west-2}"

# Auto-detect security group by name if SECURITY_GROUP is not set but AWS_SECURITY_GROUP_NAME is
if [ -z "$SECURITY_GROUP" ] && [ -n "${AWS_SECURITY_GROUP_NAME:-}" ]; then
    DEFAULT_SG_ID=$(aws ec2 describe-security-groups \
        --region "$REGION" \
        --filters Name=group-name,Values="$AWS_SECURITY_GROUP_NAME" Name=ip-permission.from-port,Values=22 \
        --query 'SecurityGroups[0].GroupId' \
        --output text 2>/dev/null || echo "")
    if [ -n "$DEFAULT_SG_ID" ] && [ "$DEFAULT_SG_ID" != "None" ]; then
        SECURITY_GROUP="$DEFAULT_SG_ID"
    fi
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== AWS GPU Instance Management ===${NC}"
echo ""

# Check AWS credentials
if [ -z "$AWS_ACCESS_KEY_ID" ] || [ -z "$AWS_SECRET_ACCESS_KEY" ] || [ -z "$AWS_SESSION_TOKEN" ]; then
    echo -e "${RED}Error: AWS credentials not set${NC}"
    echo "Set: export AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_SESSION_TOKEN"
    exit 1
fi

# Check if SSH key file exists
KEY_FILE="$HOME/.ssh/$KEY_NAME.pem"
if [ ! -f "$KEY_FILE" ]; then
    echo -e "${YELLOW}Warning: SSH key file not found: $KEY_FILE${NC}"
    echo "The key pair '$KEY_NAME' exists in AWS but the private key file is missing locally."
    echo ""
    echo "Options:"
    echo "1. If you have the key file elsewhere, copy it to: $KEY_FILE"
    echo "2. If you lost the key file, you'll need to:"
    echo "   - Create a new key pair in AWS"
    echo "   - Download the private key"
    echo "   - Save it to $KEY_FILE"
    echo "   - Set permissions: chmod 400 $KEY_FILE"
    echo ""
    echo "For now, the script will continue but SSH will fail."
    echo ""
    read -p "Continue anyway? (y/N): " CONTINUE
    if [ "$CONTINUE" != "y" ] && [ "$CONTINUE" != "Y" ]; then
        exit 1
    fi
fi

# Check required parameters
if [ -z "$AMI_ID" ] || [ -z "$KEY_NAME" ] || [ -z "$SECURITY_GROUP" ]; then
    echo -e "${YELLOW}Configuration:${NC}"
    echo "  INSTANCE_TYPE: $INSTANCE_TYPE"
    echo "  AMI_ID: ${AMI_ID:-NOT SET}"
    echo "  KEY_NAME: ${KEY_NAME:-NOT SET}"
    echo "  SECURITY_GROUP: ${SECURITY_GROUP:-NOT SET}"
    echo "  SUBNET_ID: ${SUBNET_ID:-NOT SET (will auto-detect)}"
    echo ""
    echo -e "${RED}Error: Missing required parameters${NC}"
    echo ""
    echo "Quick setup (recommended):"
    echo "  ./tests/aws/setup_aws_resources.sh"
    echo "  # Then copy the export commands it shows"
    echo ""
    echo "Or set manually:"
    echo "  export SECURITY_GROUP=sg-xxxxx"
    echo ""
    echo "Note:"
    echo "  - AMI_ID defaults to ami-0076e7fffffc9251d (Ubuntu 20.04, PyTorch 2.3.1)"
    echo "  - KEY_NAME must be set via environment variable"
    echo "  - SECURITY_GROUP must be set via environment variable (or AWS_SECURITY_GROUP_NAME for auto-lookup)"
    exit 1
fi

echo -e "${GREEN}1. Launching AWS instance...${NC}"

# Auto-detect subnet if not set (after security group is confirmed)
SUBNET_ID="${SUBNET_ID:-}"
if [ -z "$SUBNET_ID" ]; then
    echo "Auto-detecting subnet..."
    # Try to find subnet in the same VPC as the security group
    SG_VPC=$(aws ec2 describe-security-groups \
        --region "$REGION" \
        --group-ids "$SECURITY_GROUP" \
        --query 'SecurityGroups[0].VpcId' \
        --output text 2>/dev/null || echo "")
    
    if [ -n "$SG_VPC" ] && [ "$SG_VPC" != "None" ] && [ "$SG_VPC" != "" ]; then
        # Find subnet in the same VPC
        SUBNET_ID=$(aws ec2 describe-subnets \
            --region "$REGION" \
            --filters Name=vpc-id,Values="$SG_VPC" Name=state,Values=available \
            --query 'Subnets[0].SubnetId' \
            --output text 2>/dev/null | grep -v "^None$" | head -1 || echo "")
    fi
    
    # If still no subnet, find any available subnet in the region
    if [ -z "$SUBNET_ID" ] || [ "$SUBNET_ID" == "None" ] || [ "$SUBNET_ID" == "" ]; then
        SUBNET_ID=$(aws ec2 describe-subnets \
            --region "$REGION" \
            --filters Name=state,Values=available \
            --query 'Subnets[0].SubnetId' \
            --output text 2>/dev/null | grep -v "^None$" | head -1 || echo "")
    fi
    
    if [ -n "$SUBNET_ID" ] && [ "$SUBNET_ID" != "None" ] && [ "$SUBNET_ID" != "" ]; then
        echo "Found subnet: $SUBNET_ID"
    else
        echo -e "${RED}Error: No subnet found. Cannot launch instance.${NC}"
        echo "Please set a subnet manually:"
        echo "  export SUBNET_ID=subnet-xxxxx"
        echo ""
        echo "To find available subnets:"
        echo "  aws ec2 describe-subnets --region $REGION --filters Name=state,Values=available --query 'Subnets[*].[SubnetId,VpcId,AvailabilityZone]' --output table"
        exit 1
    fi
fi

# Launch instance with subnet
if [ -n "$SUBNET_ID" ] && [ "$SUBNET_ID" != "None" ] && [ "$SUBNET_ID" != "" ]; then
    INSTANCE_OUTPUT=$(aws ec2 run-instances \
        --image-id "$AMI_ID" \
        --instance-type "$INSTANCE_TYPE" \
        --key-name "$KEY_NAME" \
        --security-group-ids "$SECURITY_GROUP" \
        --subnet-id "$SUBNET_ID" \
        --region "$REGION" \
        --output json)
else
    echo -e "${RED}Error: No subnet available. Cannot launch instance.${NC}"
    exit 1
fi

INSTANCE_ID=$(echo "$INSTANCE_OUTPUT" | jq -r '.Instances[0].InstanceId')
echo "Instance ID: $INSTANCE_ID"

echo -e "${YELLOW}Waiting for instance to be running...${NC}"
aws ec2 wait instance-running --instance-ids "$INSTANCE_ID" --region "$REGION"

# Get public IP
PUBLIC_IP=$(aws ec2 describe-instances \
    --instance-ids "$INSTANCE_ID" \
    --region "$REGION" \
    --query 'Reservations[0].Instances[0].PublicIpAddress' \
    --output text)

echo "Public IP: $PUBLIC_IP"
echo ""

echo -e "${GREEN}2. Waiting for SSH to be ready...${NC}"
sleep 10  # Give instance time to boot

# Wait for SSH
for i in {1..30}; do
    if ssh -o StrictHostKeyChecking=no -o ConnectTimeout=5 \
           -o IdentitiesOnly=yes -o PreferredAuthentications=publickey \
           -i "$KEY_FILE" ubuntu@"$PUBLIC_IP" "echo 'SSH ready'" 2>/dev/null; then
        echo "SSH is ready!"
        break
    fi
    echo "Waiting for SSH... ($i/30)"
    sleep 5
done

echo ""
echo -e "${GREEN}3. Setting up instance and running tests...${NC}"

# Create test script to run on instance
cat > /tmp/run_tests.sh << 'TESTSCRIPT'
#!/bin/bash
set -e

# Install dependencies
echo "Installing dependencies..."
sudo apt-get update -qq
sudo apt-get install -y python3-pip git nvidia-driver-535 2>/dev/null || true

# Install Python packages
pip3 install --user ray torch boto3

# Code will be copied from local machine (see script below)
cd ~/distributed-zkml

# Set AWS credentials (passed from host)
export AWS_ACCESS_KEY_ID="$1"
export AWS_SECRET_ACCESS_KEY="$2"
export AWS_SESSION_TOKEN="$3"

# Run tests
echo "Running GPU tests..."
python3 tests/aws/gpu_test.py

echo "Tests completed!"
TESTSCRIPT

# Copy test script and code to instance
if [ ! -f "$KEY_FILE" ]; then
    echo -e "${RED}Error: Cannot copy files - SSH key file not found: $KEY_FILE${NC}"
    exit 1
fi

# Copy the entire distributed-zkml directory (excluding .git to save space)
echo "Copying code to instance..."
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"
tar --exclude='.git' --exclude='target' --exclude='__pycache__' --exclude='*.pyc' \
    -czf /tmp/distributed-zkml.tar.gz .
scp -i "$KEY_FILE" -o StrictHostKeyChecking=no \
    -o IdentitiesOnly=yes -o PreferredAuthentications=publickey \
    /tmp/distributed-zkml.tar.gz ubuntu@"$PUBLIC_IP":~/distributed-zkml.tar.gz

# Copy test script
scp -i "$KEY_FILE" -o StrictHostKeyChecking=no \
    -o IdentitiesOnly=yes -o PreferredAuthentications=publickey \
    /tmp/run_tests.sh ubuntu@"$PUBLIC_IP":~/run_tests.sh

# Extract code on instance
ssh -i "$KEY_FILE" -o StrictHostKeyChecking=no \
    -o IdentitiesOnly=yes -o PreferredAuthentications=publickey \
    ubuntu@"$PUBLIC_IP" \
    "cd ~ && rm -rf distributed-zkml && mkdir distributed-zkml && tar -xzf distributed-zkml.tar.gz -C distributed-zkml && rm distributed-zkml.tar.gz"

# Run tests on instance
echo "Executing tests on instance..."
ssh -i "$KEY_FILE" -o StrictHostKeyChecking=no \
    -o IdentitiesOnly=yes -o PreferredAuthentications=publickey \
    ubuntu@"$PUBLIC_IP" \
    "chmod +x ~/run_tests.sh && ~/run_tests.sh '$AWS_ACCESS_KEY_ID' '$AWS_SECRET_ACCESS_KEY' '$AWS_SESSION_TOKEN'"

echo ""
echo -e "${GREEN}4. Shutting down instance...${NC}"
aws ec2 terminate-instances --instance-ids "$INSTANCE_ID" --region "$REGION"
echo "Instance $INSTANCE_ID terminated"

echo ""
echo -e "${GREEN}=== Done ===${NC}"

