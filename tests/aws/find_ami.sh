#!/bin/bash
# Helper script to find Deep Learning AMI

REGION="${AWS_REGION:-us-west-2}"

echo "Finding Deep Learning AMIs in region: $REGION"
echo ""

# Use the working command format (this is the one that works!)
aws ec2 describe-images \
  --owners amazon \
  --filters Name=name,Values="*Deep Learning*" \
  --query "Images[*].[ImageId,Name,CreationDate]" \
  --output table \
  --region "$REGION" 2>&1 | grep -i pytorch | head -15 || true

echo ""
echo "Default AMI (Ubuntu 20.04, PyTorch 2.3.1):"
echo "  ami-0076e7fffffc9251d"
echo ""
echo "To use this default AMI:"
echo "  export AMI_ID=ami-0076e7fffffc9251d"
echo ""
echo "Or copy a different AMI ID from above and set:"
echo "  export AMI_ID=ami-xxxxx"

