## Repository Audit: `kodyka/fcast-android-sender`

### Current Root Structure

```
.claude/                          # AI assistant commands
.github/                          # 7 workflows + 1 composite action
TODO.codecs/                      # 17 codec task files (dot-prefix, non-standard)
app/                              # Android Gradle module (JNI, Java/Kotlin)
ci/                               # CI build scripts + JNI symbol baseline
crates/                           # Rust sub-crates (migration-runtime, gstpop-runtime)
docs/                             # ~120+ markdown files across 10+ subdirs
draft/                            # ~200+ research/planning files (moblin-ui, slint-ui)
scripts/                          # Dev helper scripts
src/                              # Main Rust crate (~18 modules)
tests/                            # UI snapshot tests
ui/                               # Slint UI tree
vendor/                           # Vendored gstpop crate
.gitlab-ci.yml                    # DEPRECATED — kept for emergency rollback
.pre-commit-config.yaml
Cargo.toml / Cargo.lock
build.gradle / settings.gradle
flake.nix
gradlew / gradlew.bat
LICENSE / README.md
```

### Issues Found

| # | Category | Issue | Severity |
|---|----------|-------|----------|
| 1 | Structure | `TODO.codecs/` uses non-standard dot-prefix naming at root | Medium |
| 2 | Structure | `draft/` contains **~200+ files** of historical research — most completed phases still committed | High |
| 3 | Structure | `docs/` has **~120+ files** mixing active guides, draft plans, and completed migration docs | High |
| 4 | Structure | No `.gitignore` file | High |
| 5 | Docs | No `CONTRIBUTING.md`, `CHANGELOG.md`, or `SECURITY.md` | Medium |
| 6 | README | Missing: CI badges, architecture diagram, project status, contributing link | Medium |
| 7 | CI/CD | `.gitlab-ci.yml` deprecated but still at root, adds confusion | Low |
| 8 | Structure | `docs/` has loose files (`draft-plan-*.md`) mixed with deep subdirectories | Medium |
| 9 | Structure | `draft/slint-ui/docs/` mirrors upstream Slint documentation — ~50 files that don't belong in-repo | High |
| 10 | Build | `app/build.gradle` references `libs.plugins.*` / `libs.*` but no `gradle/libs.versions.toml` found in index | Medium | [1-cite-0](#1-cite-0) [1-cite-1](#1-cite-1) [1-cite-2](#1-cite-2) [1-cite-3](#1-cite-3) 

---

### Proposed Revised Folder Structure

```
fcast-android-sender/
├── .github/
│   ├── actions/android-ci-setup/   # (keep as-is)
│   ├── workflows/                  # (keep as-is, 7 workflows)
│   ├── CONTRIBUTING.md             # NEW
│   └── SECURITY.md                 # NEW
├── .claude/                        # (keep as-is)
├── app/                            # Android Gradle module (unchanged)
├── ci/                             # CI scripts (unchanged)
├── crates/                         # Rust sub-crates (unchanged)
├── docs/
│   ├── architecture.md             # NEW — high-level arch overview
│   ├── cross-repo-sync.md          # (keep)
│   ├── ui-testing-tutorial.md      # (keep)
│   ├── adr/                        # NEW — Architecture Decision Records
│   │   └── 001-extract-from-monorepo.md
│   ├── guides/                     # NEW — active how-to guides
│   │   ├── slintcn-migration/      # (move from docs/slintcn-migration/)
│   │   ├── streampack-migration/   # (move from docs/streampack-migration/)
│   │   └── ui-slint-best-practices/# (move from docs/ui-slint-best-practices/)
│   ├── plans/                      # NEW — active implementation plans
│   │   ├── v0.1.0/                 # (move from docs/v0.1.0/)
│   │   ├── v0.2.0/                 # (move from docs/v0.2.0/)
│   │   └── codecs/                 # (move from TODO.codecs/)
│   └── archive/                    # NEW — completed/historical docs
│       ├── refactor-implementation-guide/  # (move, 11 chapters done)
│       ├── plan-android-service-gst-perf-test/
│       ├── plan-codecs-test-page/
│       ├── plan-gst-performance-test/
│       └── draft-plans/            # (move loose draft-plan-*.md files)
├── draft/                          # SLIM DOWN — keep only active research
│   ├── README.md
│   ├── moblin-*.md                 # (keep 3 mapping files)
│   └── slint-ui/
│       ├── phases/                 # (keep active phases only)
│       │   ├── README.md
│       │   ├── STATUS.md
│       │   └── MVP-PHASE-*.md      # active MVP phases
│       └── analysis/
├── scripts/                        # (unchanged)
├── src/                            # (unchanged)
├── tests/                          # (unchanged)
├── ui/                             # (unchanged)
├── vendor/                         # (unchanged)
├── .gitignore                      # NEW
├── .gitlab-ci.yml                  # MOVE → ci/legacy-gitlab-ci.yml
├── .pre-commit-config.yaml
├── CHANGELOG.md                    # NEW
├── Cargo.toml / Cargo.lock
├── LICENSE
├── README.md                       # REWRITE
├── build.gradle / settings.gradle
├── flake.nix
└── gradlew / gradlew.bat
```

Key changes:
- **`TODO.codecs/`** → `docs/plans/codecs/` (proper location, no dot-prefix)
- **`draft/slint-ui/docs/`** (upstream Slint mirror) → **delete** (not project docs)
- **`draft/slint-ui/phases/PHASE-{28..48}*.md`** (far-future roadmap) → `docs/archive/roadmap/`
- **`docs/refactor-implementation-guide/`** (completed) → `docs/archive/`
- **Loose `docs/draft-plan-*.md`** → `docs/archive/draft-plans/`
- **`.gitlab-ci.yml`** → `ci/legacy-gitlab-ci.yml`
- Add `.gitignore`, `CONTRIBUTING.md`, `CHANGELOG.md`

---

### Updated README.md (Complete Draft)

```markdown
# fcast-android-sender

[![Android Debug APK](https://github.com/kodyka/fcast-android-sender/actions/workflows/android-debug-apk.yml/badge.svg)](https://github.com/kodyka/fcast-android-sender/actions/workflows/android-debug-apk.yml)
[![Android Release APK](https://github.com/kodyka/fcast-android-sender/actions/workflows/android-release-apk.yml/badge.svg)](https://github.com/kodyka/fcast-android-sender/actions/workflows/android-release-apk.yml)
[![UI Lint](https://github.com/kodyka/fcast-android-sender/actions/workflows/ui-lint.yml/badge.svg)](https://github.com/kodyka/fcast-android-sender/actions/workflows/ui-lint.yml)

Standalone Android sender app for the [FCast protocol](https://github.com/kodyka/fcast).
Screen-captures or camera-captures video on Android, encodes via GStreamer,
and streams to FCast receivers over WHEP/SRT.

Extracted from `kodyka/fcast` at commit `63980e6`.

## Architecture

```
┌─────────────────────────────────────────────┐
│              Android (Kotlin)               │
│  MainActivity ─ ScreenCapture ─ StreamPack  │
│                    │  JNI                   │
├────────────────────┼────────────────────────┤
│              Rust (cdylib)                  │
│  ┌──────────┐ ┌────────────┐ ┌───────────┐ │
│  │ Slint UI │ │ GStreamer  │ │ fcast-sdk │ │
│  │ (bridge) │ │ pipelines  │ │ (WHEP/SRT)│ │
│  └──────────┘ └────────────┘ └───────────┘ │
│       migration-runtime  │  gstpop-runtime  │
└─────────────────────────────────────────────┘
```

## Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | stable + Android targets | Native library |
| Android SDK | API 34–36 | Build tooling |
| Android NDK | r28c | C/C++ cross-compilation |
| GStreamer Android SDK | 1.28.0 | Media pipeline |
| Java | 21 (Temurin) | Gradle builds |
| Nix (optional) | latest | Reproducible dev shell |

## Quick Start

### Option A: Nix dev shell (recommended)

```console
# Full Android shell (SDK + NDK + cargo-ndk + adb)
$ nix develop .#android -L

# Build, install on connected device, and launch
$ ./scripts/build-deploy.sh

# Release build
$ ./scripts/build-deploy.sh --release
```

### Option B: Manual setup

Export the following environment variables:

```console
export ANDROID_HOME=/path/to/Android/Sdk
export ANDROID_SDK_ROOT=$ANDROID_HOME
export ANDROID_NDK_ROOT=/path/to/android-ndk-r28c
export ANDROID_NDK_HOME=$ANDROID_NDK_ROOT
export GSTREAMER_ROOT_ANDROID=/path/to/gstreamer-1.0-android-universal-1.28.0
```

Then build:

```console
# Rust library
$ cargo check --target aarch64-linux-android
$ cargo build --release --target aarch64-linux-android

# Full APK
$ ./gradlew assembleDebug
$ ./gradlew installDebug
```

## Repository Layout

| Path | Description |
|------|-------------|
| `src/` | Main Rust crate (`android-sender`) — JNI bridge, app logic, GStreamer nodes |
| `crates/` | Internal Rust sub-crates: `migration-runtime`, `gstpop-runtime` |
| `ui/` | Slint UI tree: pages, components, bridge, theme, i18n |
| `app/` | Android Gradle module: Kotlin activity, JNI glue, manifest, resources |
| `ci/` | CI build scripts (`build-gstreamer-android-glue.sh`, `build-rust-android-lib.sh`) |
| `scripts/` | Developer helper scripts (build-deploy, smoke tests, slint-viewer) |
| `tests/` | Headless UI snapshot tests |
| `vendor/` | Vendored `gstpop` crate |
| `docs/` | Guides, plans, and architecture docs |
| `draft/` | Active research and UI migration planning |

## SDK Dependencies

This crate depends on three SDK crates from the FCast monorepo:

- `fcast-protocol`
- `fcast-sender-sdk`
- `mcore`

Pinned as Git dependencies. To bump the SDK pin, see [docs/cross-repo-sync.md](docs/cross-repo-sync.md).

## UI Development

Preview Slint UI without an Android device:

```console
# Via nix-shell
$ nix-shell -p slint-viewer --run "slint-viewer ui/main.slint --auto-reload"

# Or install globally (must match pinned version 1.16.0)
$ cargo install slint-viewer --version "=1.16.0" --force
$ slint-viewer ui/main.slint --auto-reload
```

Run UI validation and snapshot tests:

```console
$ ci/ui-validate.sh
$ cargo test --test ui_snapshots
```

## CI/CD

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `android-debug-apk` | push, PR | UI audit + arm64 debug APK + JVM unit tests |
| `android-release-apk` | release published, manual | Signed release APK, attached to GitHub Release |
| `android-instrumented-tests` | PR (app/src changes), nightly | Emulator-based instrumented tests |
| `symbol-stability` | PR (src/crates/app changes) | JNI symbol baseline diff |
| `ui-lint` | PR (ui/ changes) | Forbid raw hex colors, hardcoded sizes, direct state writes |
| `slint-viewer-smoke` | PR (ui/ changes) | Compile `ui/main.slint` with `slint-viewer` |
| `gstpop-smoke` | push, PR (src/ui/ci changes) | gst-pop backend integration tests |

## Debug Logs

```console
# App-focused
$ adb logcat -s fcastsender RustStdoutStderr

# Broader filter
$ adb logcat | grep -i fcast
```

## Contributing

See [CONTRIBUTING.md](.github/CONTRIBUTING.md) for guidelines on code style,
commit conventions, and the PR process.

## License

MIT
```

---

### Proposed `.gitignore`

```gitignore
# Rust
/target/
**/*.rs.bk

# Android
/local.properties
/.android/
/thirdparty/
*.apk
*.aab
*.ap_
*.dex
app/build/
build/
.gradle/
captures/

# IDE
.idea/
*.iml
.vscode/
*.swp
*~

# OS
.DS_Store
Thumbs.db

# Nix
result
result-*
```

---

### Proposed `CONTRIBUTING.md`

```markdown
# Contributing to fcast-android-sender

## Development Setup

1. Install Nix (recommended) or set up Android SDK/NDK manually
2. Enter the dev shell: `nix develop .#android -L`
3. Build and deploy: `./scripts/build-deploy.sh`

## Pre-commit Hooks

Install pre-commit hooks to catch lint issues before pushing:

```console
$ pip install pre-commit
$ pre-commit install
```

The hooks enforce:
- No raw hex colors in Slint files (use `Theme.*` tokens)
- No hardcoded `font-size: Npx` (use `Theme.font-size-*`)
- No direct `Bridge.active-panel` writes (use `PanelBridge.push`/`.pop`)
- No direct `Bridge.lifecycle` writes (use lifecycle callbacks)

## Branch and PR Conventions

- Branch from `main`
- Use descriptive branch names: `feat/srt-destination`, `fix/jni-crash`, `docs/readme-update`
- PRs require passing CI before merge
- If your PR changes JNI symbols, update `ci/jni-symbol-baseline.txt`

## Code Style

- **Rust**: `cargo fmt` + `cargo clippy`
- **Kotlin**: follow existing patterns in `app/src/`
- **Slint UI**: follow [docs/ui-slint-best-practices/](docs/ui-slint-best-practices/)

## Testing

```console
# Rust unit tests
$ cargo test

# JVM unit tests
$ ./gradlew :app:testDebugUnitTest

# UI snapshot tests
$ cargo test --test ui_snapshots

# UI validation
$ ci/ui-validate.sh

# Full instrumented tests (requires emulator)
$ ./gradlew :app:connectedDebugAndroidTest
```
``` [1-cite-4](#1-cite-4) [1-cite-5](#1-cite-5) [1-cite-6](#1-cite-6) [1-cite-7](#1-cite-7) [1-cite-8](#1-cite-8) [1-cite-9](#1-cite-9) [1-cite-10](#1-cite-10) [1-cite-11](#1-cite-11) 

---

### Step-by-Step Migration Instructions

**Phase 1: Housekeeping files (no code changes)**

1. Create `.gitignore` with the content above
2. Create `.github/CONTRIBUTING.md` with the content above
3. Create `CHANGELOG.md` (start with `## [Unreleased]` section)
4. Move `.gitlab-ci.yml` → `ci/legacy-gitlab-ci.yml` (update comment at top to note new path)

**Phase 2: Reorganize `docs/`**

5. Create `docs/archive/` directory
6. Move completed work:
   ```
   mv docs/refactor-implementation-guide/ docs/archive/
   mv docs/plan-android-service-gst-perf-test/ docs/archive/
   mv docs/plan-codecs-test-page/ docs/archive/
   mv docs/plan-gst-performance-test/ docs/archive/
   ```
7. Move loose draft-plan files:
   ```
   mkdir docs/archive/draft-plans/
   mv docs/draft-plan-*.md docs/archive/draft-plans/
   mv docs/code-examples-*.md docs/archive/draft-plans/
   mv docs/examples-code-*.md docs/archive/draft-plans/
   ```
8. Create `docs/guides/` and move active guides:
   ```
   mv docs/slintcn-migration/ docs/guides/
   mv docs/streampack-migration/ docs/guides/
   mv docs/streampack-migration-plan.md docs/guides/
   mv docs/ui-slint-best-practices/ docs/guides/
   mv docs/plan-orientation-mode.md docs/guides/
   ```
9. Create `docs/plans/` and move active version plans:
   ```
   mv docs/v0.1.0/ docs/plans/
   mv docs/v0.1.0-implementation-plan.md docs/plans/
   mv docs/v0.2.0/ docs/plans/
   ```
10. Move `TODO.codecs/` into docs:
    ```
    mv TODO.codecs/ docs/plans/codecs/
    ```
    Update the internal `../TODO.codecs.md` reference in `docs/plans/codecs/README.md`.

**Phase 3: Slim down `draft/`**

11. Delete the upstream Slint docs mirror (not project-owned content):
    ```
    rm -rf draft/slint-ui/docs/astro/
    rm -rf draft/slint-ui/docs/safety/
    rm -rf draft/slint-ui/docs/nodejs/
    rm -rf draft/slint-ui/docs/internal/
    rm -rf draft/slint-ui/docs/development/
    rm -rf draft/slint-ui/docs/skills/
    ```
    Keep only `draft/slint-ui/docs/current-fcast-slint-notes.md` and `draft/slint-ui/docs/slint-docs-used.md` as references.

12. Archive completed/far-future phases:
    ```
    mkdir docs/archive/roadmap/
    mv draft/slint-ui/phases/PHASE-{28..48}*.md docs/archive/roadmap/
    ```

13. Update `draft/README.md` to reflect slimmed-down contents.

**Phase 4: Update README**

14. Replace `README.md` with the updated version above.
15. Verify all internal links still resolve after moves (especially `docs/cross-repo-sync.md`).

**Phase 5: Validate**

16. Run `ci/ui-validate.sh` to confirm no lint regressions.
17. Run `cargo test` and `./gradlew :app:testDebugUnitTest`.
18. Verify GitHub Actions still pass (the workflows reference `ci/` scripts by relative path — unchanged).
19. Grep for broken internal markdown links:
    ```console
    grep -rn '\.\./TODO\.codecs' docs/
    grep -rn 'TODO\.codecs/' .
    ```

Repository: kodyka/fcast-android-sender (branch: main)

## Goal
Reorganize the folder structure, consolidate documentation, remove stale content, and add missing housekeeping files (.gitignore, CONTRIBUTING.md, CHANGELOG.md). Update the README with badges, architecture diagram, and comprehensive setup/CI documentation.

## Step-by-step

### 1. Add `.gitignore` at repo root
Create `.gitignore` covering Rust (target/), Android (local.properties, .android/, thirdparty/, app/build/, build/, .gradle/, *.apk), IDE (.idea/, .vscode/), OS (.DS_Store), and Nix (result, result-*).

### 2. Add `.github/CONTRIBUTING.md`
Create a contributing guide covering: dev setup (Nix shell), pre-commit hooks, branch/PR conventions, code style (Rust fmt+clippy, Kotlin, Slint best practices), and testing commands (cargo test, gradlew unit tests, UI snapshots, instrumented tests). Reference the existing `.pre-commit-config.yaml` hooks.

### 3. Add `CHANGELOG.md` at repo root
Start with a `## [Unreleased]` section. Note the extraction from kodyka/fcast monorepo as the initial entry.

### 4. Move `.gitlab-ci.yml` → `ci/legacy-gitlab-ci.yml`
The file is already deprecated (see its header comment). Moving it out of root reduces clutter. Keep the same content but update the header comment to note the new location.

### 5. Reorganize `docs/` — create `docs/archive/`
Move completed/historical documentation:
- `docs/refactor-implementation-guide/` → `docs/archive/refactor-implementation-guide/`
- `docs/plan-android-service-gst-perf-test/` → `docs/archive/plan-android-service-gst-perf-test/`
- `docs/plan-codecs-test-page/` → `docs/archive/plan-codecs-test-page/`
- `docs/plan-gst-performance-test/` → `docs/archive/plan-gst-performance-test/`
- `docs/draft-plan-*.md` and `docs/code-examples-*.md` and `docs/examples-code-*.md` → `docs/archive/draft-plans/`

### 6. Reorganize `docs/` — create `docs/guides/`
Move active guides:
- `docs/slintcn-migration/` → `docs/guides/slintcn-migration/`
- `docs/streampack-migration/` → `docs/guides/streampack-migration/`
- `docs/streampack-migration-plan.md` → `docs/guides/streampack-migration-plan.md`
- `docs/ui-slint-best-practices/` → `docs/guides/ui-slint-best-practices/`
- `docs/plan-orientation-mode.md` → `docs/guides/plan-orientation-mode.md`

### 7. Reorganize `docs/` — create `docs/plans/`
Move active version plans:
- `docs/v0.1.0/` → `docs/plans/v0.1.0/`
- `docs/v0.1.0-implementation-plan.md` → `docs/plans/v0.1.0-implementation-plan.md`
- `docs/v0.2.0/` → `docs/plans/v0.2.0/`

### 8. Move `TODO.codecs/` → `docs/plans/codecs/`
Delete the `TODO.codecs/` directory from root and place it under `docs/plans/codecs/`. Update the internal reference to `../TODO.codecs.md` in the README.md inside that directory — it may need to be adjusted since the relative path changes.

### 9. Slim down `draft/slint-ui/docs/`
Delete the upstream Slint documentation mirror that was copied for research but is not project-owned content:
- `draft/slint-ui/docs/astro/` (entire directory)
- `draft/slint-ui/docs/safety/` (entire directory)
- `draft/slint-ui/docs/nodejs/` (entire directory)
- `draft/slint-ui/docs/internal/` (entire directory)
- `draft/slint-ui/docs/development/` (entire directory)
- `draft/slint-ui/docs/skills/` (entire directory)
- Other upstream-only files: `draft/slint-ui/docs/building.md`, `draft/slint-ui/docs/embedded-tutorials.md`, `draft/slint-ui/docs/install_qt.md`, `draft/slint-ui/docs/nightly-release-notes.md`, `draft/slint-ui/docs/release-artifacts.md`, `draft/slint-ui/docs/release-notes.md`, `draft/slint-ui/docs/testing.md`, `draft/slint-ui/docs/torizon.md`, `draft/slint-ui/docs/swiftui-to-slint-guide.md`, `draft/slint-ui/docs/readme.md`
Keep ONLY: `draft/slint-ui/docs/current-fcast-slint-notes.md`, `draft/slint-ui/docs/slint-docs-used.md`, and `draft/slint-ui/docs/_MIRROR.md`

### 10. Archive far-future draft phases
Move completed/far-future roadmap phases from `draft/slint-ui/phases/` to `docs/archive/roadmap/`:
- All `PHASE-{28..48}*.md` files (PHASE-28 through PHASE-48 — these are far-future features like chat overlay, replay buffer, iOS targets, in-app purchase, etc.)
Keep active MVP phases and near-term phases in place.

### 11. Update `draft/README.md`
Rewrite to reflect the slimmed-down contents after removals. Note that upstream Slint docs were removed and explain where archived phases went.

### 12. Rewrite `README.md`
Replace root README.md with the updated version from the analysis. Key additions:
- CI status badges for android-debug-apk, android-release-apk, and ui-lint workflows
- Architecture ASCII diagram showing Android/Kotlin → JNI → Rust (Slint UI + GStreamer + fcast-sdk) layers
- Prerequisites table (Rust, Android SDK, NDK r28c, GStreamer 1.28.0, Java 21, Nix)
- Repository layout table with updated paths
- CI/CD table documenting all 7 GitHub Actions workflows
- Link to CONTRIBUTING.md
- Keep all existing build instructions

### 13. Fix broken internal links
After all moves, grep for broken references:
- `grep -rn 'TODO.codecs' .` — update any references to point to `docs/plans/codecs/`
- `grep -rn 'refactor-implementation-guide' .` — update to `docs/archive/refactor-implementation-guide/`
- Check all relative links in moved markdown files

### 14. Validate
- Run `ci/ui-validate.sh` to confirm no regressions
- Run `cargo test` to confirm Rust builds
- Run `./gradlew :app:testDebugUnitTest` for JVM tests
- Verify no GitHub Actions workflow files reference moved paths (they reference `ci/`, `src/`, `ui/`, `app/` which are unchanged)
