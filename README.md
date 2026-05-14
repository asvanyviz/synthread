# Synthread

**P2P framework with plugin system in Rust.**

Synthread is a peer-to-peer communication framework built on libp2p, designed for private, encrypted messaging between humans and agents. It features a modular plugin architecture, KDE Plasma desktop integration, and a headless mode for servers.

> 🚧 **Status:** Early development (Phase 0 — Scaffolding)

## Architecture

```
┌────────────────────────────────────────────────┐
│              FRONTEND LAYER                     │
│  GUI App (Qt 6 + Kirigami)  │  Headless (TUI)  │
│  "human" mode               │  "server" mode   │
└──────────────┬───────────────┴─────────────────┘
               │
┌──────────────▼─────────────────────────────────┐
│              CORE LIBRARY                       │
│  Identity │ DHT │ Peer Manager │ Plugin Manager │
│  Security Layer (Visibility │ Encryption)       │
│  Network Layer (libp2p transport)               │
└──────────────┬─────────────────────────────────┘
               │
┌──────────────▼─────────────────────────────────┐
│              PLUGIN LAYER                       │
│  Chat DM (MVP) │ Forum │ Agent Wiki (future)    │
└────────────────────────────────────────────────┘
```

## Features (Planned)

- **E2EE Direct Messages** — X25519 + ChaCha20-Poly1305 with PFS
- **KDE Plasma Integration** — Qt 6 + Kirigami native GUI
- **Headless Mode** — TUI (Ratatui) + embedded WebUI
- **Plugin System** — Isolated, permissioned plugin architecture
- **F2F Friendship** — Friend-based social graph, no global namespace
- **Offline Messages** — DHT inbox pointers, encrypted local queue
- **NAT Traversal** — AutoNAT + hole punching + relay fallback

## Quick Start

```bash
# Build
cargo build

# Run (GUI mode — desktop detected automatically)
cargo run

# Run (headless with WebUI)
cargo run -- --mode headless --port 7700
```

## Development

```bash
# Run tests
cargo test

# Format
cargo fmt

# Lint
cargo clippy
```

## License

AGPL-3.0 — see [LICENSE](LICENSE)
