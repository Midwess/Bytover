#!/bin/bash
set -euo pipefail

# Change to project root
cd "$(dirname "$0")/../.."

# Source production environment variables if they exist
if [ -f "scripts/desktop/prod-env.env" ]; then
  echo "--- Loading production environment variables ---"
  set -a
  source scripts/desktop/prod-env.env
  set +a
fi

# Usage function
usage() {
  echo "Usage: $0 <package-file-path>"
  echo ""
  echo "Uploads a desktop release package to S3 with public read access."
  echo ""
  echo "Arguments:"
  echo "  <package-file-path>  Path to the package file to upload"
  echo ""
  echo "Environment variables required:"
  echo "  AWS_ACCESS_KEY_ID    S3 access key"
  echo "  AWS_SECRET_ACCESS_KEY S3 secret key"
  echo "  AWS_S3_BUCKET_NAME  S3 bucket name (e.g., bytover)"
  echo "  AWS_ENDPOINT_URL    S3-compatible endpoint (e.g., https://nyc3.digitaloceanspaces.com)"
  exit 1
}

# Check arguments
if [ $# -lt 1 ]; then
  echo "--- Error: Missing package file path ---"
  usage
fi

PACKAGE_FILE="$1"

# Validate package file exists
if [ ! -f "$PACKAGE_FILE" ]; then
  echo "--- Error: Package file not found: $PACKAGE_FILE ---"
  exit 1
fi

# Validate required environment variables
MISSING_VARS=()
[ -z "${AWS_ACCESS_KEY_ID:-}" ] && MISSING_VARS+=("AWS_ACCESS_KEY_ID")
[ -z "${AWS_SECRET_ACCESS_KEY:-}" ] && MISSING_VARS+=("AWS_SECRET_ACCESS_KEY")
[ -z "${AWS_S3_BUCKET_NAME:-}" ] && MISSING_VARS+=("AWS_S3_BUCKET_NAME")
[ -z "${AWS_ENDPOINT_URL:-}" ] && MISSING_VARS+=("AWS_ENDPOINT_URL")

if [ ${#MISSING_VARS[@]} -gt 0 ]; then
  echo "--- Error: Missing required environment variables: ${MISSING_VARS[*]} ---"
  usage
fi

# Extract filename
FILENAME=$(basename "$PACKAGE_FILE")

# S3 destination path
S3_PATH="bytover/desktop/releases/$FILENAME"

echo "--- Uploading $FILENAME to S3 ---"
echo "--- Destination: s3://$AWS_S3_BUCKET_NAME/$S3_PATH ---"

# Upload to S3 with public-read ACL (allow script to continue to check result)
set +e
aws s3 cp "$PACKAGE_FILE" "s3://$AWS_S3_BUCKET_NAME/$S3_PATH" \
  --acl public-read \
  --endpoint-url "$AWS_ENDPOINT_URL"
AWS_RESULT=$?
set -e

if [ $AWS_RESULT -eq 0 ]; then
  echo "--- Upload complete ---"
  echo "--- Public URL: $AWS_ENDPOINT_URL/$AWS_S3_BUCKET_NAME/$S3_PATH ---"
else
  echo "--- Error: Upload failed with exit code $AWS_RESULT ---"
  exit 1
fi
