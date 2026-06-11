# STEP-3 — Scene compositor via `MixerNode` reuse (sub-steps)

> The performance-critical step: a widget = a mixer slot; `SetScene` emits only
> existing `Connect`/`Disconnect`/`AddControlPoint` primitives. No new compositor.
> **Must land with STEP-2.** **Refs:** PHASE-40 §40-B, mapping doc §4.

## Why reuse the mixer

`MixerNode` (`nodes/mixer.rs`) is already a `compositor` with dynamic sink pads;
its slot validator accepts `x/y/width/height/zorder/alpha` (`mixer.rs:177`) and
`set_dynamic_pad_property` (`mixer.rs:323`) pushes them onto pads. Slot props
arrive via `Connect { config }` and update live via `AddControlPoint`.

| Scene/Widget concept | Existing primitive |
|---|---|
| compositor | `CreateMixer` |
| camera base layer | `Connect camera→mixer` `video::zorder=0` |
| widget overlay | `Connect widget-src→mixer` `video::x/y/width/height/zorder/alpha` |
| move/resize live | `AddControlPoint { property: "video::x", mode: Set }` |
| remove widget | `Disconnect { link_id }` |
| crop | `videocrop` on camera source (STEP-4), not a slot |

| # | File | Scope |
|---|------|-------|
| 3A | [STEP-3A-registry.md](STEP-3A-registry.md) | `SceneRegistry` in `NodeManager` |
| 3B | [STEP-3B-setscene-expansion.md](STEP-3B-setscene-expansion.md) | `SetScene` diff → graph primitives + layout→slot mapping |
| 3C | [STEP-3C-layout-and-crud.md](STEP-3C-layout-and-crud.md) | `UpdateWidgetLayout` + scene/widget CRUD |
| 3D | [STEP-3D-switching-and-tests.md](STEP-3D-switching-and-tests.md) | quick-switch cost + tests |

→ Next: [../step-4/INDEX.md](../step-4/INDEX.md)
