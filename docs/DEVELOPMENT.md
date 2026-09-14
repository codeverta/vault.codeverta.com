# Vault developer guide

This guide covers the codebase, local development workflow, and rules for security-sensitive changes.

## Architecture

```text
React + TypeScript + Tailwind
          │ Tauri invoke commands
Rust security boundary
          │
SQLite (ciphertext payloads only)
```

The frontend never opens the database or derives the master key. It calls the small command surface in `src-tauri/src/lib.rs`. Rust owns password derivation, encryption/decryption, database access, file dialogs, export/import, and the in-memory key lifecycle.

## Local setup

Install Node.js 20+, Rust stable, and the Tauri prerequisites for your operating system. Then run:

```bash
npm install
npm run tauri dev
```

## Useful commands

```bash
npm run check
npm run build
npm run tauri dev
npm run tauri build

cargo fmt --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## Adding a vault item field

1. Add the field to `VaultItem` and `ItemInput` in Rust with a safe default for backward compatibility.
2. Add it to `VaultItem`/`ItemDraft` in `src/types.ts`.
3. Update the create/edit form in `src/App.tsx`.
4. Keep the field inside the encrypted JSON payload. Do not add plaintext searchable columns.
5. Add or update a focused Rust test when the field changes serialization or encryption behavior.

## Security rules

- Never log passwords, secrets, decrypted item payloads, database contents, or backup contents.
- Keep sensitive values in Rust for as short a time as possible and use `Zeroizing` for key material.
- Use a fresh random nonce for every AES-GCM encryption operation.
- Treat clipboard contents and exported files as sensitive; keep cleanup and password prompts intact.
- Do not add network calls, analytics, remote fonts, or third-party scripts to the application.
- Do not weaken the Content Security Policy to work around a frontend issue.

## Testing expectations

Every change should pass TypeScript checks and the Rust test suite. Changes in the security boundary should also pass Clippy with warnings denied. Test tampering, wrong passwords, lock/unlock transitions, and import collisions when those paths are affected.

## Pull requests

Explain the user-facing behavior, security impact, and commands you ran. Keep real credentials, private keys, `.vault` files, and local SQLite databases out of commits and screenshots.
