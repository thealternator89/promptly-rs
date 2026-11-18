# Promptly (`promptly-rs`)

A native desktop AI prompt builder and modular template library, rebuilt in [**Rust**](https://rust-lang.org/) with [**GPUI Kit**](https://gpui-kit.com/).

`promptly-rs` is a ground-up native Rust port of the original Electron + React + TypeScript [**Promptly**](https://github.com/thealternator89/promptly) desktop application. It preserves 100% file and JSON schema compatibility with the original app while replacing the Chromium/Node.js runtime with a GPU-accelerated, multi-pane desktop interface.

---

## Core Concepts & Features

Promptly solves the problem of assembling structured, repeatable, multi-part prompts for LLMs without manually copying, pasting, and reformatting Markdown boilerplate every time.

### 1. Modular Block-Based Templates
Prompts are composed of an ordered sequence of modular **Parts (Blocks)** across 7 block types:

| Block Type | Description |
| :--- | :--- |
| **`Heading`** | Static Markdown headings (`H1`, `H2`, `H3`) with an optional **"Exclude if next part is empty"** rule that automatically omits the heading if the following block evaluates to empty text. |
| **`Fixed`** | Read-only static boilerplate or system instructions defined by the template author. |
| **`Custom`** | Fillable single-line input or multi-line textarea in the Viewer, configurable with default text and placeholders. |
| **`Quote`** | Fillable multi-line block whose lines are automatically prefixed with `> ` in the compiled Markdown output. |
| **`Code`** | Fillable monospace code editor with a default fence language tag (`rust`, `typescript`, `python`, `csharp`, `sql`, `json`, `bash`, etc.) that can be overridden per session in the Viewer. |
| **`HR`** | Structural horizontal rule divider (`---`). |
| **`Repeatable`** | A nested sub-template container (`templateParts`) that can be instantiated multiple times (`Instance #1`, `Instance #2`, …) during a Viewer session, with isolated inputs per instance. |

### 2. Two-Mode Workflow: Viewer & Editor
- **Prompt Assembly (Viewer & Scratchpad):** Open any saved template—or start in the pinned **Scratchpad**—to fill out dynamic fields, spawn or remove repeatable section instances, drag-and-drop blocks to reorder them for the current session, omit blocks, or append temporary ad-hoc sections (`Custom Input`, `Quote`, `Code Block`, `Rule`) on the fly.
- **Template Authoring (Editor):** Construct or edit reusable templates with drag-and-drop ordering for both top-level blocks and nested repeatable child blocks, metadata fields (`Title` and `Description`), JSON export, and a two-step delete confirmation with a 3-second auto-reset safety timer.
- **Promote Sessions to Templates ("Add to Library"):** Convert a Scratchpad session or a modified template session (with temporary sections) into a permanent library template in one click. Current custom input values are promoted to the new template's default text, and your active session state is transferred seamlessly without losing typed inputs.

### 3. Smart Code Indentation Normalization
Copying a multi-line code snippet starting mid-line in an IDE leaves the first line with `0` leading spaces while subsequent lines retain their surrounding class/function indentation.
- When pasting into a **Code** block, Promptly automatically detects this pattern, computes the minimum shared indentation across subsequent lines, and normalizes the snippet on paste.
- Every Code block provides a one-click **Fix Indentation** / **Reset Indentation** toggle so you can normalize indentation manually or revert to the exact raw clipboard whitespace at any time.

### 4. Two-Pass Markdown Compilation & URL Sanitization
- **Two-Pass Compiler:** Evaluates top-level parts and nested repeatable instances in Pass 1, then evaluates conditional headings (`excludeIfNextEmpty`) in Pass 2 before joining non-empty blocks into clean Markdown.
- **URL Sanitizer (Disallowed Domains):** Prevents accidental leakage of internal hostnames or confidential URLs when pasting into external LLMs. Any domain listed in **Settings → Disallowed Domains** (including its subdomains) is automatically stripped from Markdown links (`[text](https://internal.corp/doc)` $\rightarrow$ `[text]()`) and HTML `href` attributes (`href="https://sub.internal.corp"` $\rightarrow$ `href=""`) during compilation and clipboard copy.

### 5. JSON Import, Export & Legacy Migration
- **JSON Import/Export:** Export any template as pretty-printed JSON or import raw JSON definitions with schema validation, recursive part normalization, missing ID generation, and automatic prompt ID collision resolution.
- **Automatic Electron Data Migration:** On first launch, `promptly-rs` automatically detects the original Electron app's data directory and non-destructively copies `settings.json`, `prompts/order.json`, and all `prompts/*.json` files into its own data directory. Migration can also be re-run manually at any time from **Settings**.

---

## Rebuilding in Rust & Differences from the Original App

While `promptly-rs` maintains full functional parity and data compatibility with the [`promptly`](https://github.com/thealternator89/promptly) project, rebuilding the application in Rust with **GPUI Kit** introduced several architectural and UX improvements:

| Area | Original (`promptly`) | Rust Port (`promptly-rs`) |
| :--- | :--- | :--- |
| **Runtime & UI Framework** | **Electron 40** + Chromium + Node.js with **React 19**, **TypeScript**, and **Bootstrap 5**. | Native **Rust (Edition 2024)** binary powered by **GPUI Kit (`gpui-kit 0.10.0`)** with GPU-accelerated rendering. |
| **Process Architecture** | Split **Main** (`ipcMain`) and **Renderer** (`contextBridge` / `preload.ts`) processes communicating over asynchronous IPC channels. | Single-process native architecture. UI views interact directly with `StorageService` and pure Rust domain services with zero IPC serialization overhead. |
| **Window Layout & Navigation** | Single-pane page router (`React Router 7`) that swaps the entire window between Library, Viewer, Editor, and Settings screens. | Persistent **Multi-Pane Desktop Layout** with a resizable **Library Sidebar**, central **Workspace Pane** (`Viewer` $\leftrightarrow$ `Editor`), and modal overlays. |
| **Live Markdown Preview** | Compiled output is only produced when clicking **Copy Completed Prompt**. | Includes a toggleable, resizable **Live Preview** split pane on the right that renders the compiled and URL-sanitized Markdown in real time as you type. |
| **In-Memory Session Persistence** | Navigating back to the Library unmounts the Viewer component and discards unsaved session inputs. | `AppShell` caches active `Entity<ViewerSession>` instances per prompt ID (and for the Scratchpad), preserving all typed inputs, repeatable instances, and temporary blocks when switching between prompts. |
| **Session Reset** | Requires navigating away and reopening the prompt to clear session edits. | Dedicated **Reset Session** button in the Viewer header restores blocks and inputs to template defaults on demand. |
| **Sidebar Search & Filtering** | Full-page card grid without text filtering. | Built-in live search input in the persistent sidebar to filter saved prompts by title or description. |
| **Component Architecture** | Monolithic switch-case rendering inside `Editor.tsx` and `Viewer.tsx` with `@dnd-kit`. | Modular per-block components (`src/ui/blocks/*.rs`) wrapped in a shared `BlockFrame` with native GPUI drag-and-drop and retained `Entity` control states. |
| **Disk Persistence Safety** | Direct `fs.writeFileSync` in Electron main process handlers. | Atomic writes via temporary files (`.tmp-<uuid>` + rename) and path traversal validation on prompt IDs. |

---

## Tech Stack

- **Language:** Rust (Edition 2024)
- **UI Framework:** [`gpui-kit`](https://gpui-kit.com/) (`0.10.0`) — Bundles GPUI (`gpui-pre`), `gpui-base`, `gpui-component` (styled UI components), and `gpui-kit-assets` (Lucide icons)
- **Serialization:** `serde` & `serde_json` (`camelCase` and tagged union wire compatibility)
- **Text & URL Processing:** `regex` & `url`
- **Platform Integration:** `dirs` (OS config/app-data paths), `uuid` (v4 IDs), and `objc2` / `objc2-app-kit` (native macOS Dock icon integration)

---

## Getting Started

### Prerequisites

- **Rust toolchain** (Rust 1.85+ / Edition 2024 support via [rustup](https://rustup.rs/))
- Platform build dependencies for GPUI (Xcode Command Line Tools on macOS)

### Common Commands

```bash
# Type-check the project
cargo check

# Run the unit and integration test suite
cargo test

# Build and launch the desktop application
cargo run

# Build an optimized release binary
cargo build --release
```

---

## Project Structure

```text
promptly-rs/
├── Cargo.toml                       # Package manifest & dependencies
├── AGENTS.md                        # GPUI Kit conventions & contributor notes
├── assets/                          # Embedded application icons (.icns, .ico)
├── docs/
│   ├── PROJECT_BREAKDOWN.md         # Functional specification & domain algorithms
│   └── RUST_PORT_PLAN.md            # Phased architecture & implementation plan
└── src/
    ├── main.rs                      # Application entry point, window setup & theme init
    ├── app.rs                       # Root AppShell entity, multi-pane layout & global state
    ├── branding.rs                  # Embedded icon assets & macOS Dock icon setup
    ├── model/
    │   ├── prompt.rs                # Prompt, PromptPart enum, Language enum & JSON import validator
    │   ├── session.rs               # ViewerSession state, composite keying & promotion logic
    │   └── settings.rs              # AppSettings model (disallowed domains)
    ├── services/
    │   ├── compiler.rs              # 2-pass Markdown prompt compiler
    │   ├── indentation.rs           # Smart IDE paste dedent & reset logic
    │   ├── sanitizer.rs             # Markdown & HTML URL domain stripper
    │   └── storage.rs               # Atomic disk I/O, order.json sorting & Electron migration
    └── ui/
        ├── sidebar.rs               # Persistent Library sidebar (filter, Scratchpad, drag-sort)
        ├── modals/                  # Import, Export, Add to Library & Settings modal dialogs
        ├── workspace/
        │   ├── viewer.rs            # Session assembly view & temporary section bar
        │   ├── editor.rs            # Template builder view & 2-step delete confirmation
        │   └── preview.rs           # Resizable right-hand Live Markdown Preview pane
        └── blocks/                  # Shared BlockFrame & modular Editor/Viewer block components
```

---

## Data Storage & Compatibility

`promptly-rs` stores its data in a dedicated `Promptly-RS` directory inside your OS configuration/application support folder so it can safely coexist alongside the original Electron app:

- **macOS:** `~/Library/Application Support/Promptly-RS/`
- **Windows:** `%APPDATA%\Promptly-RS\`
- **Linux:** `~/.config/Promptly-RS/`

### Directory Contents
- `prompts/<id>.json` — Individual prompt templates (identical JSON schema to the Electron app).
- `prompts/order.json` — Array of prompt IDs defining custom sidebar ordering (unordered prompts fall back to `createdAt` descending).
- `settings.json` — Global settings (`{ "disallowedDomains": "..." }`).
- `.migrated` — Marker file indicating initial check/migration from the legacy Electron `Promptly` directory has completed.

---

## Additional Documentation

- [Project Breakdown & Functional Specification](./docs/PROJECT_BREAKDOWN.md)
- [Phased Rust + GPUI-Kit Port Plan](./docs/RUST_PORT_PLAN.md)
- [Original Electron App - `promptly`](https://github.com/thealternator89/promptly)

## License

MIT
