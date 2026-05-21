# CLAUDE.md

**anywhere** — browser extension. Authors: **Rust + `.crepus` only**. No hand-written JS in this repo.

## Scope

- `runtime/src/lib.rs` — WASM: parse widgets, render shells/iframe, popup routes
- `views/popup.crepus` — popup (main / help / crepus ref)
- `views/ui.crepus` — `Panel`, `AnywhereShell`, etc.
- `webext.toml` — manifest + capabilities

## Dependencies

`../crepuscularity/crates/`. Side-by-side checkout for path deps.

## Build

```bash
crepus webext build
```

→ `dist/unpacked/`. Install CLI first:

```bash
cargo install --path ../crepuscularity/crates/crepuscularity-cli
```

Reinstall CLI with `--force` after framework asset changes, then rebuild. Reload extension in Chrome.

## Rules

- Logic: Rust + `.crepus` only.
- JS surface: `#[wasm_bindgen]` exports only.
- No edit `dist/unpacked/` by hand — build copies framework assets.
- Bootstrap DOM: `crepuscularity-webext/assets/content.js` (scan `<pre>`, call WASM). `content_main()` in Rust is stub for now.

## Content WASM API (called from framework `content.js`)

- `extract_widgets` / `extract_specs` — parse message text
- `render_frontend` — shell HTML from `views/ui.crepus`
- `render_anywhere_frame_doc` / `render_frame_doc` — iframe `srcdoc`
- `handle_popup_action` / `render_popup` — popup storage routes

## Pre-rendered popup

`crepus webext build` → `prerender_popup_html` → 3 static divs:

| View div id   | Rendered with               |
|---------------|-----------------------------|
| `view-main`   | `show_help=false, show_crepus=false` |
| `view-help`   | `show_help=true,  show_crepus=false` |
| `view-crepus` | `show_help=false, show_crepus=true`  |

`popup.js` toggles views on `data-action`. No WASM in popup.

`{system_prompt}` from `SYSTEM_PROMPT` in `crepuscularity-cli/src/webext.rs`.

## handle_popup_action

| action           | storage_op                                    |
|------------------|-----------------------------------------------|
| `set-enabled`    | `set` → `enabled` (bool, sync)               |
| `set-auto-render`| `set` → `autoRender` (bool, sync)            |
| `show-help`      | `set` → `showHelp: true` (local)             |
| `hide-help`      | `set` → `showHelp: false` (local)            |
| `show-crepus`    | `set` → `showCrepus: true` (local)           |
| `hide-crepus`    | `set` → `showCrepus: false` (local)          |

`copy-prompt`: `popup.js` Clipboard only.

Storage ops: `push`, `remove`, `set`.
