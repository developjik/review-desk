# ReviewDesk

ReviewDesk is a macOS desktop workspace for reviewing GitHub pull requests with repeated analysis runs, editable review drafts, inline comment selection, and guarded publish flows.

## Install On macOS

Download the latest `ReviewDesk_*.dmg` from GitHub Releases:

https://github.com/developjik/review-desk/releases

Open the DMG and drag `ReviewDesk.app` into `Applications`.

The current public release is an unsigned developer preview. macOS Gatekeeper may require opening it from Finder with `Control` + click, then `Open`. For broad distribution without that warning, build with an Apple Developer ID certificate and notarization.

## Development

```bash
npm install
npm run desktop
```

## Release Build

Unsigned local DMG:

```bash
npm run desktop:release:unsigned
```

Signed/notarized DMG, when Apple signing credentials are configured:

```bash
npm run desktop:release
```

Release bundles are written under:

```text
src-tauri/target/release/bundle/
```
