# STEP-4D — Clock renderer

A clock is a Text widget re-rendered on a timer. Reuse `render_text_widget`,
formatting `chrono::Local::now()` each second, and `push_rgba` to the slot:

```rust
// Spawn when the clock widget source is created; stop on remove.
let appsrc = node.appsrc.clone();
std::thread::spawn(move || loop {
    let now = chrono::Local::now().format("%H:%M:%S").to_string();
    if let Ok(rgba) = render_text_widget(&now, w, h, 32.0, [255,255,255,255]) {
        if push(&appsrc, &rgba).is_err() { break; }
    }
    std::thread::sleep(std::time::Duration::from_millis(1000));
});
```

Use the widget's `Clock.format` for the strftime string and `font_size`/`color`
for rendering. Stop the thread on widget removal (a shared `AtomicBool` flag, as
the v0.1.0 keyframe ticker does).

→ Next: [STEP-4E-crop-and-registration.md](STEP-4E-crop-and-registration.md)
