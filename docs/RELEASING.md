# Publishing a Vault release

Vault releases are published by GitHub Actions from semantic version tags. The workflow builds signed Tauri artifacts for macOS, Linux, and Windows in parallel.

## One-time signing setup

The private `rsign` key must never be committed. Add these repository secrets:

- `TAURI_SIGNING_PRIVATE_KEY`: complete contents of the private key file.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: passphrase for that encrypted key.

Using GitHub CLI:

```bash
gh auth login
gh secret set TAURI_SIGNING_PRIVATE_KEY -R codeverta/vault.codeverta.com < .tauri/codeverta-erp.key
read -rsp "Signing key password: " KEY_PASSWORD
printf '%s' "$KEY_PASSWORD" | gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD -R codeverta/vault.codeverta.com
unset KEY_PASSWORD
```

The public key is committed in `src-tauri/tauri.conf.json` and is used by the updater to verify signatures.

## Publish a new version

1. Update the version in `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`.
2. Add `RELEASE_NOTES_vX.Y.Z.md` with highlights, security notes, and verification results.
3. Run the local checks:

   ```bash
   npm run check
   npm run build
   cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
   cargo test --manifest-path src-tauri/Cargo.toml
   cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
   ```

4. Commit the release and create an annotated tag:

   ```bash
   git add .
   git commit -m "release: Vault vX.Y.Z"
   git tag -a vX.Y.Z -m "Vault vX.Y.Z"
   git push origin main
   git push origin vX.Y.Z
   ```

5. Verify the workflow is green and the release contains macOS `.dmg`, Linux `.AppImage`/`.deb`, Windows `.msi`/`.exe`, and updater signature files when signing secrets are configured.

## Publish from the existing workspace

You do not need to clone or download the repository again. Run these commands from the workspace that already contains Vault:

```bash
npm run release:local
npm run release:ci
```

`release:local` builds the current operating system and uploads its installers to GitHub Releases. `release:ci` asks GitHub Actions to build macOS, Linux, and Windows from the current `main` branch.

Both commands derive the tag from the version in `package.json`. To target a specific version explicitly:

```bash
npm run release:local -- --tag=v1.0.0
npm run release:ci -- --tag=v1.0.0
```

`release:local` requires `gh auth login` and publishes only artifacts available on the current machine. `release:ci` requires the signing secrets in the repository and lets GitHub's runners build all three desktop platforms.

For a signed local build, export the encrypted key and its passphrase only in the current shell before running `release:local`:

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(< .tauri/codeverta-erp.key)"
read -rsp "Signing key password: " KEY_PASSWORD
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$KEY_PASSWORD"
npm run release:local
unset TAURI_SIGNING_PRIVATE_KEY TAURI_SIGNING_PRIVATE_KEY_PASSWORD KEY_PASSWORD
```

The private key is read from the existing local file and is never written to the repository by this workflow.

## Re-running a release

The workflow has a manual `workflow_dispatch` trigger. Run it from `main` and pass the release tag; this matters when rebuilding an older tag with a newer workflow:

```bash
gh workflow run release.yml -R codeverta/vault.codeverta.com --ref main -f release_tag=vX.Y.Z
```

This is useful after adding signing secrets or recovering a failed platform build. Do not create a second tag for the same version.

## Release checklist

- [ ] Version files agree on `X.Y.Z`.
- [ ] Release notes are present.
- [ ] TypeScript, Rust tests, fmt, and Clippy pass.
- [ ] CI on `main` is green.
- [ ] Signing secrets are configured before publishing signed artifacts.
- [ ] macOS, Linux, and Windows assets are present on the release.
- [ ] `.sig` and `latest.json` exist when updater signing is enabled.
- [ ] The release page and README download links work.
