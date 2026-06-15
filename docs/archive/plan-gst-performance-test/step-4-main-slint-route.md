# Step 4 — `ui/main.slint`: import + route

← [Step 3](step-3-codec-perf-page-slint.md) · [Index](README.md) · Next → [Step 5](step-5-settings-entry.md)

### 4a — Import

`CodecTestPage` is imported at **line 70**. Add the perf page import right after:

```slint
import { CodecTestPage }                from "pages/codec_test_page.slint";
import { CodecPerfPage }                from "pages/codec_perf_page.slint";   // ← ADD
```

### 4b — Route inside `PanelHost`

The `codec-test` route is at **line 178**. Add the perf route directly after:

```slint
            if PanelBridge.active == Panel.codec-test:          CodecTestPage           { }
            if PanelBridge.active == Panel.codec-perf:          CodecPerfPage           { }   // ← ADD
            if PanelBridge.active == Panel.backup-reset:        BackupResetPage         { }
```

---

← [Step 3](step-3-codec-perf-page-slint.md) · [Index](README.md) · Next → [Step 5](step-5-settings-entry.md)
