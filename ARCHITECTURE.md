# fcast-android-sender — Architecture

Standalone Android **sender** app for the [FCast protocol](https://github.com/kodyka/fcast).
A thin Kotlin/Android shell hosts a Rust core (`fcastsender`, built as a `cdylib`)
that drives a Slint UI and a GStreamer/WebRTC media pipeline. The Kotlin and Rust
sides talk over a JNI boundary; media frames captured on the Android side are
pushed into the Rust media-graph runtime, encoded via GStreamer, and streamed
(WHEP/WebRTC or RTMP) to a discovered FCast receiver.

```mermaid
flowchart TB
    subgraph ANDROID["Android shell — Kotlin / Java (org.fcast.android.sender)"]
        direction TB
        App["FcastApp (Application)"]
        Graph["AppGraph — composition root"]
        Act["MainActivity — Slint NativeActivity host"]

        subgraph SVC["Foreground services"]
            direction LR
            SCS["ScreenCaptureService"]
            GPS["GstPopService"]
            MRS["MigrationRuntimeService"]
            CBS["CodecBenchmarkService (AIDL)"]
        end

        subgraph CAP["Capture"]
            direction LR
            CapEng["CaptureEngine / CameraCaptureEngine"]
            ScrCoord["ScreenCaptureCoordinator"]
            CamCoord["CameraCaptureCoordinator"]
            SPCoord["StreamPackCameraCoordinator"]
        end

        subgraph KBR["Runtime + JNI bridges (Kotlin)"]
            direction LR
            RB["RuntimeBridge / JniRuntimeBridge"]
            GPB["GstPopServiceBridge"]
            MRB["MigrationRuntimeServiceBridge"]
            SSB["SecretStoreBridge"]
        end

        Disc["discovery.FCastDiscoveryListener (mDNS)"]
        Secret["data.AndroidSecretStore"]
        Shell["shell.SenderController / UiState"]
        GstJava["org.freedesktop.gstreamer — GStreamer Java glue"]
    end

    subgraph JNI["JNI boundary — lib.rs (Java_org_fcast_android_sender_*)"]
        direction LR
        JEntry["android_main — bootstrap"]
        JBridge["jni_bridge: main_activity, discovery,<br/>gstpop_bridge, migration_bridge, camera, helpers"]
    end

    subgraph RUST["Rust core — crate android-sender / lib fcastsender (cdylib)"]
        direction TB
        AppRs["app.rs — App::production (composition)"]
        Application["application — Application, defaults, status"]

        subgraph BK["backend"]
            direction LR
            Reg["BackendRegistry (Gstpop | Migration)"]
            GBe["gstpop_backend"]
            MBe["migration_backend"]
            Life["lifecycle / persistence"]
        end

        Plat["platform — platform_app, panel_stack, gst_init"]
        Cmd["command — http_runner, probe"]
        Cfg["config — migration"]
        SecretRs["secret — JNI secret store"]
        Codec["codec_perf / codec_bench"]
        Channels["Global state: GLOB_EVENT_CHAN,<br/>CAMERA_EVENT_CHAN, FRAME_PAIR, FRAME_POOL"]
    end

    subgraph CRATES["Workspace crates"]
        direction LR
        GpRt["gstpop-runtime — embedded gst-pop daemon + JSON-RPC client"]
        MigRt["migration-runtime — media graph: nodes, node_manager,<br/>runtime, media_bridge, whep_signaller_compat, FramePair"]
        Vendor["vendor/gstpop"]
    end

    subgraph UI["Slint UI (ui/)"]
        direction LR
        Main["main.slint — MainWindow"]
        Bridge["bridge.slint — Bridge global"]
        Pages["pages/ (~35: cast, camera, rtmp, mixer, scenes, codec...)"]
        Theme["theme / components / state / i18n"]
    end

    subgraph EXT["External"]
        direction LR
        SDK["FCast SDK (git-pinned): fcast-protocol,<br/>fcast-sender-sdk, mcore"]
        GStreamer["GStreamer + gst-plugin-webrtc (WHEP)"]
        Receiver["FCast receiver / RTMP endpoint"]
    end

    %% Android wiring
    App --> Graph
    Graph --> RB & Secret & CAP
    Act --> Bridge
    Act -.captures.-> CAP
    CAP --> SCS
    RB --> GPB & MRB
    Secret --> SSB
    GstJava -.native init.-> GStreamer

    %% Kotlin -> JNI
    Act ==> JBridge
    GPB ==> JBridge
    MRB ==> JBridge
    SSB ==> JBridge
    Disc ==> JBridge
    CAP == "frame buffers (nativeProcessFrame)" ==> JBridge

    %% JNI -> Rust
    JEntry --> AppRs
    JBridge --> AppRs
    AppRs --> Application & BK & Plat
    Application --> Channels
    JBridge --> Channels

    %% Rust -> crates
    GBe --> GpRt
    MBe --> MigRt
    Reg --> GBe & MBe
    Plat --> GStreamer
    GpRt --> Vendor
    Channels -.frames.-> MigRt

    %% Media + protocol
    MigRt --> GStreamer
    GpRt --> GStreamer
    GStreamer == "WHEP / WebRTC / RTMP" ==> Receiver
    Application --> SDK
    Disc -. discovers .-> Receiver

    %% UI binding
    AppRs -.drives.-> Main
    Main --> Pages & Theme
    Bridge -. callbacks/properties .- AppRs
```

## Layers

**Android shell (Kotlin/Java).** `FcastApp` builds `AppGraph`, the single
composition root that wires the runtime bridge, secret store, and capture
coordinators. `MainActivity` hosts the Slint `NativeActivity` and forwards UI and
lifecycle events. Four foreground services back long-running work: screen
capture, the gst-pop host, the migration runtime host, and an AIDL codec
benchmark service. Capture coordinators feed raw frames; `FCastDiscoveryListener`
finds receivers over mDNS.

**JNI boundary.** `src/lib.rs` exports the `Java_org_fcast_android_sender_*`
symbols. `android_main` bootstraps the process (JavaVM, classloader, Slint event
loop), and the `jni_bridge` modules dispatch each native call into the core.

**Rust core (`android-sender` → `libfcastsender.so`).** `app.rs` is the
composition root; `application` holds app state. The `backend` layer exposes a
`MediaBackend` trait selected through a `BackendRegistry` with two
implementations — `gstpop` and `migration`. Cross-thread frame/event handoff uses
the global `FRAME_PAIR`, `FRAME_POOL`, and crossbeam channels.

**Workspace crates.** `gstpop-runtime` is an in-process gst-pop daemon plus
JSON-RPC client; `migration-runtime` is the node-based media-graph runtime
(sources, mixers, destinations, WHEP signaller compat). Both drive GStreamer.

**Slint UI.** `main.slint` defines `MainWindow`; `bridge.slint`'s `Bridge` global
is the property/callback contract between Rust and the ~35 pages.

**External.** FCast SDK crates (`fcast-protocol`, `fcast-sender-sdk`, `mcore`) are
git-pinned to `kodyka/fcast`. Encoding and transport run through GStreamer and
`gst-plugin-webrtc` (WHEP), terminating at a discovered FCast receiver or RTMP
endpoint.

## End-to-end data flow

Screen or camera frames are captured on the Android side, pushed across JNI
(`nativeProcessFrame`) into `FRAME_PAIR`, consumed by the migration-runtime media
graph (or the gst-pop path), encoded by GStreamer, and streamed via WHEP/WebRTC
or RTMP to a receiver discovered over mDNS. Control/state flows back up through
the `Bridge` global into the Slint UI.
