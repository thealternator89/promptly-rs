# AGENTS.md

## Project Overview

`promptly-rs` is a Rust desktop application (edition 2024) built with **GPUI Kit** (`gpui-kit`).

- **Cargo manifest**: `Cargo.toml`
- **Entry point**: `src/main.rs`
- **Documentation**: https://gpui-kit.com/docs/getting-started/

## UI Library: GPUI Kit (`gpui-kit`)

We use the `gpui-kit` umbrella crate (`0.10.0`), which bundles:

- **GPUI** (`gpui-pre` / `gpui-pre-platform`, including `font-kit` enabled for macOS text rendering)
- **GPUI Base** (`gpui-base`)
- **GPUI Component** (`gpui-component` — styled UI components)
- **Default Assets** (`gpui-kit-assets` — default icon assets)

> **Important**: Do not add a separate direct `gpui` or `font-kit` dependency alongside `gpui-kit`. `gpui-kit` manages the pinned GPUI snapshot and platform features.

### Imports

- Core GPUI types, layout primitives (`div()`), `Render`, `Window`, `Context`, `application()`, `init()`, `open_window()`, `assets`:
  ```rust
  use gpui_kit::*;
  ```
- Styled components (buttons, labels, etc.):
  ```rust
  use gpui_kit::component::{
      button::{Button, ButtonVariants},
      label::Label,
  };
  ```

### Application Startup Sequence

1. **Create the application & register assets**: `application().with_assets(assets::Assets).run(|cx| { ... })`
2. **Initialize Kit layers & themes**: Call `init(cx)` once inside `.run(...)` before opening windows or constructing components.
3. **Open a window**: Call `open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| YourView))`. `open_window` automatically wraps your view in a `Root` (which owns overlay layers such as dialogs, sheets, and notifications), so return your content view directly—do not wrap it in another `Root`.

### Architecture & Conventions

- **`Entity<T>`**: Holds retained state across frames.
- **Persistent Views (`Render`)**: Structs implementing `Render` mounted as an `Entity<View>` rebuild their element tree on each render via `render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement`.
- **Stateless/Reusable Components (`RenderOnce`)**: Components constructed by value each render.
- **Retained Control State**: When a control needs lasting state (such as text input state, subscriptions, or tasks), store an `Entity` on the owning view rather than recreating it inside `render`.
- **Accessibility**: Prefer `Label::new("...")` over bare string children (`.child("...")`) for displayed text so screen readers and assistive technologies can see it in the accessibility tree.

## Common Commands

```bash
cargo check   # Type-check the project
cargo build   # Compile and link the desktop binary
cargo run     # Build and launch the application window
```
