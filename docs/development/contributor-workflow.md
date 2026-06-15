# Contributor workflow

Everything a new contributor needs from clone to merged PR. The root
`CONTRIBUTING.md` is the short version; this is the detailed reference.

## 1. Setup

```bash
git clone https://github.com/kodyka/fcast-android-sender
cd fcast-android-sender

# Recommended: reproducible Android shell (SDK + NDK + cargo-ndk + adb)
nix develop .#android -L

# Install pre-commit hooks
pip install pre-commit
pre-commit install
```

Manual (non-Nix) setup is documented in
[../build/android-build.md](../build/android-build.md#option-b--manual-environment).

## 2. Pre-commit hooks

The hooks in `.pre-commit-config.yaml` enforce UI conventions:

- No raw hex colors in Slint files — use `Theme.*` tokens.
- No hardcoded `font-size: Npx` — use `Theme.font-size-*`.
- No direct `Bridge.active-panel` writes — use `PanelBridge.push` / `.pop`.
- No direct `Bridge.lifecycle` writes — use lifecycle callbacks.

Run them on demand:

```bash
pre-commit run --all-files
```

## 3. Branch naming

```text
feat/gstpop-backend-toggle
fix/camera-permission-result
docs/repo-layout-refresh
chore/sdk-pin-bump
ci/slint-smoke-cache
```

Branch from `main`; PRs require green CI before merge.

## 4. Code style

| Language | Tooling |
|---|---|
| Rust | `cargo fmt` + `cargo clippy` |
| Kotlin | follow existing patterns in `app/src/` |
| Slint | follow `docs/guides/ui-slint-best-practices/` |

## 5. Test matrix

```bash
# Rust
cargo fmt --check
cargo clippy --all-targets
cargo test                               # unit tests
cargo test --test ui_snapshots           # headless Slint snapshots

# UI lint suite
ci/ui-validate.sh

# JVM (Kotlin/Java) unit tests
./gradlew :app:testDebugUnitTest

# Instrumented tests (requires emulator/device)
./gradlew :app:connectedDebugAndroidTest

# Native Android build sanity
cargo check --target aarch64-linux-android

# Repo layout health (new)
bash scripts/check-repo-layout.sh
```

## 6. JNI symbol stability

Exported symbols (`Java_org_fcast_android_sender_*` in `src/lib.rs`) are part of
the app ABI. If a change adds, removes, or renames a symbol, update the baseline
and explain why in the PR:

```bash
# Inspect what changed, then refresh the baseline the symbol-stability
# workflow diffs against:
$EDITOR ci/jni-symbol-baseline.txt
```

## 7. Pull request checklist

```md
## Summary
- 

## Changed areas
- [ ] Android shell   - [ ] Rust runtime   - [ ] JNI bridge
- [ ] Slint UI        - [ ] GStreamer      - [ ] Gradle/build
- [ ] Nix/dev shell   - [ ] CI             - [ ] Docs only

## Validation
- [ ] `cargo test`
- [ ] `ci/ui-validate.sh`
- [ ] `./gradlew :app:testDebugUnitTest`
- [ ] `bash scripts/check-repo-layout.sh`
- [ ] Android device smoke test (or N/A for docs-only)

## Notes
- 
```

## 8. Commit message examples

```text
docs: reorganize migration guides under docs/migrations
ci: add repository layout health check
refactor(android): extract camera preview host
fix(jni): keep native symbol baseline in sync
chore(sdk): bump fcast SDK pin to 1234abc
```

## 9. CI/CD reference (7 + 1 workflows)

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `android-debug-apk` | push, PR | UI audit + arm64 debug APK + JVM unit tests |
| `android-release-apk` | release published, manual | Signed release APK attached to the GitHub Release |
| `android-instrumented-tests` | PR (`app/src` changes), nightly | Emulator instrumented tests |
| `symbol-stability` | PR (`src`/`crates`/`app` changes) | JNI symbol baseline diff |
| `ui-lint` | PR (`ui/` changes) | Forbid raw hex, hardcoded sizes, direct state writes |
| `slint-viewer-smoke` | PR (`ui/` changes) | Compile `ui/main.slint` with `slint-viewer` |
| `gstpop-smoke` | push, PR (`src`/`ui`/`ci`) | gst-pop backend integration tests |
| `repo-health` *(new)* | push, PR (docs/scripts/README) | Layout drift + shellcheck + link check |
