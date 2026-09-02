#!/bin/bash
set -e

# ⚠️  MAINNET DEPLOYMENT - USE WITH EXTREME CAUTION ⚠️
#
# Prerequisites:
# 1. Contract has been audited
# 2. Extensive testing on testnet completed
# 3. Multi-sig or governance contract ready for admin role
# 4. soroban-cli configured with mainnet network
# 5. Identity with sufficient XLM for deployment fees

NETWORK="mainnet"
IDENTITY="deployer"
WASM_PATH="target/wasm32-unknown-unknown/release/chainbounty_optimized.wasm"

echo "⚠️  MAINNET DEPLOYMENT WARNING ⚠️"
echo "This will deploy to Stellar mainnet with real funds."
echo ""
read -p "Have you completed security audits? (yes/no): " AUDIT_CONFIRM
if [ "$AUDIT_CONFIRM" != "yes" ]; then
  echo "Deployment aborted. Complete audits before mainnet deployment."
  exit 1
fi

read -p "Type 'DEPLOY' to proceed: " CONFIRM
if [ "$CONFIRM" != "DEPLOY" ]; then
  echo "Deployment aborted."
  exit 1
fi

echo ""
echo "Deploying to Stellar mainnet..."

# Deploy contract
echo "Deploying contract..."
CONTRACT_ID=$(soroban contract deploy \
  --wasm "$WASM_PATH" \
  --source "$IDENTITY" \
  --network "$NETWORK")

echo "✓ Contract deployed to mainnet"
echo "  Contract ID: $CONTRACT_ID"

# Save contract ID
mkdir -p deployments
echo "$CONTRACT_ID" > deployments/${NETWORK}_contract_id.txt
echo "  Saved to: deployments/${NETWORK}_contract_id.txt"

echo ""
echo "⚠️  CRITICAL NEXT STEPS:"
echo "  1. Initialize with a secure admin (multi-sig recommended):"
echo "     soroban contract invoke \\"
echo "       --id $CONTRACT_ID \\"
echo "       --source $IDENTITY \\"
echo "       --network $NETWORK \\"
echo "       -- initialize \\"
echo "       --admin <MULTISIG_OR_GOVERNANCE_ADDRESS> \\"
echo "       --fee_bps <FEE_BPS>"
echo ""
echo "  2. Verify deployment and initialization"
echo "  3. Document contract address in production systems"
echo "  4. Set up monitoring and alerting"
