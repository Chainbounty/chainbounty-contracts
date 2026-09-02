# ChainBounty Contract Deployments

This file tracks deployed contract addresses across networks.

## Testnet (Stellar Test Network)

**Status:** Not yet deployed

**When deployed, record here:**

```
Contract ID: <TESTNET_CONTRACT_ID>
Network: testnet
RPC: https://soroban-testnet.stellar.org
Network Passphrase: Test SDF Network ; September 2015
Deployed: <YYYY-MM-DD>
Deployer: <DEPLOYER_ADDRESS>
Admin: <ADMIN_ADDRESS>
Platform Fee: <FEE_BPS> bps (<FEE_PERCENT>%)
```

**Initialization Transaction:**
```
Transaction Hash: <TX_HASH>
Initialization Parameters:
  - admin: <ADMIN_ADDRESS>
  - fee_bps: <FEE_BPS>
```

**Verification:**
- [ ] Contract deployed successfully
- [ ] Contract initialized with correct admin
- [ ] `bounty_count()` returns 0
- [ ] Test bounty posted and completed
- [ ] Events emitted correctly
- [ ] Fee calculation verified

**Explorer Links:**
- Contract: `https://stellar.expert/explorer/testnet/contract/<CONTRACT_ID>`
- Init Transaction: `https://stellar.expert/explorer/testnet/tx/<TX_HASH>`

---

## Mainnet (Stellar Public Network)

**Status:** Not deployed

⚠️ **Mainnet deployment requires:**
- [ ] Complete security audit
- [ ] Extensive testnet testing (min 30 days)
- [ ] Multi-sig or governance contract for admin
- [ ] Monitoring and alerting infrastructure
- [ ] Incident response plan
- [ ] Legal and compliance review

**When deployed, record here:**

```
Contract ID: <MAINNET_CONTRACT_ID>
Network: mainnet
RPC: https://soroban-mainnet.stellar.org
Network Passphrase: Public Global Stellar Network ; September 2015
Deployed: <YYYY-MM-DD>
Deployer: <DEPLOYER_ADDRESS>
Admin: <ADMIN_ADDRESS> (multi-sig recommended)
Platform Fee: <FEE_BPS> bps (<FEE_PERCENT>%)
```

**Audit Reports:**
- [ ] Smart contract security audit: <LINK_TO_REPORT>
- [ ] Economic model review: <LINK_TO_REPORT>

**Initialization Transaction:**
```
Transaction Hash: <TX_HASH>
Initialization Parameters:
  - admin: <ADMIN_ADDRESS>
  - fee_bps: <FEE_BPS>
```

**Explorer Links:**
- Contract: `https://stellar.expert/explorer/public/contract/<CONTRACT_ID>`
- Init Transaction: `https://stellar.expert/explorer/public/tx/<TX_HASH>`

---

## Deployment Checklist

### Pre-Deployment (Both Networks)

- [ ] Build optimized WASM: `./scripts/build-contracts.sh`
- [ ] All tests passing: `cargo test`
- [ ] Rust toolchain version matches `rust-toolchain.toml`
- [ ] Soroban SDK version matches `Cargo.toml`
- [ ] README documentation up to date
- [ ] Deployer identity configured and funded

### Testnet Deployment

```bash
./scripts/deploy-testnet.sh
```

After deployment:
1. Record contract ID in this file
2. Initialize contract with test admin
3. Run verification tests
4. Update README with testnet contract address

### Mainnet Deployment

**DO NOT DEPLOY TO MAINNET WITHOUT:**
- Complete security audit (see Security Notes in README)
- Multi-sig admin setup
- Legal and compliance clearance

```bash
./scripts/deploy-mainnet.sh
```

After deployment:
1. Record contract ID in this file
2. Initialize with multi-sig admin
3. Run comprehensive verification
4. Set up monitoring and alerting
5. Announce contract address to community

---

## Version History

| Version | Network | Contract ID | Deployed | Notes |
|---------|---------|-------------|----------|-------|
| 0.1.0   | testnet | TBD         | TBD      | Initial testnet deployment |
| 1.0.0   | mainnet | TBD         | TBD      | Audited mainnet release |

---

## Integration Resources

### Testnet

- **Horizon API:** https://horizon-testnet.stellar.org
- **RPC Endpoint:** https://soroban-testnet.stellar.org
- **Friendbot (Funding):** https://friendbot.stellar.org
- **Laboratory:** https://laboratory.stellar.org

### Mainnet

- **Horizon API:** https://horizon.stellar.org
- **RPC Endpoint:** https://soroban-mainnet.stellar.org
- **StellarExpert:** https://stellar.expert/explorer/public
- **Laboratory:** https://laboratory.stellar.org (use public network)
