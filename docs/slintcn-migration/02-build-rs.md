# Step 2 — (No build.rs change)

← [Step 1: Install](01-install.md) · [Index](README.md) · Next → [Step 3: Theme](03-theme.md)

Intentionally a no-op. Unlike the original research doc, **do not** rewrite `build.rs` to shell
out to `node slintcn.mjs`. This step is kept numbered so reviewers cross-referencing the original
doc see why it's skipped.

## Why

The current `build.rs` already compiles Slint directly:

```rust
// build.rs (current — leave as-is)
let mut config = slint_build::CompilerConfiguration::new();
if !target.contains("android") {
    config = config.with_debug_info(true);   // host builds: enable element-tree walking for tests
}
slint_build::compile_with_config("ui/main.slint", config).unwrap();
```

slintcn files are **vendored** (committed under `ui/slintcn/`), exactly like the existing
`ui/components/std/` set. The Slint compiler resolves them through normal relative-path imports.
There is nothing to generate at build time.

## What the original research doc proposed (and why it's wrong here)

```rust
// ❌ DO NOT DO THIS — wrong for this repo
let _ = fs::remove_dir_all("ui/slintcn");
Command::new("node").arg("slintcn.mjs").args(["add", "button", …]).status()…;
```

Problems:
- The real CLI is `npx slintcn@latest`, not a checked-in `slintcn.mjs`.
- Regenerating on every build would require Node on every build host/CI runner.
- It fights the repo's established **vendoring** convention (`VENDORING.md`).

← [Step 1: Install](01-install.md) · [Index](README.md) · Next → [Step 3: Theme](03-theme.md)
