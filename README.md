# Veydrak

My chef's choice reimagination of eDEX-UI: a sci-fi terminal dashboard built on
Tauri 2 (Rust) and Svelte 5.

- **Terminal**: tabbed PTY shells (portable-pty + xterm.js)
- **System monitor**: live CPU/memory history
- **LAN roster**: ARP + mDNS discovery, ICMP probing with hysteresis (3 failures to go offline, 2 successes to recover); pinned devices that go offline turn the whole UI red
- **Phosphor themes**: green / amber, red on alert

## Development

Prerequisites: Node 22, pnpm 10, Rust stable, and on Linux `libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev`.

```sh
pnpm install
pnpm tauri dev      # run the app
pnpm check && pnpm test
cd src-tauri && cargo clippy && cargo test
pnpm tauri build    # release bundle
```

ICMP probes need unprivileged ICMP sockets (Linux: `net.ipv4.ping_group_range` must include your group) or
`CAP_NET_RAW`. Without it, devices show as *unknown* rather than offline.

## Not yet wired

- Auto-update (needs signing keys; see `.github/workflows/release.yml`).

## Security hardening

- Refuses to start as root (Unix); override with `VEYDRAK_ALLOW_ROOT=1`.
- Terminal shells start with an allowlisted environment (no inherited tokens or API keys) and `TERM=xterm-256color`.
- Strict CSP, scoped Tauri capabilities, validated PTY ids.
- Release bundles: `deb`, `rpm`, `dmg`, `app`, `nsis` (no AppImage).
