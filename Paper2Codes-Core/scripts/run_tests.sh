#!/bin/bash
# Run Paper2Codes Test Suite

set -e

echo "========================================="
echo "Paper2Codes Test Suite"
echo "========================================="
echo

# Run cargo tests
echo "Running unit tests..."
cargo test --lib

echo
echo "Running integration tests..."
cargo test --test '*'

echo
echo "Running doc tests..."
cargo test --doc

echo
echo "========================================="
echo "All Tests Passed!"
echo "========================================="

