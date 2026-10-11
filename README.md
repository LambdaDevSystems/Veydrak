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


## Security hardening

- Refuses to start as root (Unix); override with `VEYDRAK_ALLOW_ROOT=1`.
- Terminal shells start with an allowlisted environment (no inherited tokens or API keys) and `TERM=xterm-256color`.
- Strict CSP, scoped Tauri capabilities, validated PTY ids.
- Release bundles: `deb`, `rpm`, `dmg`, `app`, `nsis` (no AppImage).

## Configuration

`~/.config/veydrak/config.json` (`$XDG_CONFIG_HOME/veydrak/`, or `%APPDATA%\veydrak\` on Windows) is created with defaults on first run:

```json
{
  "theme": "green",
  "layout": { "main": "terminal", "side": ["sysmon", "lan"] }
}
```

- `theme`: `green` or `amber` (the status bar toggle saves your choice here).
- `layout.main` / `layout.side`: panel ids from `terminal`, `sysmon`, `lan`. Each panel can appear once; side holds up to 4. Unknown ids are ignored.
- Missing keys use defaults. An unparseable file falls back to defaults, shows a warning in the status bar, and is never overwritten.
- The system panel shows battery charge when `/sys/class/power_supply` reports one (Linux).

## Updates

Signed updates come from GitHub Releases (`.../releases/latest/download/latest.json`). On launch (5 s after start) Veydrak checks for one; if found, the status bar offers **install & restart**. Nothing downloads or restarts without your click. Disable the check with `"check_updates": false` in `config.json`.

### Cutting a release

1. One-time: `pnpm tauri signer generate -w ~/.tauri/veydrak.key`; put the public key in `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`) and add the private key as the repo secret `TAURI_SIGNING_PRIVATE_KEY` (plus `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` if you set one). Back the private key up: if it is lost, installed copies cannot verify future updates.
2. Bump the version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` (the release workflow fails if they differ from the tag).
3. Commit, then `git tag vX.Y.Z && git push origin vX.Y.Z`.
4. The workflow builds Linux/macOS/Windows bundles and creates a **draft** release. Publish the draft to make the update visible to installed apps.

macOS bundles are not notarized; the update signature is separate from Apple code signing.

