# Repository health checks

Automated guards that keep the repository layout from drifting back into its
pre-refresh state. Two real files implement this:

- [`scripts/check-repo-layout.sh`](../../scripts/check-repo-layout.sh) — the guard (annotated copy: [examples/check-repo-layout.sh.md](../examples/check-repo-layout.sh.md))
- [`.github/workflows/repo-health.yml`](../../.github/workflows/repo-health.yml) — runs it on PRs (annotated copy: [examples/repo-health-workflow.yml.md](../examples/repo-health-workflow.yml.md))

## What the script enforces

1. **Required files exist** — `README`, `LICENSE`, `.gitignore`, `CONTRIBUTING.md`,
   `SECURITY.md`, `CHANGELOG.md`, `Cargo.toml`, `settings.gradle`, `flake.nix`.
2. **Stale paths are gone** — `TODO.codecs/`, root `.gitlab-ci.yml`, the
   `draft/slint-ui/docs/{astro,safety,nodejs,…}` upstream mirror.
3. **No loose `draft-plan-*.md`** at the `docs/` root.
4. **README NDK rev matches `flake.nix`** — the single-sourcing guard for the
   toolchain version (the original bug was README r25c vs flake r28c).
5. **No dangling references** to moved paths (`TODO.codecs/`,
   `refactor-implementation-guide/`).
6. **`CONTRIBUTING.md` is resolvable** at root or `.github/`.

## Usage

```bash
./scripts/check-repo-layout.sh           # CI mode: non-zero exit on drift
./scripts/check-repo-layout.sh --warn    # report only, always exit 0
./scripts/check-repo-layout.sh --quiet   # print failures only
```

During the migration, run with `--warn` and watch failures turn green phase by
phase; flip CI to hard-fail once the refresh PR is clean.

Example output before the refresh (drift present):

```text
Toolchain version consistency
  FAIL README NDK rev disagrees with flake.nix
       flake pins 28.0.13004108 (r28); README says 'NDK r25c'

Community file placement
  FAIL CONTRIBUTING.md not found at root or .github/
────────────────────────────────────────
repo-health: 9 passed, 4 failed
```

## The workflow

`repo-health.yml` runs three independent jobs on PRs that touch docs, scripts, or
the README — no Android toolchain, so it finishes in well under a minute:

| Job | What it does |
|---|---|
| `layout` | Runs `scripts/check-repo-layout.sh` |
| `shellcheck` | Lints every `scripts/*.sh` and `ci/*.sh` at `--severity=warning` |
| `markdown-links` | Fails on broken repo-relative Markdown links |

## Failure policy

A pull request should fail when:

- A required file is missing or a forbidden stale path reappears.
- A loose draft plan is committed at the `docs/` root.
- The README's NDK revision disagrees with `flake.nix`.
- An internal Markdown link points at a moved or missing doc.
- A shell script has a `shellcheck` warning or error.

## Extending the checks

Add a new invariant by appending a `section`/`pass`/`fail` block in
`scripts/check-repo-layout.sh`. Keep checks dependency-free (coreutils + grep +
find) so they run identically on a laptop and in CI.
