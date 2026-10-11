# Security policy

## Reporting a vulnerability

Please **do not open a public issue**. Use GitHub's private vulnerability reporting:
the **Security** tab -> **Report a vulnerability**. I'll acknowledge it as soon as I can and keep you
updated until it is fixed.

## Supported versions

Only the latest release receives security fixes.

## Notes

- Veydrak refuses to run as root and starts terminal shells with an allowlisted environment.
- Updates are signed; the app only installs updates verified against the public key embedded in
  `src-tauri/tauri.conf.json`. The private signing key is stored offline and as a protected
  `release` environment secret. If you believe it has been exposed, report it via the channel above.
- The Home Assistant access token is kept in the OS keyring (or `VEYDRAK_HA_TOKEN`), never in `config.json`.
