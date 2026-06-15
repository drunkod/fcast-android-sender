# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning is SemVer.

## [Unreleased]

### Added

- `CONTRIBUTING.md`, `SECURITY.md`, and this `CHANGELOG.md`.
- `scripts/check-repo-layout.sh` and the `repo-health` CI workflow
  (`.github/workflows/repo-health.yml`).
- `ARCHITECTURE.md` — Mermaid diagram of the Android/JNI/Rust stack.
- `docs/` reorganized into `architecture/`, `build/`, `development/`, `guides/`,
  `plans/`, and `archive/`, plus a `docs/README.md` index and a consolidated
  `docs/repository-refresh-plan.md`.

### Changed

- README corrected to **NDK r28c** (matches `flake.nix`); added CI badges,
  an architecture link, and a CI/workflow table.
- `.gitlab-ci.yml` moved to `ci/legacy-gitlab-ci.yml`.
- `TODO.codecs/` moved to `docs/plans/codecs/` (broken `../TODO.codecs.md`
  provenance links removed).

### Removed

- Vendored upstream Slint documentation mirror (~313 files) from
  `draft/slint-ui/docs/`; far-future roadmap phases archived under
  `docs/archive/roadmap/`.

## [0.1.0]

- Initial extraction from `kodyka/fcast` at `63980e6`.
