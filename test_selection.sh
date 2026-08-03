#!/bin/bash

# Test script to simulate spotcheck interaction
# This tests the selection logic by sending keys to the TUI

cd /home/sal/spotcheck

# Build first
cargo build

# Test: Select from "nginx" to "443"
# The expected selection should be: "nginx[4421]: failed to bind port 443"

echo "Testing spotcheck selection..."
echo "Simulating: type 'nginx', press Enter, type '443', press Enter, press Enter"

# Use expect or similar would be better, but for now let's just document the manual test
echo "Manual test steps:"
echo "1. Run: ./target/debug/spotcheck"
echo "2. Type: nginx"
echo "3. Press Enter (to select start point)"
echo "4. Type: 443"
echo "5. Press Enter (to select end point)"
echo "6. Press Enter (to copy to clipboard)"
echo ""
echo "Expected selection: nginx[4421]: failed to bind port 443"
echo ""
echo "To test automatically, we'd need expect or a similar TUI automation tool"
