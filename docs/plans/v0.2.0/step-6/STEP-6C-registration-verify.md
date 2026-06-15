# STEP-6C — Registration & verify

**File:** `ui/main.slint`

```slint
import { SceneListPage } from "pages/scene_list_page.slint";
import { SceneEditPage } from "pages/scene_edit_page.slint";
// inside PanelHost:
if PanelBridge.active == Panel.scene-list: SceneListPage { }
if PanelBridge.active == Panel.scene-edit: SceneEditPage { }
```

> `Bridge.open-scene-edit(id)` (Rust, STEP-9) sets `editing-scene-id` +
> `editing-scene-widgets`, then `PanelBridge.push(Panel.scene-edit)`.

## Verify

```bash
slint-lsp ui/main.slint 2>&1 | grep error   # → (none)
```
Manual: create scene → appears in list; tap → edit; toggle enable; quick-group
cycles 0→4→0; delete pops back.

## Done — STEP-6 complete

→ Next: [../step-7/INDEX.md](../step-7/INDEX.md)
