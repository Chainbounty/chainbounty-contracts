#!/bin/bash
set -e

# Prerequisites:
# 1. soroban-cli installed with network configured
# 2. Identity configured: soroban keys generate deployer
# 3. Funded testnet account

NETWORK="testnet"
IDENTITY="deployer"
WASM_PATH="target/wasm32-unknown-unknown/release/chainbounty_optimized.wasm"

echo "Deploying to Stellar $NETWORK..."

# Deploy contract
echo "Deploying contract..."
CONTRACT_ID=$(soroban contract deploy \
  --wasm "$WASM_PATH" \
  --source "$IDENTITY" \
  --network "$NETWORK")

echo "✓ Contract deployed"
echo "  Contract ID: $CONTRACT_ID"

# Save contract ID for reference
mkdir -p deployments
echo "$CONTRACT_ID" > deployments/${NETWORK}_contract_id.txt
echo "  Saved to: deployments/${NETWORK}_contract_id.txt"

echo ""
echo "Next steps:"
echo "  1. Initialize the contract with admin and fee_bps:"
echo "     soroban contract invoke \\"
echo "       --id $CONTRACT_ID \\"
echo "       --source $IDENTITY \\"
echo "       --network $NETWORK \\"
echo "       -- initialize \\"
echo "       --admin <ADMIN_ADDRESS> \\"
echo "       --fee_bps 500"
echo ""
echo "  2. Verify deployment:"
echo "     soroban contract invoke \\"
echo "       --id $CONTRACT_ID \\"
echo "       --source $IDENTITY \\"
echo "       --network $NETWORK \\"
echo "       -- bounty_count"
