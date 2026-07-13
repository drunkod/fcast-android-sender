# STEP-8D — Manual-connect button, registration & v0.1.0 wrap-up

> The fallback `Button`, the `main.slint` registration note, verification, and
> the full-plan completion summary. Closes the file opened in 8A.

---

## The change — append as the last child of the outer `VerticalLayout`

**File:** `ui/pages/connect_page.slint`

```slint
        // ── Manual connect button (always shown as fallback) ──────────────────
        Button {
            text: @tr("Enter address manually");
            variant: ButtonVariant.outline;
            size: ButtonSize.default;
            clicked => {
                // MVP-PHASE-1 follow-up: push a manual-address entry panel.
                // For now opens settings where the receiver list lives.
                Bridge.invoke-action("open-settings");
            }
        }
    }
}
```

The trailing `}` braces close the outer `VerticalLayout` and the `ConnectView`
component opened in 8A.

---

## Complete file

The full `connect_page.slint` = 8A's shell + header, then 8B's empty state, 8C's
receiver-list `Card`, and 8D's manual `Button`, in that order inside the outer
`VerticalLayout`. No snippet is duplicated across sub-steps.

---

## Registration

`ConnectView` is exported and already consumed by the app state machine — the
component name is unchanged, so **no `main.slint` import change is needed**.
Confirm the existing reference:

```bash
grep -n "ConnectView\|connect_page" ui/main.slint
# → expect one import line + one usage site
```

---

## Rust side (no changes required)

`Bridge.connect-receiver(id)` and `Bridge.invoke-action("open-settings")` are
already wired in the JNI/backend layer. The new UI calls the same callbacks —
no Rust changes for this step.

---

## Verification

```bash
slint-lsp ui/main.slint 2>&1 | grep error
# → (none)
```

Manual walkthrough:
1. Zero devices → empty state (satellite emoji + "No receivers found").
2. Devices present (mock fixture) → list renders; tapping a row fires
   `Bridge.connect-receiver(device.id)`.
3. Default receiver shows the "Default" `Badge`.
4. "Enter address manually" fires `invoke-action("open-settings")`.

---

## Done — STEP-8 complete

| Sub-step | Status |
|---|---|
| 8A scaffold + header | ✅ |
| 8B empty state | ✅ |
| 8C receiver list | ✅ |
| 8D manual button + done | ✅ |

---

## v0.1.0 — all eight steps complete

| Step | Folder | Status |
|---|---|---|
| 1 `DestinationFamily::Srt` | [../step-1/](../step-1/INDEX.md) | ✅ |
| 2 Pipeline profile arm | [../step-2/](../step-2/INDEX.md) | ✅ |
| 3 `build_live_pipeline` arm | [../step-3/](../step-3/INDEX.md) | ✅ |
| 4 Unit tests | [../step-4/](../step-4/INDEX.md) | ✅ |
| 5 Bridge Panel enum | [../step-5/](../step-5/INDEX.md) | ✅ |
| 6 RTMP settings page | [../step-6/](../step-6/INDEX.md) | ✅ |
| 7 SRT settings page | [../step-7/](../step-7/INDEX.md) | ✅ |
| 8 Connect page | [../step-8/](INDEX.md) | ✅ |

**Squash rule:** Steps 1 + 2 + 3 land in one commit (exhaustive `match`).
Steps 4–8 are independent and can land separately.

Next milestone: [v0.2.0 — Scene System + Basic Widgets](../../../../draft/moblin-fcast-version-map.md#v020--scene-system--basic-widgets)
