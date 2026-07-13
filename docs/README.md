# Documentation index — `fcast-android-sender`

Standalone Android **sender** app for the [FCast protocol](https://github.com/kodyka/fcast):
a thin Kotlin/Android shell hosting a Rust core (`fcastsender`, `cdylib`) that
drives a Slint UI and a GStreamer/WebRTC media pipeline over a JNI boundary.

## Start here

| If you want to… | Read |
|---|---|
| See current status and next work | [plans/README.md](plans/README.md) |
| Review branch cleanup decisions | [plans/branch-audit-2026-07-13.md](plans/branch-audit-2026-07-13.md) |
| Understand how the pieces fit together | [architecture/overview.md](architecture/overview.md) |
| Build and deploy the app | [build/android-build.md](build/android-build.md) |
| Contribute code | [development/contributor-workflow.md](development/contributor-workflow.md) |
| Understand the repo-health automation | [development/repo-health-checks.md](development/repo-health-checks.md) |
| Review the completed repo refresh runbook | [migrations/repo-refresh/step-by-step.md](migrations/repo-refresh/step-by-step.md) |
| See the completed refresh rationale | [repository-refresh-plan.md](repository-refresh-plan.md) |
| Read the annotated example files | [examples/check-repo-layout.sh.md](examples/check-repo-layout.sh.md) · [examples/repo-health-workflow.yml.md](examples/repo-health-workflow.yml.md) |

## Directory map

```text
docs/
├── README.md                          # this index
├── repository-refresh-plan.md         # the consolidated plan (overview + rationale)
├── plans/
│   ├── README.md                      # current implementation roadmap
│   ├── branch-audit-2026-07-13.md     # local/remote branch disposition
│   └── codecs/                        # reconciled codec backlog
├── architecture/
│   └── overview.md                    # layered architecture + Mermaid diagram
├── build/
│   └── android-build.md               # toolchain, Nix/manual builds, logs, UI preview
├── development/
│   ├── contributor-workflow.md        # setup, hooks, branches, tests, CI table
│   └── repo-health-checks.md          # the layout guard + workflow, explained
├── examples/
│   ├── check-repo-layout.sh.md        # annotated copy of the health script
│   └── repo-health-workflow.yml.md    # annotated copy of the CI workflow
└── migrations/
    └── repo-refresh/
        └── step-by-step.md            # phased, copy-paste migration runbook
```

Completed v0.1.0 and v0.2.0 implementation plans live under
[`archive/plans/`](archive/plans/).

The runnable versions of the two examples live at their real paths:
[`scripts/check-repo-layout.sh`](../scripts/check-repo-layout.sh) and
[`.github/workflows/repo-health.yml`](../.github/workflows/repo-health.yml).

## Documentation rules

- *Active* docs live under a committed `docs/` subdir.
- *Completed/historical* docs move to `docs/archive/<topic>/`.
- *Active migration plans* live under `docs/migrations/<topic>/`.
- *Vendored upstream copies* (e.g. a mirror of the Slint docs) do **not** belong
  in this repo.
- *Local scratch notes* live in `draft/local/` (git-ignored) and are not committed.
- Toolchain version numbers are single-sourced from `flake.nix` and enforced by
  the repo-health check — see [repo-health-checks.md](development/repo-health-checks.md).
