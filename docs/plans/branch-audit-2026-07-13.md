# Branch audit: 2026-07-13

> Compared with `origin/main` at `c816a6e` after `git fetch --all --prune`.
> `ahead` and `behind` are commit counts relative to `origin/main`.

## Result

- `main` is synchronized with `origin/main`.
- All six remote topic branches are ancestors of `origin/main`.
- Twenty-three local topic branches are ancestors of `origin/main` and can be removed
  locally after approval.
- Four local branches diverge from `main`; none should be merged wholesale.
- No branches were deleted during this audit.

## Divergent local branches

| Branch | Ahead | Behind | Disposition |
|---|---:|---:|---|
| `android12-keyboard-research` | 16 | 170 | Do not merge. It patches the removed Java host (`MainActivity.java`). Reproduce the IME issue on the Kotlin host, then port only the required behavior in a new branch. |
| `feature/service-abstraction-refactor` | 3 | 196 | Drop after approval. It targets pre-extraction `src/migration/` and duplicates the service/backend architecture now on `main`. |
| `gst-pop-android-mvp` | 20 | 101 | Do not merge. The embedded runtime/JNI baseline is already present in newer form; media tools, typed client, desktop CLI, and device-test work need a new product decision and a fresh plan. |
| `gstpop-runtime-extraction` | 1 | 173 | Drop after approval. Its `.gitignore` patch would remove current Graphify, GStreamer, and local-environment ignores and replace them with less precise patterns. |

## Merged local branches

Every branch below has `ahead=0` and is fully contained in `origin/main`. The behind
count is informational; all are local deletion candidates.

| Branch | Behind | Tracking state |
|---|---:|---|
| `chore/repo-refresh` | 2 | remote exists |
| `codex/android-flake-deploy-shell` | 257 | gone |
| `codex/mvp-phase-12-gstpop-backend-toggle` | 250 | gone |
| `devin/1779282704-gstpop-dbus-module` | 239 | gone |
| `devin/1779867871-migration-runtime-service-guide` | 174 | gone |
| `devin/fix-android-gha-cross-pkgconfig` | 267 | gone |
| `devin/move-draft-to-android-repo` | 265 | gone |
| `devin/phase-10-extract-android-sender` | 270 | gone |
| `docs/0.0.2` | 19 | remote exists; local tip is also merged |
| `docs/android-service-gst-perf-plan` | 30 | remote exists; local tip is also merged |
| `docs/codec-test-page-plan` | 44 | gone |
| `docs/gst-performance-page-plan` | 35 | remote exists; local tip is also merged |
| `feat/gstpop-android-service` | 231 | gone |
| `feat/v0.1.0-srt-destination-wiring` | 29 | remote exists |
| `feature/ci-slint-viewer-smoke` | 205 | gone |
| `feature/settings-open-test` | 206 | gone |
| `feature/test-functionality-page` | 193 | gone |
| `feature/ui-refactoring-all-steps` | 212 | gone |
| `fix/camera-rtmp-stream` | 71 | gone |
| `old-main` | 167 | remote exists |
| `phase-10` | 102 | gone |
| `refactor` | 81 | gone |
| `slintcn` | 52 | gone |

## Remote branches

These remote topic refs are all ancestors of `origin/main` and are remote deletion
candidates: `origin/chore/repo-refresh`, `origin/docs/0.0.2`,
`origin/docs/android-service-gst-perf-plan`, `origin/docs/gst-performance-page-plan`,
`origin/feat/v0.1.0-srt-destination-wiring`, and `origin/old-main`.

Deleting remote branches changes shared repository state and requires an explicit
maintainer decision. Keep `origin/main` and `origin/HEAD`.

## Recommended cleanup command set

Run only after approving the dispositions above:

```bash
git branch -d <merged-local-branch>...
git branch -D feature/service-abstraction-refactor gstpop-runtime-extraction
```

Review `android12-keyboard-research` and `gst-pop-android-mvp` once more for selective
salvage before force-deleting their final refs. Delete remote branches separately through
the hosting service or an explicitly reviewed `git push origin --delete ...` command.
