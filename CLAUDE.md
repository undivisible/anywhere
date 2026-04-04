# CLAUDE.md

App authors write **Rust + `.crepus` only**. No hand-written JS.

## Scope

Single extension at the repo root:
- `runtime/src/lib.rs` — all WASM logic
- `views/popup.crepus` — 3-view popup (main / help / crepus syntax reference)
- `views/ui.crepus` — shared crepus components
- `webext.toml` — extension metadata and capabilities

## Dependencies

Framework crates live in `../crepuscularity/crates/`. Both repos must be checked out
side-by-side for Cargo path deps to resolve.

## Build

```bash
crepus webext build
```

Produces `dist/unpacked/`. Install the CLI first:

```bash
cargo install --path ../crepuscularity/crates/crepuscularity-cli
```

## Rules

- All extension logic stays in Rust + `.crepus`.
- The JS-visible API surface is only `#[wasm_bindgen]`-exported functions.
- Do not add handwritten JavaScript inside this repo.
- Framework JS assets come from `../crepuscularity/crates/crepuscularity-webext/assets/`.
  Update the framework, not this repo, if browser bootstrap needs to change.

## Pre-rendered popup protocol

`crepus webext build` calls `prerender_popup_html` in the CLI, which renders
`views/popup.crepus` into three static HTML views at build time:

| View div id   | Rendered with               |
|---------------|-----------------------------|
| `view-main`   | `show_help=false, show_crepus=false` |
| `view-help`   | `show_help=true,  show_crepus=false` |
| `view-crepus` | `show_help=false, show_crepus=true`  |

`popup.js` (framework) shows/hides the correct div on `data-action` clicks.
No WASM is loaded in the popup context — it opens instantly from static HTML.

The `{system_prompt}` context variable is injected by the CLI from the
`SYSTEM_PROMPT` const in `crepuscularity-cli/src/webext.rs`.

## handle_popup_action routes

| action           | storage_op                                    |
|------------------|-----------------------------------------------|
| `set-enabled`    | `set` → `enabled` (bool, sync)               |
| `set-auto-render`| `set` → `autoRender` (bool, sync)            |
| `show-help`      | `set` → `showHelp: true` (local)             |
| `hide-help`      | `set` → `showHelp: false` (local)            |
| `show-crepus`    | `set` → `showCrepus: true` (local)           |
| `hide-crepus`    | `set` → `showCrepus: false` (local)          |

`copy-prompt` is handled entirely in `popup.js` via the Clipboard API (no WASM round-trip).

## Supported storage_op types

`push`, `remove`, `set` — applied by framework `popup.js`.
