# Repository refresh — step-by-step runbook

The phased, copy-paste migration. Each phase is an independent, reversible commit.
Run from the repo root; use `nix develop .#android` for the build-validation
phases. **Use `git mv`** so history follows the files. Rationale and the corrected
audit live in [../../repository-refresh-plan.md](../../repository-refresh-plan.md).

> Tip: while migrating, run `bash scripts/check-repo-layout.sh --warn` after each
> phase and watch failures turn green.

## Phase 0 — branch & safety net

```bash
git switch -c chore/repo-refresh
git status --porcelain        # must be clean before starting
```

## Phase 1 — housekeeping files (no moves)

Extend the **existing** `.gitignore` (do not recreate it) and add the three
community files. Templates are in
[../../repository-refresh-plan.md](../../repository-refresh-plan.md#10-community-file-templates).

```bash
$EDITOR .gitignore CONTRIBUTING.md SECURITY.md CHANGELOG.md
git add .gitignore CONTRIBUTING.md SECURITY.md CHANGELOG.md
git commit -m "chore: add CONTRIBUTING/SECURITY/CHANGELOG, extend .gitignore"
```

## Phase 2 — retire the deprecated GitLab CI

```bash
git mv .gitlab-ci.yml ci/legacy-gitlab-ci.yml
sed -i '1i # NOTE: moved to ci/legacy-gitlab-ci.yml during the 2026 repo refresh.' \
  ci/legacy-gitlab-ci.yml
git commit -am "chore: move deprecated .gitlab-ci.yml to ci/legacy-gitlab-ci.yml"
```

## Phase 3 — reorganize `docs/`

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

## Phase 4 — relocate `TODO.codecs/`

```bash
git mv TODO.codecs docs/plans/codecs
# Fix the internal back-reference that assumed a root-level sibling:
grep -rln '\.\./TODO\.codecs' docs/plans/codecs/ \
  | xargs -r sed -i 's#\.\./TODO\.codecs#.#g'
git commit -m "docs: move TODO.codecs/ to docs/plans/codecs/"
```

## Phase 5 — slim `draft/` (the big one)

Delete the vendored upstream Slint documentation (a copy of another project's
docs, not maintained here). Verify the count first.

```bash
# 5a. Remove the upstream Slint mirror (~313 files)
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

If a draft tree is genuinely local scratch, move it to `draft/local/` and add that
to `.gitignore` instead of deleting.

## Phase 6 — rewrite README + add docs index

```bash
$EDITOR README.md docs/README.md
# Fix the NDK rev (r25c -> r28c), add badges, an architecture link, and the CI table.
git add README.md docs/README.md
git commit -m "docs: rewrite README (badges, arch, NDK r28c) and add docs index"
```

## Phase 7 — wire up repo-health automation

```bash
chmod +x scripts/check-repo-layout.sh
git add scripts/check-repo-layout.sh .github/workflows/repo-health.yml
git commit -m "ci: add repo-layout health check and workflow"
```

## Phase 8 — fix dangling links & validate

```bash
# References to moved paths (excluding the plan + checker, which mention them on purpose):
grep -rn --exclude-dir=.git \
  -e 'TODO\.codecs/' -e 'docs/refactor-implementation-guide/' . \
  | grep -v -e 'repository-refresh-plan.md' -e 'check-repo-layout.sh'

bash scripts/check-repo-layout.sh        # must exit 0
```

Update the doc comments in `src/backend/registry.rs` and `src/app.rs` that point
at `docs/refactor-implementation-guide/...` — after Phase 3a those become
`docs/archive/refactor-implementation-guide/...`.

## Phase 9 — full validation

```bash
bash scripts/check-repo-layout.sh
cargo test
./gradlew :app:testDebugUnitTest
ci/ui-validate.sh
cargo check --target aarch64-linux-android   # inside nix develop .#android
```

## Phase 10 — open the PR

```bash
git push -u origin chore/repo-refresh
# Open a PR titled "chore: repository structure refresh"
```

## Rollback

Every phase is its own commit:

```bash
git log --oneline chore/repo-refresh     # find the phase commit
git revert <sha>                         # undo one phase
# or abandon entirely:
git switch main && git branch -D chore/repo-refresh
```

All relocations use `git mv`, so `git log --follow` and `git blame` survive the
move. The only destructive step is Phase 5's `git rm`; recover from history with
`git checkout <sha>^ -- <path>` if ever needed.
