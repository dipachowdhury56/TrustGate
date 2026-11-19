# TrustGate | Milestone-Gated Escrow with Dispute Arbitration on Soroban

TrustGate is a decentralized stage-gated escrow protocol engineered for Web3 freelance agreements, contractor retainers, and cross-party service deals. Built natively for Stellar's Soroban smart contract framework, it protects both parties: sellers are guaranteed timely payments or auto-release if clients go unresponsive, while buyers only release capital when verified deliverable hashes are provided.

---

## 💡 Ecosystem Prior Art & How TrustGate is Differentiated

| Feature | Standard Escrow (Soroban examples / all-or-nothing) | **TrustGate (Our Implementation)** |
| :--- | :--- | :--- |
| **Payment Structure** | 100% upfront lock; single release or full refund | **Multi-Milestone Stage Gating**: Single contract holds $N$ independent milestones with separate budgets and deliverables. |
| **Deliverable Proof** | Offchain trust; no proof anchored to contract | **Onchain Deliverable Hash**: Seller anchors IPFS CID / proof URI to the milestone state onchain before requesting approval. |
| **Ghosting Protection** | If buyer ghosts, seller funds are permanently locked | **Automated Grace Period Release**: If review window (e.g. 7 days) elapses without buyer response, seller can trigger self-claim. |
| **Dispute Resolution** | Binary all-to-buyer or all-to-seller | **Granular Split Arbitration**: Neutral arbiter can allocate proportional splits (e.g. 70% to seller for partial delivery, 30% refund to buyer). |

---

## 🛠 Project Structure

```
project-2-trustgate-escrow/
├── contracts/
│   └── trustgate/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs       # Escrow contract, state transitions, arbitration
│       │   ├── types.rs     # Data structures (EscrowAgreement, Milestone, Status)
│       │   └── test.rs      # Unit test suite (multi-milestone, timeouts, arbitration)
├── client/
│   ├── package.json         # Node dependencies (@stellar/stellar-sdk)
│   ├── test-client.js       # Client simulation & parameter encoding test
│   └── index.html           # Live interactive multi-role Escrow dashboard
└── README.md
```

---

## 🚀 Building & Testing

### 1. Run Smart Contract Tests
```bash
cd contracts/trustgate
cargo test
```
Verifies:
- Multi-milestone creation and sequential approval.
- Grace period timeout auto-release mechanism.
- Dispute triggering and arbiter partial split settlements.

### 2. Build the WASM Binary
```bash
stellar contract build
```
Compiled artifact generated at:
`target/wasm32v1-none/release/trustgate.wasm`

### 3. Run the Client SDK Verification
```bash
cd client
npm install
npm test
```

### 4. Interactive Web UI
Open `client/index.html` in your browser. Switch roles between **Buyer**, **Seller**, and **Arbiter** to test real-time milestone submissions, approvals, auto-release timers, and dispute splits.
