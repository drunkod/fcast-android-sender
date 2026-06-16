# Contributing to fcast-android-sender

Thanks for contributing! This is the short version; the detailed reference lives
in [docs/development/contributor-workflow.md](docs/development/contributor-workflow.md).

## Development setup

1. Install Nix (recommended) or set up the Android SDK/NDK manually — see
   [docs/build/android-build.md](docs/build/android-build.md).
2. Enter the dev shell: `nix develop .#android -L`
3. Build and deploy: `./scripts/build-deploy.sh`
4. Install hooks: `pip install pre-commit && pre-commit install`

## Pre-commit hooks

The hooks in `.pre-commit-config.yaml` enforce UI conventions:

- No raw hex colors in Slint files (use `Theme.*` tokens)
- No hardcoded `font-size: Npx` (use `Theme.font-size-*`)
- No direct `Bridge.active-panel` writes (use `PanelBridge.push`/`.pop`)
- No direct `Bridge.lifecycle` writes (use lifecycle callbacks)

## Branches & PRs

- Branch from `main`: `feat/...`, `fix/...`, `docs/...`, `chore/...`, `ci/...`
- PRs require passing CI before merge
- If a change alters exported JNI symbols, update `ci/jni-symbol-baseline.txt`
  (the `symbol-stability` workflow diffs against it)

## Code style

- **Rust:** `cargo fmt` + `cargo clippy`
- **Kotlin:** follow existing patterns in `app/src/`
- **Slint:** follow [docs/guides/ui-slint-best-practices/](docs/guides/ui-slint-best-practices/)

## Testing

```bash
cargo test                          # Rust unit tests
cargo test --test ui_snapshots      # headless Slint snapshots
ci/ui-validate.sh                   # UI lint suite
./gradlew :app:testDebugUnitTest    # JVM unit tests
bash scripts/check-repo-layout.sh   # repo-health guard
```
