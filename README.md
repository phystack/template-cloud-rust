# Cloud App (Rust)

A Phystack cloud app written in Rust — a server-side service that receives
operator-configured settings and emits analytics events back to PhyHub.

There is no `@phystack/hub-cloud-app` Rust crate. This template makes raw
HTTPS calls to PhyHub's `/v1/cloud-app/events` and `/v1/cloud-app/settings`
using `reqwest` + `tokio`. The contract is the same as the TypeScript SDK —
just hand-rolled in ~150 lines.

## Setup

```bash
cp .env.example .env
# fill in APP_ID, APP_SECRET (from `phy app create`), and PHYHUB_URL
cargo build
cargo run
```

## What's in here

| File | Purpose |
|------|---------|
| `src/main.rs` | Entrypoint — wires settings polling + event emission |
| `Cargo.toml` | Rust deps (tokio, reqwest, serde, chrono, tracing) |
| `schema.json` | Operator-editable settings schema |
| `meta-schema.json` | UI hints |
| `analytics-schema.json` | Event types this app emits |
| `manifest.json` | App-store metadata |
| `package.json` | Phystack registration metadata (no Node deps) |

## Resilience contract

Persist settings to your own store on **every** version bump (the
`fetch_settings` call returns `Ok(true)` when settings move forward) and
operate from your own store at runtime. The HTTP contract delivers events;
it does not own settings state.
