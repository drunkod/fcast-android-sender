# Repository Refresh Plan — `fcast-android-sender`

> **Status:** Completed · **Owner:** maintainers · **Last verified against `main`:** 2026-07-13
>
> This is the consolidated, fact-checked plan that supersedes the two earlier
> drafts (`repository-refresh-plan.md` and `draft-plan-refresh-repo.md`). It folds
> in their good ideas, **corrects several inaccurate audit findings** (see
> [Audit corrections](#1-audit-corrections)), and ships with executable tooling:
> [`scripts/check-repo-layout.sh`](../scripts/check-repo-layout.sh) and
> [`.github/workflows/repo-health.yml`](../.github/workflows/repo-health.yml).
> The refresh landed in PR #27. The repo-health workflow remains available for
> manual dispatch, but automatic push/pull-request triggers are currently disabled.

## Detailed split docs

This plan is the overview and rationale. The step-level detail (with full code
snippets) is split into focused files:

| Topic | File |
|---|---|
| Docs index | [README.md](README.md) |
| Architecture | [architecture/overview.md](architecture/overview.md) |
| Android build | [build/android-build.md](build/android-build.md) |
| Contributor workflow | [development/contributor-workflow.md](development/contributor-workflow.md) |
| Repo-health checks | [development/repo-health-checks.md](development/repo-health-checks.md) |
| Migration runbook | [migrations/repo-refresh/step-by-step.md](migrations/repo-refresh/step-by-step.md) |
| Example: health script | [examples/check-repo-layout.sh.md](examples/check-repo-layout.sh.md) |
| Example: CI workflow | [examples/repo-health-workflow.yml.md](examples/repo-health-workflow.yml.md) |

## Contents

1. [Audit corrections (what the earlier drafts got wrong)](#1-audit-corrections)
2. [Goals & non-goals](#2-goals--non-goals)
3. [Current state (measured)](#3-current-state-measured)
4. [Documentation policy](#4-documentation-policy)
5. [Target repository layout](#5-target-repository-layout)
6. [Migration: phased, copy-paste runbook](#6-migration-phased-copy-paste-runbook)
7. [Android build documentation](#7-android-build-documentation)
8. [Contributor workflow](#8-contributor-workflow)
9. [Repo-health: script + CI](#9-repo-health-script--ci)
10. [Community file templates](#10-community-file-templates)
11. [Validation checklist](#11-validation-checklist)
12. [Rollback](#12-rollback)

---

## 1. Audit corrections

The earlier `draft-plan-refresh-repo.md` audit contained findings that do **not**
match the repository as it stands on `main`. The refresh must not act on these
without correction, or it will "fix" things that aren't broken and miss the real
problems.

| # | Earlier audit claim | Reality (verified) | Action |
|---|---------------------|--------------------|--------|
| 1 | "No `.gitignore` file (High)" | **`.gitignore` exists** (28 lines). Present but thin. | Downgrade to Low. **Extend**, don't create. |
| 2 | "`app/build.gradle` references `libs.*` but no `gradle/libs.versions.toml` found (Medium)" | **`gradle/libs.versions.toml` exists.** | **Drop this finding.** No action. |
| 3 | "`docs/` has ~120+ files" | **322 Markdown files** under `docs/`. | Scale of cleanup is larger; plan accordingly. |
| 4 | "`draft/` ~200+ files" | **6,835 files** under `draft/` (incl. a 313-file upstream Slint mirror). | Single biggest cleanup item. |
| 5 | (not flagged) | **Real bug:** `README.md` says **NDK r25c**; `flake.nix` pins **r28c** (`28.0.13004108`). | **Fix README** — see [§7](#7-android-build-documentation). Guarded by the health script. |
| 6 | "No `CONTRIBUTING.md` / `CHANGELOG.md` / `SECURITY.md`" | Confirmed **missing**. | Add all three — see [§10](#10-community-file-templates). |
| 7 | "`.gitlab-ci.yml` deprecated, at root" | Confirmed; header already says "DEPRECATED: kept for rollback". | Move to `ci/legacy-gitlab-ci.yml`. |
| 8 | "7 workflows" | Confirmed **7**: `android-debug-apk`, `android-instrumented-tests`, `android-release-apk`, `gstpop-smoke`, `slint-viewer-smoke`, `symbol-stability`, `ui-lint`. | Document in README CI table. |
| 9 | "`TODO.codecs/` 17 files" | **18 files**, dot-prefixed at root. | Move to `docs/plans/codecs/`. |

**Net:** two "High" findings (#1, #2) were false. The genuinely high-impact items
are the `draft/` bloat (#4), the `docs/` sprawl (#3), the missing community files
(#6), and the **NDK version drift** (#5) which no earlier draft caught.

---

## 2. Goals & non-goals

**Goals**

- Make the repo legible to a new contributor in under 15 minutes.
- Separate *active* docs from *archived* drafts and *vendored upstream* material.
- Keep the Android / Rust / Slint / GStreamer / CI / SDK-pin responsibilities discoverable.
- Add **repeatable, enforced** checks so the layout cannot silently drift again.
- Make every toolchain version assertion single-sourced (no README-vs-flake drift).

**Non-goals**

- No source-code refactoring. `src/`, `crates/`, `ui/`, `app/`, `ci/`, `vendor/`,
  `tests/`, `scripts/` keep their current responsibilities and paths.
- No CI behavior change to the seven Android workflows (they reference unchanged paths).
- No history rewrite. All moves use `git mv` so blame/history survive.

---

## 3. Current state (measured)

```text
fcast-android-sender/            # workspace root (Cargo + Gradle + Nix)
├── .github/                     # 7 workflows + 1 composite action  (keep)
├── .claude/                     # assistant commands                (keep)
├── app/                         # Android Gradle module: Kotlin/Java, JNI glue
├── ci/                          # build scripts + JNI symbol baseline
├── crates/                      # migration-runtime, gstpop-runtime
├── src/                         # main Rust crate `android-sender` (~18 modules)
├── ui/                          # Slint UI tree (~35 pages)
├── vendor/                      # vendored gstpop crate
├── tests/                       # headless UI snapshot tests
├── scripts/                     # dev helpers (build-deploy, smoke, slint-viewer)
├── docs/                        # 322 .md files across 10+ subdirs   ← sprawl
├── draft/                       # 6,835 files (incl. 313-file Slint mirror) ← bloat
├── TODO.codecs/                 # 18 dot-prefixed task files at root  ← misplaced
├── .gitlab-ci.yml               # DEPRECATED (kept for rollback)      ← move
├── .gitignore                   # present (28 lines)                  ← extend
├── flake.nix                    # NDK pinned r28c (28.0.13004108)
├── README.md                    # says NDK r25c                       ← stale, fix
├── Cargo.toml / Cargo.lock / build.rs
├── build.gradle / settings.gradle / gradle/ (libs.versions.toml present)
└── gradlew / gradlew.bat
```

Reproduce the measurements:

```bash
find docs  -name '*.md' | wc -l          # 322
find draft -type f       | wc -l          # 6835
find draft/slint-ui/docs -type f | wc -l  # 313 (upstream Slint mirror)
find TODO.codecs -type f | wc -l          # 18
```

---

## 4. Documentation policy

A single rule decides where any document lives and whether it is committed.

| Document type | Location | Commit? |
|---|---|---|
| Onboarding / index | `docs/README.md` | Yes |
| Architecture | `docs/architecture/` | Yes |
| Build & setup | `docs/build/` | Yes |
| Developer workflow | `docs/development/` | Yes |
| Active how-to guides | `docs/guides/` | Yes |
| Active implementation plans | `docs/plans/<topic>/` | Yes |
| Completed / historical | `docs/archive/<topic>/` | Yes (read-only) |
| Vendored upstream copies | **not in this repo** | **No — delete** |
| Local scratch notes | `draft/local/` (git-ignored) | No |

Guiding principle: *if it describes how the project works today, it's under a
committed `docs/` subdir. If it's finished, it's in `docs/archive/`. If it's a
copy of someone else's docs, it does not belong here.*

---

## 5. Target repository layout

```text
fcast-android-sender/
├── README.md                    # REWRITE (badges, arch, NDK r28c, CI table)
├── CONTRIBUTING.md              # NEW
├── SECURITY.md                  # NEW
├── CHANGELOG.md                 # NEW (Keep a Changelog format)
├── LICENSE
├── .gitignore                   # EXTEND
├── Cargo.toml / Cargo.lock / build.rs
├── build.gradle / settings.gradle / gradle/libs.versions.toml
├── flake.nix / flake.lock
├── gradlew / gradlew.bat
├── app/  src/  crates/  ui/  vendor/  tests/  scripts/   # unchanged
├── ci/
│   └── legacy-gitlab-ci.yml      # MOVED from /.gitlab-ci.yml
├── .github/
│   ├── actions/                  # unchanged
│   └── workflows/                # 7 existing + repo-health.yml (NEW)
└── docs/
    ├── README.md                 # NEW — docs index
    ├── repository-refresh-plan.md# THIS FILE
    ├── architecture/
    │   └── overview.md           # NEW (links the Mermaid ARCHITECTURE.md)
    ├── build/
    │   └── android-build.md      # NEW — single source of truth for the build
    ├── development/
    │   ├── contributor-workflow.md
    │   └── repo-health-checks.md
    ├── guides/                   # active how-tos (moved out of docs/ root)
    │   ├── slintcn-migration/
    │   ├── streampack-migration/
    │   └── ui-slint-best-practices/
    ├── plans/                    # active plans
    │   ├── v0.1.0/  v0.2.0/
    │   └── codecs/               # MOVED from /TODO.codecs/
    ├── migrations/
    │   └── repo-refresh/
    │       └── step-by-step.md   # the runbook in §6, extractable
    └── archive/                  # completed / historical (read-only)
        ├── refactor-implementation-guide/
        ├── roadmap/              # far-future draft phases
        └── draft-plans/          # loose docs/draft-plan-*.md
```

The folder skeleton sketched in the request maps onto this as: `architecture/overview.md`,
`build/android-build.md`, `development/{contributor-workflow,repo-health-checks}.md`,
and `examples/*` (the script/CI examples — here delivered as the *real* files in
[§9](#9-repo-health-script--ci) rather than `.md`-wrapped copies).

---

## 6. Migration: phased, copy-paste runbook

Each phase is independently committable and reversible. Run from repo root,
inside `nix develop .#android` for the build-validation phases. **Use `git mv`**
so history follows the files.

### Phase 0 — branch & safety net

```bash
git switch -c chore/repo-refresh
git status --porcelain        # must be clean before starting
```

### Phase 1 — housekeeping files (no moves)

```bash
# Extend (do NOT recreate) .gitignore — append the block from §10.
# Then add the community files from §10:
$EDITOR .gitignore CONTRIBUTING.md SECURITY.md CHANGELOG.md
git add .gitignore CONTRIBUTING.md SECURITY.md CHANGELOG.md
git commit -m "chore: add CONTRIBUTING/SECURITY/CHANGELOG, extend .gitignore"
```

### Phase 2 — retire the deprecated GitLab CI

```bash
git mv .gitlab-ci.yml ci/legacy-gitlab-ci.yml
sed -i '1i # NOTE: moved to ci/legacy-gitlab-ci.yml during the 2026 repo refresh.' \
  ci/legacy-gitlab-ci.yml
git commit -am "chore: move deprecated .gitlab-ci.yml to ci/legacy-gitlab-ci.yml"
```

### Phase 3 — reorganize `docs/`

```bash
mkdir -p docs/archive/draft-plans docs/guides docs/plans

# 3a. Archive completed work
for d in refactor-implementation-guide plan-android-service-gst-perf-test \
         plan-codecs-test-page plan-gst-performance-test; do
  [ -d "docs/$d" ] && git mv "docs/$d" "docs/archive/$d"
done

# 3b. Archive loose draft-plan / example files at docs/ root
git mv docs/draft-plan-*.md    docs/archive/draft-plans/ 2>/dev/null || true
git mv docs/code-examples-*.md docs/archive/draft-plans/ 2>/dev/null || true
git mv docs/examples-code-*.md docs/archive/draft-plans/ 2>/dev/null || true

# 3c. Active guides
for d in slintcn-migration streampack-migration ui-slint-best-practices; do
  [ -d "docs/$d" ] && git mv "docs/$d" "docs/guides/$d"
done
git mv docs/streampack-migration-plan.md docs/guides/ 2>/dev/null || true
git mv docs/plan-orientation-mode.md     docs/guides/ 2>/dev/null || true

# 3d. Active version plans
for d in v0.1.0 v0.2.0; do
  [ -d "docs/$d" ] && git mv "docs/$d" "docs/plans/$d"
done
git mv docs/v0.1.0-implementation-plan.md docs/plans/ 2>/dev/null || true

git commit -m "docs: split docs/ into guides, plans, and archive"
```

### Phase 4 — relocate `TODO.codecs/`

```bash
git mv TODO.codecs docs/plans/codecs
# Fix the internal back-reference that assumed a root-level sibling:
grep -rln '\.\./TODO\.codecs' docs/plans/codecs/ \
  | xargs -r sed -i 's#\.\./TODO\.codecs#.#g'
git commit -m "docs: move TODO.codecs/ to docs/plans/codecs/"
```

### Phase 5 — slim `draft/` (the big one)

Delete the **vendored upstream Slint documentation** — it is a copy of another
project's docs and is not maintained here.

```bash
# 5a. Remove the upstream Slint mirror (313 files). Verify count first:
find draft/slint-ui/docs -type f | wc -l
git rm -r draft/slint-ui/docs/astro draft/slint-ui/docs/safety \
         draft/slint-ui/docs/nodejs draft/slint-ui/docs/internal \
         draft/slint-ui/docs/development draft/slint-ui/docs/skills \
         draft/slint-ui/docs/common draft/slint-ui/docs/site 2>/dev/null || true
# Keep only project-authored notes:
#   draft/slint-ui/docs/current-fcast-slint-notes.md
#   draft/slint-ui/docs/slint-docs-used.md

# 5b. Archive far-future roadmap phases
mkdir -p docs/archive/roadmap
git mv draft/slint-ui/phases/PHASE-{28..48}*.md docs/archive/roadmap/ 2>/dev/null || true

git commit -m "chore: remove vendored Slint docs mirror, archive far-future phases"
```

> **Tip:** if a draft tree is genuinely just local scratch, move it to
> `draft/local/` and add `draft/local/` to `.gitignore` instead of deleting.

### Phase 6 — rewrite README + add docs index

```bash
$EDITOR README.md docs/README.md
# Fix the NDK rev (r25c -> r28c), add badges, arch link, CI table (§7).
git add README.md docs/README.md
git commit -m "docs: rewrite README (badges, arch, NDK r28c) and add docs index"
```

### Phase 7 — wire up repo-health automation

```bash
chmod +x scripts/check-repo-layout.sh
git add scripts/check-repo-layout.sh .github/workflows/repo-health.yml
git commit -m "ci: add repo-layout health check and workflow"
```

### Phase 8 — fix dangling links & validate

```bash
# Find references to moved paths (excluding this plan + the checker, which
# intentionally mention the old names):
grep -rn --exclude-dir=.git \
  -e 'TODO\.codecs/' \
  -e 'docs/refactor-implementation-guide/' . \
  | grep -v -e 'repository-refresh-plan.md' -e 'check-repo-layout.sh'

bash scripts/check-repo-layout.sh        # must exit 0
```

> **Note on source comments:** `src/backend/registry.rs` and `src/app.rs` contain
> doc comments pointing at `docs/refactor-implementation-guide/...`. After Phase 3a
> those become `docs/archive/refactor-implementation-guide/...`; update the
> comments in Phase 8 so the health check stays green.

---

## 7. Android build documentation

> Single source of truth for the build. Lives at `docs/build/android-build.md`;
> the README links here instead of duplicating it. **All version numbers below
> are taken from `flake.nix`, which is authoritative.**

### Toolchain (authoritative versions)

| Tool | Version | Source of truth |
|------|---------|-----------------|
| Rust | stable + Android targets | `rust-toolchain` / flake |
| Android SDK | API 34 (target), min 26 | `Cargo.toml` `[package.metadata.android.sdk]` |
| Android NDK | **r28c** = `28.0.13004108` | `flake.nix` `ndkVersion` |
| GStreamer Android SDK | 1.28.0 | `README` / `build.rs` env contract |
| Java | 21 (Temurin) | flake / Gradle |
| Nix (optional) | latest | `flake.nix` |

> **Drift guard:** the README previously said *NDK r25c*. The number now lives in
> exactly one place (`flake.nix`); `check-repo-layout.sh` fails CI if the README
> disagrees. When bumping the NDK, change `flake.nix` and the README's single
> reference together.

### Option A — Nix dev shell (recommended)

```console
# Full Android shell: SDK + NDK + cargo-ndk + adb
$ nix develop .#android -L

# Build, install on a connected device, and launch
$ ./scripts/build-deploy.sh
$ ./scripts/build-deploy.sh --release      # release build
$ ./scripts/build-deploy.sh --no-install   # build only
```

### Option B — manual environment

`build.rs` is a no-op on non-Android targets. For Android targets
(`aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android`,
`i686-linux-android`) export:

```console
export ANDROID_HOME=/path/to/Android/Sdk
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_ROOT=/path/to/android-ndk-r28c     # 28.0.13004108
export ANDROID_NDK_HOME="$ANDROID_NDK_ROOT"
export GSTREAMER_ROOT_ANDROID=/path/to/gstreamer-1.0-android-universal-1.28.0
```

Then:

```console
# Rust native library (cdylib: libfcastsender.so)
$ cargo check --target aarch64-linux-android
$ cargo build --release --target aarch64-linux-android

# Full APK via Gradle
$ ./gradlew assembleDebug
$ ./gradlew installDebug
```

CI validates only the `aarch64-linux-android` path for speed; the crate metadata
lists the other three targets for local/release builds.

### Debug logs

```console
$ adb logcat -s fcastsender RustStdoutStderr   # app-focused
$ adb logcat | grep -i fcast                   # broader
```

### UI preview without a device

```console
$ bash scripts/check-slint-viewer.sh           # verify viewer == pinned 1.16.0
$ nix-shell -p slint-viewer --run "slint-viewer ui/main.slint --auto-reload"
$ slint-viewer ui/pages/media_backend_page.slint --component MediaBackendPage
```

---

## 8. Contributor workflow

> Lives at `docs/development/contributor-workflow.md`; summarized in
> `CONTRIBUTING.md` (template in [§10](#10-community-file-templates)).

### Setup

```console
$ nix develop .#android -L          # or set up SDK/NDK manually (§7B)
$ pip install pre-commit && pre-commit install
```

### Pre-commit hooks (from `.pre-commit-config.yaml`)

The UI lint hooks enforce:

- No raw hex colors in Slint — use `Theme.*` tokens.
- No hardcoded `font-size: Npx` — use `Theme.font-size-*`.
- No direct `Bridge.active-panel` writes — use `PanelBridge.push` / `.pop`.
- No direct `Bridge.lifecycle` writes — use lifecycle callbacks.

### Branch & PR conventions

- Branch from `main`: `feat/srt-destination`, `fix/jni-crash`, `docs/readme-update`, `chore/...`.
- PRs require green CI before merge.
- If a change alters exported JNI symbols, update `ci/jni-symbol-baseline.txt`
  (the `symbol-stability` workflow diffs against it).

### Test matrix

```console
$ cargo test                               # Rust unit tests
$ cargo test --test ui_snapshots           # headless Slint snapshots
$ ci/ui-validate.sh                        # UI lint suite
$ ./gradlew :app:testDebugUnitTest         # JVM unit tests
$ ./gradlew :app:connectedDebugAndroidTest # instrumented (needs emulator)
$ bash scripts/check-repo-layout.sh        # repo-health (new)
```

### CI/CD reference (7 + 1 workflows)

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

---

## 9. Repo-health: script + CI

Two real, runnable files ship with this plan (not Markdown-wrapped):

- [`scripts/check-repo-layout.sh`](../scripts/check-repo-layout.sh) — local + CI guard.
- [`.github/workflows/repo-health.yml`](../.github/workflows/repo-health.yml) — runs it on every PR.

### What the script enforces

1. Required files exist: `README`, `LICENSE`, `.gitignore`, `CONTRIBUTING.md`,
   `SECURITY.md`, `CHANGELOG.md`, `Cargo.toml`, `settings.gradle`, `flake.nix`.
2. Stale paths are gone: `TODO.codecs/`, root `.gitlab-ci.yml`, the
   `draft/slint-ui/docs/{astro,safety,nodejs}` mirror.
3. No loose `draft-plan-*.md` at `docs/` root.
4. **README NDK rev matches `flake.nix`** (the bug from [§1](#1-audit-corrections)).
5. No dangling references to moved paths (`TODO.codecs/`, `refactor-implementation-guide/`).
6. `CONTRIBUTING.md` is resolvable.

### Usage

```console
$ ./scripts/check-repo-layout.sh           # CI mode: non-zero on drift
$ ./scripts/check-repo-layout.sh --warn    # report only, exit 0 (pre-refresh)
$ ./scripts/check-repo-layout.sh --quiet   # failures only
```

During the migration, run with `--warn` so you can watch failures turn green
phase by phase; flip CI to hard-fail once Phase 8 passes.

### The workflow at a glance

`repo-health.yml` runs three independent jobs on PRs touching docs/scripts/README:

- **layout** — runs `check-repo-layout.sh`.
- **shellcheck** — lints every `scripts/*.sh` and `ci/*.sh` at `--severity=warning`.
- **markdown-links** — fails on broken repo-relative links.

It needs no Android toolchain, so it returns in well under a minute.

---

## 10. Community file templates

### `.gitignore` (append to the existing file)

```gitignore
# ── added by repo refresh ───────────────────────────────────────────────
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

# Local scratch (never commit)
draft/local/
```

### `CONTRIBUTING.md`

```markdown
# Contributing to fcast-android-sender

## Development setup
1. Install Nix (recommended) or set up Android SDK/NDK manually (see docs/build/android-build.md).
2. Enter the dev shell: `nix develop .#android -L`
3. Build and deploy: `./scripts/build-deploy.sh`
4. Install hooks: `pip install pre-commit && pre-commit install`

## Pre-commit hooks
The hooks (`.pre-commit-config.yaml`) enforce:
- No raw hex colors in Slint files (use `Theme.*` tokens)
- No hardcoded `font-size: Npx` (use `Theme.font-size-*`)
- No direct `Bridge.active-panel` writes (use `PanelBridge.push`/`.pop`)
- No direct `Bridge.lifecycle` writes (use lifecycle callbacks)

## Branches & PRs
- Branch from `main`: `feat/...`, `fix/...`, `docs/...`, `chore/...`
- PRs require passing CI before merge
- JNI-symbol changes must update `ci/jni-symbol-baseline.txt`

## Code style
- Rust: `cargo fmt` + `cargo clippy`
- Kotlin: follow existing patterns in `app/src/`
- Slint: follow `docs/guides/ui-slint-best-practices/`

## Testing
    cargo test
    cargo test --test ui_snapshots
    ci/ui-validate.sh
    ./gradlew :app:testDebugUnitTest
    bash scripts/check-repo-layout.sh
```

### `SECURITY.md`

```markdown
# Security Policy

## Supported versions
The `main` branch receives security fixes. Tagged releases are fixed on a
best-effort basis.

## Reporting a vulnerability
Please report security issues privately via GitHub's "Report a vulnerability"
(Security Advisories) on this repository, rather than opening a public issue.
Include reproduction steps and the affected commit/tag. We aim to acknowledge
within 72 hours.

## Scope notes
This app bridges Android (JNI) to a Rust core and a GStreamer/WebRTC media
pipeline. Reports about memory safety at the JNI boundary, signalling/WHEP
handling, or secret storage (`AndroidSecretStore`) are especially welcome.
```

### `CHANGELOG.md`

```markdown
# Changelog

All notable changes are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning is SemVer.

## [Unreleased]
### Added
- `CONTRIBUTING.md`, `SECURITY.md`, `CHANGELOG.md`.
- `scripts/check-repo-layout.sh` and `repo-health` CI workflow.
- `docs/` reorganized into `architecture/`, `build/`, `development/`, `guides/`,
  `plans/`, `archive/`.
### Changed
- README corrected to NDK r28c (matches `flake.nix`); added CI badges + arch.
- `.gitlab-ci.yml` moved to `ci/legacy-gitlab-ci.yml`.
- `TODO.codecs/` moved to `docs/plans/codecs/`.
### Removed
- Vendored upstream Slint documentation mirror (~313 files) from `draft/`.

## [0.1.0]
- Initial extraction from `kodyka/fcast` at `63980e6`.
```

---

## 11. Validation checklist

Run after Phase 8. Every line must pass before opening the PR.

```console
# Layout & consistency (the new guard)
$ bash scripts/check-repo-layout.sh

# No dangling links to moved paths (should print nothing)
$ grep -rn --exclude-dir=.git -e 'TODO\.codecs/' \
    -e 'docs/refactor-implementation-guide/' . \
    | grep -v -e repository-refresh-plan.md -e check-repo-layout.sh

# Code still builds & tests pass
$ cargo test
$ ./gradlew :app:testDebugUnitTest
$ ci/ui-validate.sh

# Android build smoke (inside nix develop .#android)
$ cargo check --target aarch64-linux-android
```

- [ ] `check-repo-layout.sh` exits 0
- [ ] No dangling links
- [ ] `cargo test` green
- [ ] JVM unit tests green
- [ ] `ci/ui-validate.sh` green
- [ ] README NDK rev == `flake.nix`
- [ ] `CONTRIBUTING.md`, `SECURITY.md`, `CHANGELOG.md` present
- [ ] The 7 existing workflows still reference unchanged paths

---

## 12. Rollback

Every phase is its own commit, so rollback is granular:

```bash
git log --oneline chore/repo-refresh     # find the phase commit
git revert <sha>                         # undo one phase
# or abandon the whole branch:
git switch main && git branch -D chore/repo-refresh
```

Because all relocations use `git mv` (not delete+add), `git log --follow` and
`git blame` keep working across the move. The only destructive step is Phase 5's
`git rm` of the vendored Slint mirror — recoverable from history via
`git checkout <sha>^ -- <path>` if ever needed.
