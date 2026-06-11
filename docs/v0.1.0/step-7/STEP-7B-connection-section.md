# STEP-7B — Connection section (SRT URL)

> First section in `body`: the SRT URL field, built with the slintcn `Input`.

---

## Goal

A `Card` with a labelled slintcn `Input` two-way bound to `draft-uri`. The URL
carries the SRT mode (`?mode=listener`) when needed — see the note below.

---

## slintcn `Input` API (from registry)

```slint
import { Input } from "slintcn/components/input.slint";
Input {
    placeholder: "you@example.com";
    text <=> email;
    edited(t) => { /* … */ }
}
```

The installed `input.slint` also exposes `password: bool` (used in 7C for the
passphrase) and `read-only` / `enabled`.

---

## The change — append to `body`

**File:** `ui/pages/protocol_srt_settings_page.slint` (inside `body`, first child)

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
                            // SRT URI input. Append `?mode=listener` here to accept
                            // inbound connections; default (caller) connects outward.
                            Rectangle {
                                height: 72px;
                                background: RowColors.normal;
                                VerticalLayout {
                                    padding: Theme.padding-screen;
                                    spacing: 4px;
                                    alignment: center;
                                    Text {
                                        text: @tr("SRT URL");
                                        color: Theme.text-secondary;
                                        font-size: Theme.font-size-label;
                                    }
                                    Input {
                                        text <=> root.draft-uri;
                                        placeholder: "srt://media.example.com:9000";
                                        edited(v) => { root.draft-uri = v; }
                                    }
                                }
                            }
                        }
                    }
```

---

## Why `Input` is wrapped in a labelled `Rectangle`

slintcn `Input` is a bare single-line field (focus ring + placeholder). The
settings idiom here is *label above field* in a 72px row, so the `Input` sits
inside a `Rectangle { VerticalLayout { Text(label); Input } }`. The `Card`
provides the rounded group surface; `RowColors.normal` matches the other rows.

> `text <=> root.draft-uri` already keeps the two in sync; the explicit
> `edited(v) => { root.draft-uri = v }` is redundant but harmless and documents
> intent. Either alone is sufficient.

---

## SRT connection mode (`caller` vs `listener`)

No dedicated toggle in v0.1.0 — the mode is part of the URI:

- **caller** (default): `srt://host:port` — phone dials out to a server.
- **listener**: `srt://0.0.0.0:port?mode=listener` — phone accepts inbound.

The field accepts the full query string. A first-class mode picker is v0.2.0.

---

## Next

→ [STEP-7C-transport-section.md](STEP-7C-transport-section.md)
