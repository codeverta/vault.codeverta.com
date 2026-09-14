# Contributing to Vault

Thanks for helping improve Vault. Keep changes small, explain the user-facing impact, and include the relevant validation command in the pull request description.

Before opening a pull request:

```bash
npm install
npm run check
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Never commit credentials, tokens, private keys, real vault exports, or local database files. Security-sensitive changes should include a short threat-model note in the pull request.
