# Step 7 — Verification & risks

← [Step 6](step-6-android-main-rs.md) · [Index](README.md)

## Verify

1. Host build (exercises the `not(target_os="android")` stubs + Slint codegen):
   `cargo check`
2. Android lib check: `cargo check --target aarch64-linux-android`
3. APK: `./gradlew assembleDebug`
4. Install: `adb install -r app/build/outputs/apk/debug/app-debug.apk`
5. In-app: Settings → **H.264 encoder test** → tap **Run full codec test**.
6. Expect: status `Badge` flips to *running*, buttons disable, then the `ScrollArea`
   log fills and the badge returns to *idle*.
7. Logcat: `adb logcat -s CodecDump`

> Repo has a Nix dev shell — see the `debug-android-nix` skill / `nix develop` if any
> of the toolchain binaries (`cargo`, `adb`, `gradlew` JDK) are missing on PATH.

---

## Risks / notes

- **`@JvmStatic` is load-bearing.** Without it the methods land on `CodecDump$Companion`
  and the `call_static_method(class, …)` upcall fails at runtime (not compile time).
- **ScrollArea vs ScrollView.** slintcn `ScrollArea` has no auto content sizing — it needs
  `content-height` and a width-pinned child. The std `ScrollView` in the raw research
  snippet would size itself but reintroduces a `std-widgets` dependency the migration is
  removing. Plan keeps `ScrollArea`.
- **Smoke test side effects.** `smokeTestVideoEncoders` actually `configure`s + `start`s
  encoders. It releases them in `finally`, but on flaky vendor codecs a failed start can
  leave a codec instance briefly held — hence "Dump codecs only" as the safe default.
- **`vm()` visibility.** It is `pub(crate)`; `codec_test.rs` lives in the same crate so the
  import compiles. If the module is ever moved out of the crate, switch to a `pub` re-export.
- **Long reports.** A full `ALL_CODECS` + `REGULAR_CODECS` dump is large; it's a single
  `set_codec_test_log` string. Fine for a debug panel; not a streaming log.

---

← [Step 6](step-6-android-main-rs.md) · [Index](README.md)
