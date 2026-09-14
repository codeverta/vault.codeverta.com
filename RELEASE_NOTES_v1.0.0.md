# Vault v1.0.0

The first stable release of Vault: a secure, offline-first desktop vault for secrets and sensitive text.

## Included

- Local vault creation and unlock with an Argon2id-derived master key.
- AES-256-GCM encryption for every stored item, with a unique nonce per item.
- Support for API keys, secret keys, access tokens, passwords, SSH private keys, recovery codes, license keys, environment variables, secure notes, plain text, JSON, and .env data.
- Search, categories, tags, favorites, reveal/hide, copy, edit, and delete.
- Password-protected .vault export and import for device migration.
- Five-minute inactivity auto-lock and 30-second clipboard cleanup after copying a secret.
- Signed Tauri release artifacts and updater metadata.

## Security posture

Vault is 100% offline by design. No account, telemetry, sync service, or cloud database is required. Master passwords are never stored, and the encryption key is cleared from memory when the vault is locked.

## Verification

- TypeScript production build passed.
- Rust cargo check passed.
- Rust unit tests passed: encryption round-trip, unique nonce, and tamper rejection.
- Rust Clippy passed with warnings treated as errors.
