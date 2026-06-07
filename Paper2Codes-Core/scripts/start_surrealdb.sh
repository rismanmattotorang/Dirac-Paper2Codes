#!/bin/bash
# Start SurrealDB for Paper2Codes

set -e

echo "Starting SurrealDB..."

# Check if SurrealDB is installed
if ! command -v surreal &> /dev/null; then
    echo "❌ SurrealDB is not installed"
    echo "Install from: https://surrealdb.com/docs/installation"
    exit 1
fi

# Create data directory
mkdir -p ./data

# Start SurrealDB
echo "✓ Starting SurrealDB on ws://127.0.0.1:8000"
echo "  Namespace: paper2codes"
echo "  Database: main"
echo "  Username: root"
echo "  Password: root"
echo

surreal start \
    --bind 0.0.0.0:8000 \
    --user root \
    --pass root \
    --log trace \
    file://./data/surrealdb

echo
echo "SurrealDB stopped"
