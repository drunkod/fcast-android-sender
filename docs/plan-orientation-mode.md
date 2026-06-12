# Orientation Mode — Implementation Plan

Ported from Moblin (Swift/iOS) to FCast Android (Kotlin + Rust/GStreamer).

---

## What Moblin Does (Swift reference)

| Moblin concept | File | Notes |
|---|---|---|
| `Orientation` (ObservableObject, `isPortrait: Bool`) | `Model.swift:337` | Published state drives every view |
| `getOrientation()` → `UIDeviceOrientation` | `UiUtils.swift:18` | Reads `UIDevice.current.orientation`, falls back to `UIWindowScene` |
| `orientationDidChange()` → `updateOrientation()` | `Model.swift:1618` | Registered on `UIDevice.orientationDidChangeNotification` |
| `updateOrientation()` | `Model.swift:1601` | Calls `media.setVideoOrientation()` and refreshes preview layer |
| `updateOrientationLock()` | `Model.swift:2035` | Sets `AppDelegate.orientationLock` → `.portrait` or `.landscape` |
| `stream.portrait: Bool` | `Settings.swift:1105` | Per-stream setting, persisted in JSON |
| `AVCaptureVideoOrientation` on `connection` | `VideoUnit.swift:247` | Applied per camera output connection |
| `isLandscapeStreamAndPortraitUi` logic | `VideoUnit.swift:28` | Handles landscape-stream-in-portrait-UI edge case |
| `videoflip horizontal-flip` | already in Android | Mirror; rotation is a separate `method` value |
| Quick button `.portrait` | `Settings.swift:1885` | Toggle in control bar |
| `ControlBarPortraitView` / `ControlBarLandscapeView` | separate Swift views | Layout switches on `orientation.isPortrait` |

---

## Android / Rust equivalents

| Moblin (Swift/iOS) | FCast Android target |
|---|---|
| `UIDevice.orientationDidChangeNotification` | `OrientationEventListener` (API 9+) |
| `AVCaptureVideoOrientation` | `SENSOR_ORIENTATION` + device rotation math → degrees `{0,90,180,270}` |
| `Orientation` ObservableObject | `StateFlow<OrientationMode>` exposed to Slint via adapter |
| `AppDelegate.orientationLock` | `requestedOrientation` on `MainActivity` |
| `stream.portrait: Bool` | `OrientationMode` enum in `CameraCaptureConfig` + Rust `CameraSourceConfig` |
| `setVideoOrientation(value:)` → `videoUnit.videoOrientation` | `videoflip` `method` property on existing GStreamer element |
| Quick button `.portrait` | Slint control-bar `orientation-mode` toggle button |
| `ControlBarPortraitView/LandscapeView` | Slint `if` on `is-portrait` property in `control_bar.slint` |

---

## Rotation Math (Camera2)

```
sensorDegrees = CameraCharacteristics.SENSOR_ORIENTATION   // e.g. 90 for most phones
deviceDegrees = {0, 90, 180, 270}                          // from OrientationEventListener

// Back camera
videoRotation = (sensorDegrees - deviceDegrees + 360) % 360

// Front camera (mirror flips the axis)
videoRotation = (sensorDegrees + deviceDegrees) % 360
```

Map to GStreamer `videoflip` method:

| degrees | `videoflip` method string |
|---|---|
| 0 | `"none"` |
| 90 | `"clockwise"` |
| 180 | `"rotate-180"` |
| 270 | `"counterclockwise"` |

---

## Phases

### Phase 1 — `OrientationMode` type (Kotlin + Rust)

**Kotlin** — add to `CameraCaptureConfig.kt`:

```kotlin
enum class OrientationMode { PORTRAIT, LANDSCAPE, AUTO }

data class CameraCaptureConfig(
    val cameraIdx: Int,
    val width: Int,
    val height: Int,
    val maxFps: Int,
    val mirror: Boolean = false,
    val stabilization: Boolean = true,
    val zoom: Float = 1.0f,
    val orientationMode: OrientationMode = OrientationMode.PORTRAIT,  // NEW
) { ... }
```

**Rust** — add to `crates/migration-runtime/src/nodes/camera_source.rs` (or a shared types file):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OrientationMode {
    #[default]
    Portrait,
    Landscape,
    Auto,
}
```

Add `orientation_mode: OrientationMode` to `CameraSourceConfig` (wherever camera params are stored in Rust).

---

### Phase 2 — Device orientation sensor (Kotlin)

Create `OrientationSensor.kt` in `capture/`:

```kotlin
class OrientationSensor(context: Context) {
    private val listener = object : OrientationEventListener(context) {
        override fun onOrientationChanged(degrees: Int) {
            if (degrees == ORIENTATION_UNKNOWN) return
            val snapped = when {
                degrees in 315..360 || degrees in 0..44   -> 0
                degrees in 45..134                        -> 90
                degrees in 135..224                       -> 180
                else                                      -> 270
            }
            _deviceRotation.value = snapped
        }
    }

    private val _deviceRotation = MutableStateFlow(0)
    val deviceRotation: StateFlow<Int> = _deviceRotation.asStateFlow()

    fun start() { listener.enable() }
    fun stop()  { listener.disable() }
}
```

Start in `CameraCaptureEngine.start()`, stop in `shutdown()`.

---

### Phase 3 — Camera2 rotation compensation (Kotlin)

In `CameraCaptureEngine`, read `SENSOR_ORIENTATION` from `CameraCharacteristics` and combine with `OrientationSensor.deviceRotation`:

```kotlin
private fun calcVideoRotation(chars: CameraCharacteristics, isFront: Boolean, deviceDeg: Int): Int {
    val sensor = chars.get(CameraCharacteristics.SENSOR_ORIENTATION) ?: 0
    return if (isFront) (sensor + deviceDeg) % 360
           else         (sensor - deviceDeg + 360) % 360
}
```

When `OrientationMode.AUTO` is active, recompute on each `deviceRotation` change and call a new `nativeSetCameraRotation(deg: Int)` JNI method (see Phase 5).

When `PORTRAIT` or `LANDSCAPE` is fixed, lock to the corresponding degrees (0 or 90) regardless of sensor.

---

### Phase 4 — GStreamer `videoflip` for rotation (Rust)

`camera_source.rs` already creates a `videoflip` element for mirroring. Extend to also handle rotation:

```rust
// Existing mirror element uses method = "horizontal-flip"
// Add a separate rotation element, or combine into one if only one transform is needed.

fn videoflip_method(rotation_deg: u32, mirror: bool) -> &'static str {
    match (rotation_deg, mirror) {
        (0,   false) => "none",
        (0,   true)  => "horizontal-flip",
        (90,  false) => "clockwise",
        (90,  true)  => "upper-right-diagonal",   // clockwise + h-flip
        (180, false) => "rotate-180",
        (180, true)  => "vertical-flip",
        (270, false) => "counterclockwise",
        (270, true)  => "upper-left-diagonal",    // ccw + h-flip
        _            => "none",
    }
}
```

Wire into `build_live_pipeline()` — replace the existing conditional `mirror_elem` block with a single `videoflip` element whose `method` encodes both transforms.

Add a `set_rotation(deg: u32)` method that calls `element.set_property_from_str("method", ...)` at runtime so `AUTO` mode can update without pipeline teardown.

---

### Phase 5 — JNI bridge update (Rust + Kotlin)

**Rust** `src/jni_bridge/camera.rs` — add `rotation_deg` parameter to `upcall_start_camera_capture` and `upcall_start_camera_preview`:

```rust
pub fn upcall_start_camera_capture(
    camera_idx: u32, w: u32, h: u32, fps: u32,
    mirror: bool, stabilization: bool, zoom: f32,
    rotation_deg: u32,           // NEW
) -> Result<(), String> {
    // JNI signature changes from "(IIIIZZF)V" to "(IIIIZZFI)V"
}
```

Add a new `upcall_set_camera_rotation(deg: u32)` for live rotation updates (AUTO mode).

**Kotlin** `MainActivity.kt` — update `startCameraCapture` signature and add `setCameraRotation(deg: Int)` native + JNI-receiver method.

---

### Phase 6 — Activity screen lock (Kotlin)

```kotlin
fun applyOrientationLock(mode: OrientationMode) {
    requestedOrientation = when (mode) {
        OrientationMode.PORTRAIT  -> ActivityInfo.SCREEN_ORIENTATION_PORTRAIT
        OrientationMode.LANDSCAPE -> ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE
        OrientationMode.AUTO      -> ActivityInfo.SCREEN_ORIENTATION_UNSPECIFIED
    }
}
```

Call from the stream settings change handler (mirrors `updateOrientationLock()` in Moblin).

---

### Phase 7 — Slint UI: toggle + adaptive layout

**State** — expose in adapter:

```slint
// AppState or equivalent
in property <bool> is-portrait: true;
in-out property <int> orientation-mode: 0;  // 0=portrait 1=landscape 2=auto
```

**Control bar quick button** (`ui/components/control_bar.slint`):

```slint
Button {
    icon: root.orientation-mode == 0 ? @image-url("icons/portrait.svg")
                                     : @image-url("icons/landscape.svg");
    clicked => { root.orientation-mode-toggled(); }
}
```

**Adaptive layout** — mirrors Moblin's `ControlBarPortraitView` / `ControlBarLandscapeView` split:

```slint
if root.is-portrait : ControlBarPortrait { ... }
if !root.is-portrait : ControlBarLandscape { ... }
```

**Stream settings page** — add a `ComboBox` or `RadioGroup` for `OrientationMode` (portrait / landscape / auto), persisted via the Rust settings layer.

---

## File change map

| File | Change |
|---|---|
| `app/src/main/java/.../capture/CameraCaptureConfig.kt` | Add `OrientationMode` enum + field |
| `app/src/main/java/.../capture/OrientationSensor.kt` | New file — `OrientationEventListener` wrapper |
| `app/src/main/java/.../capture/CameraCaptureEngine.kt` | Start/stop sensor, compute rotation, call native on change |
| `app/src/main/java/.../MainActivity.kt` | `applyOrientationLock()`, updated `startCameraCapture` signature, new `setCameraRotation` |
| `src/jni_bridge/camera.rs` | Add `rotation_deg` param, new `upcall_set_camera_rotation` |
| `crates/migration-runtime/src/nodes/camera_source.rs` | `videoflip_method()` helper, combine mirror+rotation into one element, `set_rotation()` method |
| `ui/components/control_bar.slint` | Orientation toggle button, conditional portrait/landscape layout |
| `ui/pages/camera_rtmp_stream_page.slint` | Orientation mode setting row |
| `ui/state/` (or adapter) | `is-portrait`, `orientation-mode` properties |

---

## Open questions

1. **Canvas dimensions** — when switching portrait↔landscape, should the encode resolution swap (e.g. 1080×1920 ↔ 1920×1080)? Moblin swaps via `resolution.dimensions(portrait:)`. Needs a decision before Phase 1 type definition.

2. **AUTO mode liveness** — does the pipeline need to stay running while the device rotates, or is a brief restart acceptable? Live `videoflip` property update avoids restart but complicates the state machine.

3. **Screen capture (`CaptureEngine`)** — `MediaProjection` always captures at the device's current screen dimensions. Orientation mode is less relevant there (the projection IS the screen), but `VirtualDisplay` resize-on-rotation is a separate concern.

4. **Slint layout** — the current `control_bar.slint` comment says it mirrors `ControlBarPortraitView.swift`. If landscape layout is added, decide whether it's a second component file or an `if` branch in the same file.
