# STEP-6B — Connection section

> First section inside the `body` from [STEP-6A](STEP-6A-page-scaffold.md):
> the Server URL row (links to the existing camera-RTMP panel) and the stream
> key with a reveal/hide toggle.

---

## Goal

A slintcn `Card` holding two rows:
1. **Server URL** — `SettingsValueRow` that pushes `Panel.camera-rtmp-stream`.
2. **Stream Key** — a hand-built row that masks the key (`●●●●`) until tapped.

---

## Moblin → FCast mapping

| Moblin field | FCast equivalent | Notes |
|---|---|---|
| RTMP URL | `Bridge.cam-rtmp-url` | already wired via the `camera-rtmp-stream` panel |
| Stream key | `Bridge.cam-rtmp-stream-key` | masked by default, reveal toggle |

---

## The change — append to `body`

**File:** `ui/pages/protocol_rtmp_settings_page.slint` (inside `body`, first child)

```slint
                    // ── CONNECTION ───────────────────────────────────────────
                    Text {
                        text: @tr("CONNECTION");
                        color: Theme.text-secondary;
                        font-size: Theme.font-size-label;
                        font-weight: 600;
                    }
                    Card {
                        variant: CardVariant.solid;
                        card-padding: CardPadding.none;
                        clip: true;
                        VerticalLayout {
                            spacing: 1px;

                            // Server URL — opens the existing camera-RTMP panel.
                            SettingsValueRow {
                                title: @tr("Server URL");
                                value: Bridge.cam-rtmp-url != ""
                                    ? Bridge.cam-rtmp-url
                                    : @tr("Not set");
                                clicked => { PanelBridge.push(Panel.camera-rtmp-stream); }
                            }

                            // Stream key — masked until tapped (reveal/hide).
                            Rectangle {
                                height: Theme.row-height;
                                background: key-ta.pressed ? RowColors.pressed : RowColors.normal;

                                key-ta := TouchArea {
                                    clicked => {
                                        root.show-stream-key = !root.show-stream-key;
                                    }
                                    HorizontalLayout {
                                        padding-left: Theme.padding-screen;
                                        padding-right: Theme.padding-screen;
                                        spacing: Theme.spacing-default;
                                        alignment: stretch;

                                        Text {
                                            text: @tr("Stream Key");
                                            color: Theme.text-primary;
                                            font-size: Theme.font-size-body;
                                            vertical-alignment: center;
                                            horizontal-stretch: 1;
                                        }
                                        Text {
                                            text: root.show-stream-key
                                                ? (Bridge.cam-rtmp-stream-key != ""
                                                    ? Bridge.cam-rtmp-stream-key
                                                    : @tr("Not set"))
                                                : "●●●●●●●●";
                                            color: Theme.text-secondary;
                                            font-size: Theme.font-size-body;
                                            vertical-alignment: center;
                                        }
                                        Text {
                                            text: root.show-stream-key ? @tr("Hide") : @tr("Show");
                                            color: Theme.accent;
                                            font-size: Theme.font-size-label;
                                            vertical-alignment: center;
                                        }
                                    }
                                }
                            }
                        }
                    }
```

---

## Why the Card wraps `card-padding: none` + `spacing: 1px`

`CardVariant.solid` gives the rounded settings-group surface; `CardPadding.none`
keeps it flush so the inner `VerticalLayout { spacing: 1px }` renders the 1px
gaps as hairline dividers between rows. This is exactly the `SettingsSection`
idiom from `settings_rows.slint` — reused inline here so the Stream Key row
(which is not a `SettingsValueRow`) sits flush with the URL row.

---

## Why Stream Key is hand-built, not `SettingsValueRow`

`SettingsValueRow` shows a static value; the stream key needs a *tri-part* row
(label · masked/clear value · Show/Hide affordance) and a tap that toggles
local state rather than navigating. A thin `Rectangle`+`TouchArea` using
`RowColors` for the press feedback is the minimal composition.

---

## Verification

Renders after 6A's shell; full compile check in 6D.

---

## Next

→ [STEP-6C-reliability-performance.md](STEP-6C-reliability-performance.md)
