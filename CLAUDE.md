# CLAUDE.md — template-cloud-rust

Starter template for PhyStack **CLOUD** apps in Rust: a single server-side
process serving all installations of the app across every tenant. There is
no PhyStack SDK crate — `src/main.rs` talks to PhyHub's cloud-app HTTP
contract (`/v1/cloud-app/settings`, `/v1/cloud-app/events`) directly with
`reqwest` + `tokio`. Scaffolded by `phy app init <name> --type cloud --lang rust`.

## Commands

| Command | What it runs |
|---|---|
| `cargo build` / `cargo run` | Build / run the app (reads `.env`) |
| `bun run schema` | Copy `schema.json`, `meta-schema.json`, `analytics-schema.json`, `manifest.json` into `build/` |
| `bun run build` | `bun run schema && cargo build --release` |
| `bun run pub` | `bun run build && phy app build create $npm_package_name --dir . --publish` |

The `bun run` scripts exist only for the publish pipeline (package.json
carries PhyStack registration metadata, no Node dependencies); day-to-day
development is plain cargo.

## Dev loop

- No device simulator — cloud apps connect straight to PhyHub. Credentials
  live in `.env` (copy from `.env.example`): `APP_ID`, `APP_SECRET` (printed
  once by `phy app create`), `PHYHUB_URL` (local dev:
  `http://localhost:14400`), `RUST_LOG`.
- Schemas are hand-written JSON at the repo root (no ts-schema pipeline):
  `schema.json` (operator settings), `meta-schema.json` (Console UI hints),
  `analytics-schema.json` (emitted events). Edit them directly.

## Publish flow (new `phy` CLI grammar)

```bash
phy login
phy app create <name> --type cloud   # register (once), prints one-time credentials
bun run pub                          # stage schemas, submit + publish (no container image)
```

Publishing ships the schemas + manifest so the Console can render settings;
the process itself is hosted by you.

## Layout

| Path | Purpose |
|---|---|
| `src/main.rs` | Entrypoint — settings polling + event emission |
| `schema.json` / `meta-schema.json` / `analytics-schema.json` | Hand-written schemas (root, staged into `build/`) |
| `manifest.json` | App kind + runtime metadata |
| `package.json` | PhyStack registration metadata only |
| `.env.example` | Required environment variables |

## Gotchas

- `application-type` in package.json must stay `cloud`; the package.json
  `name` is the app name used by `pub` (`$npm_package_name`).
- Resilience contract: persist settings to your own store on every version
  bump (`fetch_settings` returns `Ok(true)` when settings move forward) and
  operate from that store — the HTTP contract does not own settings state.
- `build/` is generated output (`bun run schema`) and is git-ignored — never
  commit it.
- TypeScript sibling: [template-cloud-typescript](https://github.com/phystack/template-cloud-typescript)
  implements the same contract via `@phystack/hub-client`. Note its env vars
  use a `PHYSTACK_*` prefix while this template uses `APP_ID`/`APP_SECRET`/`PHYHUB_URL`.
