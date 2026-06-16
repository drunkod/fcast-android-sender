# Example: `.github/workflows/repo-health.yml`

Annotated copy of the repo-health CI workflow. The **canonical, runnable** version
lives at [`../../.github/workflows/repo-health.yml`](../../.github/workflows/repo-health.yml).

It runs three independent jobs on PRs that touch docs/scripts/README — no Android
toolchain, so it returns in well under a minute:

| Job | What it does |
|---|---|
| `layout` | Runs `scripts/check-repo-layout.sh` |
| `shellcheck` | Lints every `scripts/*.sh` and `ci/*.sh` at `--severity=warning` |
| `markdown-links` | Fails on broken repo-relative Markdown links |

## Source

```yaml
name: Repo Health

# Guards against repository-layout drift after the structure refresh:
# runs the layout checker, lints shell scripts, and detects broken internal
# Markdown links. Fast, no Android toolchain required.

on:
  push:
    branches: [main]
  pull_request:
    paths:
      - "docs/**"
      - "draft/**"
      - "scripts/**"
      - "README.md"
      - "CONTRIBUTING.md"
      - "CHANGELOG.md"
      - "SECURITY.md"
      - ".gitignore"
      - "flake.nix"
      - ".github/workflows/repo-health.yml"
  workflow_dispatch: {}

permissions:
  contents: read

concurrency:
  group: repo-health-${{ github.ref }}
  cancel-in-progress: true

jobs:
  layout:
    name: Layout & consistency
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
      - name: Run repo-layout checks
        run: bash scripts/check-repo-layout.sh

  shellcheck:
    name: Shellcheck scripts
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
      - name: Install shellcheck
        run: sudo apt-get update && sudo apt-get install -y shellcheck
      - name: Lint shell scripts
        run: |
          set -euo pipefail
          mapfile -t scripts < <(find scripts ci -name '*.sh' -type f 2>/dev/null | sort)
          if [ "${#scripts[@]}" -eq 0 ]; then echo "no shell scripts found"; exit 0; fi
          printf 'linting %d scripts\n' "${#scripts[@]}"
          shellcheck --severity=warning "${scripts[@]}"

  markdown-links:
    name: Internal link check
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
      - name: Check relative Markdown links resolve
        run: |
          set -euo pipefail
          if git ls-files '*.md' | while IFS= read -r md; do
              dir="$(dirname "$md")"
              grep -oE '\]\(([^)#]+)' "$md" | sed -E 's/^\]\(//' | while IFS= read -r link; do
                case "$link" in http://*|https://*|mailto:*|"#"*|"") continue ;; esac
                target="${link%%#*}"
                if [ ! -e "${dir}/${target}" ] && [ ! -e "${target}" ]; then echo broken; fi
              done
            done | grep -q broken; then
            echo "::error::Broken internal Markdown links detected"
            exit 1
          fi
          echo "All internal Markdown links resolve."
```

## Notes

- The `markdown-links` job only validates **repo-relative** links; `http(s)://`
  and `mailto:` are skipped. Swap in `lycheeverse/lychee-action@v2` if you also
  want external-URL checking (slower, network-dependent).
- `paths:` filters keep the workflow from running on unrelated code changes.
- The canonical file at `.github/workflows/repo-health.yml` is validated as
  parseable YAML before shipping.
