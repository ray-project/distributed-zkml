#!/bin/bash
# Helper script to get or create AWS resources needed for GPU testing

set -e

REGION="${AWS_REGION:-us-west-2}"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${GREEN}=== AWS Resource Setup ===${NC}"
echo ""

# Check AWS credentials
if [ -z "$AWS_ACCESS_KEY_ID" ] || [ -z "$AWS_SECRET_ACCESS_KEY" ]; then
    echo -e "${YELLOW}Warning: AWS credentials not set${NC}"
    echo "Set: export AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_SESSION_TOKEN"
    exit 1
fi

# 1. Key Pair
DEFAULT_KEY_NAME="${AWS_KEY_NAME:-}"  # Set via AWS_KEY_NAME env var, or leave empty to auto-detect/create
echo -e "${BLUE}1. Key Pair${NC}"
echo "Checking for existing key pairs..."
EXISTING_KEYS=$(aws ec2 describe-key-pairs --region "$REGION" --query 'KeyPairs[*].KeyName' --output text 2>/dev/null || echo "")

# Check if default key exists (if provided)
if [ -n "$DEFAULT_KEY_NAME" ] && echo "$EXISTING_KEYS" | grep -qw "$DEFAULT_KEY_NAME"; then
    KEY_NAME="$DEFAULT_KEY_NAME"
    KEY_FILE="$HOME/.ssh/$KEY_NAME.pem"
    if [ -f "$KEY_FILE" ]; then
        echo -e "${GREEN}✓ Using default key pair: $KEY_NAME${NC}"
        echo -e "${GREEN}✓ Key file found: $KEY_FILE${NC}"
    else
        echo -e "${YELLOW}⚠ Using default key pair: $KEY_NAME${NC}"
        echo -e "${YELLOW}⚠ Warning: Key file not found: $KEY_FILE${NC}"
        echo "The key pair exists in AWS but the private key file is missing locally."
        echo "You'll need the key file to SSH into instances."
        echo ""
    fi
    echo ""
else
    if [ -n "$EXISTING_KEYS" ]; then
        echo "Existing key pairs found:"
        aws ec2 describe-key-pairs --region "$REGION" --query 'KeyPairs[*].[KeyName,KeyPairId]' --output table
        echo ""
        if [ -n "$DEFAULT_KEY_NAME" ]; then
            echo "Default key '$DEFAULT_KEY_NAME' not found."
        else
            echo "No default key name specified (set AWS_KEY_NAME env var to specify one)."
        fi
        read -p "Enter key name to use (or press Enter to create new): " KEY_NAME
        if [ -z "$KEY_NAME" ]; then
            KEY_NAME="distributed-zkml-key-$(date +%s)"
            echo "Creating new key pair: $KEY_NAME"
            aws ec2 create-key-pair \
                --key-name "$KEY_NAME" \
                --region "$REGION" \
                --query 'KeyMaterial' \
                --output text > ~/.ssh/"$KEY_NAME".pem
            chmod 400 ~/.ssh/"$KEY_NAME".pem
            echo -e "${GREEN}✓ Created key pair: $KEY_NAME${NC}"
            echo "  Key saved to: ~/.ssh/$KEY_NAME.pem"
        fi
    else
        KEY_NAME="distributed-zkml-key-$(date +%s)"
        echo "No existing key pairs found. Creating: $KEY_NAME"
        aws ec2 create-key-pair \
            --key-name "$KEY_NAME" \
            --region "$REGION" \
            --query 'KeyMaterial' \
            --output text > ~/.ssh/"$KEY_NAME".pem
        chmod 400 ~/.ssh/"$KEY_NAME".pem
        echo -e "${GREEN}✓ Created key pair: $KEY_NAME${NC}"
        echo "  Key saved to: ~/.ssh/$KEY_NAME.pem"
    fi
fi

echo ""
export KEY_NAME

# 2. Security Group
DEFAULT_SG_NAME="${AWS_SECURITY_GROUP_NAME:-}"  # Set via AWS_SECURITY_GROUP_NAME env var, or leave empty to auto-detect/create
echo -e "${BLUE}2. Security Group${NC}"
echo "Checking for existing security groups..."
EXISTING_SGS=$(aws ec2 describe-security-groups \
    --region "$REGION" \
    --filters Name=ip-permission.from-port,Values=22 Name=ip-permission.to-port,Values=22 \
    --query 'SecurityGroups[*].[GroupId,GroupName]' \
    --output text 2>/dev/null || echo "")

# Check if default security group exists (if provided)
if [ -n "$DEFAULT_SG_NAME" ]; then
    DEFAULT_SG_ID=$(echo "$EXISTING_SGS" | grep -w "$DEFAULT_SG_NAME" | head -1 | awk '{print $1}')
    if [ -n "$DEFAULT_SG_ID" ]; then
        SECURITY_GROUP="$DEFAULT_SG_ID"
        echo -e "${GREEN}✓ Using default security group: $SECURITY_GROUP ($DEFAULT_SG_NAME)${NC}"
        echo ""
    else
        echo "Default security group '$DEFAULT_SG_NAME' not found."
        if [ -n "$EXISTING_SGS" ]; then
            echo "Security groups with SSH access found:"
            aws ec2 describe-security-groups \
                --region "$REGION" \
                --filters Name=ip-permission.from-port,Values=22 Name=ip-permission.to-port,Values=22 \
                --query 'SecurityGroups[*].[GroupId,GroupName,Description]' \
                --output table | head -20
            echo ""
            read -p "Enter security group ID to use (or press Enter to create new): " SECURITY_GROUP
            if [ -z "$SECURITY_GROUP" ]; then
                SG_NAME="distributed-zkml-test-$(date +%s)"
                echo "Creating new security group: $SG_NAME"
                SG_OUTPUT=$(aws ec2 create-security-group \
                    --group-name "$SG_NAME" \
                    --description "Security group for distributed-zkml GPU testing" \
                    --region "$REGION" 2>&1)
                
                if echo "$SG_OUTPUT" | grep -q "already exists"; then
                    SECURITY_GROUP=$(aws ec2 describe-security-groups \
                        --region "$REGION" \
                        --filters Name=group-name,Values="$SG_NAME" \
                        --query 'SecurityGroups[0].GroupId' \
                        --output text)
                else
                    SECURITY_GROUP=$(echo "$SG_OUTPUT" | jq -r '.GroupId' 2>/dev/null || echo "$SG_OUTPUT" | grep -o 'sg-[a-z0-9]*' | head -1)
                fi
                
                # Allow SSH
                aws ec2 authorize-security-group-ingress \
                    --group-id "$SECURITY_GROUP" \
                    --protocol tcp \
                    --port 22 \
                    --cidr 0.0.0.0/0 \
                    --region "$REGION" 2>/dev/null || echo "SSH rule may already exist"
                
                echo -e "${GREEN}✓ Created security group: $SECURITY_GROUP${NC}"
            fi
        else
            SG_NAME="distributed-zkml-test-$(date +%s)"
            echo "No security groups with SSH found. Creating: $SG_NAME"
            SG_OUTPUT=$(aws ec2 create-security-group \
                --group-name "$SG_NAME" \
                --description "Security group for distributed-zkml GPU testing" \
                --region "$REGION" 2>&1)
            
            if echo "$SG_OUTPUT" | grep -q "already exists"; then
                SECURITY_GROUP=$(aws ec2 describe-security-groups \
                    --region "$REGION" \
                    --filters Name=group-name,Values="$SG_NAME" \
                    --query 'SecurityGroups[0].GroupId' \
                    --output text)
            else
                SECURITY_GROUP=$(echo "$SG_OUTPUT" | jq -r '.GroupId' 2>/dev/null || echo "$SG_OUTPUT" | grep -o 'sg-[a-z0-9]*' | head -1)
            fi
            
            # Allow SSH
            aws ec2 authorize-security-group-ingress \
                --group-id "$SECURITY_GROUP" \
                --protocol tcp \
                --port 22 \
                --cidr 0.0.0.0/0 \
                --region "$REGION" 2>/dev/null || echo "SSH rule may already exist"
            
            echo -e "${GREEN}✓ Created security group: $SECURITY_GROUP${NC}"
        fi
    fi
else
    # No default security group name specified
    if [ -n "$EXISTING_SGS" ]; then
        echo "Security groups with SSH access found:"
        aws ec2 describe-security-groups \
            --region "$REGION" \
            --filters Name=ip-permission.from-port,Values=22 Name=ip-permission.to-port,Values=22 \
            --query 'SecurityGroups[*].[GroupId,GroupName,Description]' \
            --output table | head -20
        echo ""
        echo "No default security group name specified (set AWS_SECURITY_GROUP_NAME env var to specify one)."
        read -p "Enter security group ID to use (or press Enter to create new): " SECURITY_GROUP
        if [ -z "$SECURITY_GROUP" ]; then
            SG_NAME="distributed-zkml-test-$(date +%s)"
            echo "Creating new security group: $SG_NAME"
            SG_OUTPUT=$(aws ec2 create-security-group \
                --group-name "$SG_NAME" \
                --description "Security group for distributed-zkml GPU testing" \
                --region "$REGION" 2>&1)
            
            if echo "$SG_OUTPUT" | grep -q "already exists"; then
                SECURITY_GROUP=$(aws ec2 describe-security-groups \
                    --region "$REGION" \
                    --filters Name=group-name,Values="$SG_NAME" \
                    --query 'SecurityGroups[0].GroupId' \
                    --output text)
            else
                SECURITY_GROUP=$(echo "$SG_OUTPUT" | jq -r '.GroupId' 2>/dev/null || echo "$SG_OUTPUT" | grep -o 'sg-[a-z0-9]*' | head -1)
            fi
            
            # Allow SSH
            aws ec2 authorize-security-group-ingress \
                --group-id "$SECURITY_GROUP" \
                --protocol tcp \
                --port 22 \
                --cidr 0.0.0.0/0 \
                --region "$REGION" 2>/dev/null || echo "SSH rule may already exist"
            
            echo -e "${GREEN}✓ Created security group: $SECURITY_GROUP${NC}"
        fi
    else
        SG_NAME="distributed-zkml-test-$(date +%s)"
        echo "No security groups with SSH found. Creating: $SG_NAME"
        SG_OUTPUT=$(aws ec2 create-security-group \
            --group-name "$SG_NAME" \
            --description "Security group for distributed-zkml GPU testing" \
            --region "$REGION" 2>&1)
        
        if echo "$SG_OUTPUT" | grep -q "already exists"; then
            SECURITY_GROUP=$(aws ec2 describe-security-groups \
                --region "$REGION" \
                --filters Name=group-name,Values="$SG_NAME" \
                --query 'SecurityGroups[0].GroupId' \
                --output text)
        else
            SECURITY_GROUP=$(echo "$SG_OUTPUT" | jq -r '.GroupId' 2>/dev/null || echo "$SG_OUTPUT" | grep -o 'sg-[a-z0-9]*' | head -1)
        fi
        
        # Allow SSH
        aws ec2 authorize-security-group-ingress \
            --group-id "$SECURITY_GROUP" \
            --protocol tcp \
            --port 22 \
            --cidr 0.0.0.0/0 \
            --region "$REGION" 2>/dev/null || echo "SSH rule may already exist"
        
        echo -e "${GREEN}✓ Created security group: $SECURITY_GROUP${NC}"
    fi
fi

echo ""
export SECURITY_GROUP

# 3. Summary
echo -e "${GREEN}=== Setup Complete ===${NC}"
echo ""
echo "Export these environment variables:"
echo ""
echo -e "${YELLOW}export KEY_NAME=$KEY_NAME${NC}"
echo -e "${YELLOW}export SECURITY_GROUP=$SECURITY_GROUP${NC}"
echo ""

# Output export commands for eval (if script is being sourced/eval'd)
if [ "${BASH_SOURCE[0]}" != "${0}" ] || [ -n "$EVAL_MODE" ]; then
    echo "export KEY_NAME=$KEY_NAME"
    echo "export SECURITY_GROUP=$SECURITY_GROUP"
else
    echo "To set them automatically, run:"
    echo -e "${BLUE}export KEY_NAME=$KEY_NAME${NC}"
    echo -e "${BLUE}export SECURITY_GROUP=$SECURITY_GROUP${NC}"
    echo ""
    echo "Or copy the commands above."
    echo ""
    echo "Then run:"
    echo -e "${BLUE}./tests/aws/manage_aws_instance.sh${NC}"
fi

