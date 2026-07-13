# STEP-8C — Quick-switch UX & verify

## Quick-switch group behavior

When the tapped scene shares `quick_switch_group` with the current one, STEP-3
swaps only the compositor slots (instant). A different/none group reattaches the
camera (brief flicker). The button bar doesn't need to know — it just calls
`set-scene`; the cost difference is handled in the runtime.

## Verify

```bash
slint-lsp ui/main.slint 2>&1 | grep error   # → (none)
```
Manual: with 2+ enabled scenes while casting, the bottom bar shows scene buttons;
tapping switches the active highlight and (with widgets) changes the on-stream
overlays; the Scenes row in Settings opens the scene list.

## Done — STEP-8 complete

→ Next: [../step-9/INDEX.md](../step-9/INDEX.md)
