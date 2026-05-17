# GitHub Release Distribution

Date: 2026-05-17

## Repository

- GitHub repository: `developjik/review-desk`
- Release tag format: `v0.1.0`
- Primary macOS asset: `ReviewDesk_*.dmg`

## Local Release Build

Run verification before creating release assets:

```bash
npm run lint
npm test
cargo test
cargo test --manifest-path src-tauri/Cargo.toml
```

Build an unsigned developer-preview DMG:

```bash
npm run desktop:release:unsigned
```

Build with the normal Tauri signing path after Apple credentials are configured:

```bash
npm run desktop:release
```

Generated assets are placed under:

```text
src-tauri/target/release/bundle/
```

## GitHub Release

Create or update a release from a tag:

```bash
git tag v0.1.0
git push origin main
git push origin v0.1.0
gh release create v0.1.0 src-tauri/target/release/bundle/dmg/*.dmg \
  --repo developjik/review-desk \
  --title "ReviewDesk v0.1.0" \
  --notes-file docs/releases/v0.1.0.md
```

If the release already exists:

```bash
gh release upload v0.1.0 src-tauri/target/release/bundle/dmg/*.dmg \
  --repo developjik/review-desk \
  --clobber
```

## Signing And Notarization

The current release scripts can produce an unsigned DMG for early testers. For broad user distribution, configure Apple Developer ID signing and notarization so users can open the app without Gatekeeper workarounds.

Required outside this repository:

- Apple Developer Program membership
- `Developer ID Application` certificate installed in Keychain
- App Store Connect API key or notarization credentials
- Tauri signing/notarization environment variables in local shell or GitHub Actions secrets

Keep certificates, API keys, issuer IDs, and passwords out of git.
