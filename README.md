# anywhere

Browser extension that finds `<ai-anywhere>` blocks in AI assistant replies (ChatGPT, Claude, Gemini, etc.) and renders them as interactive panels — charts, forms, trackers, and more.

Built with [Crepuscularity](https://github.com/semitechnological/crepuscularity): Rust + `.crepus` templates, Manifest V3, minimal hand-written JS (framework bootstrap only).

## Install

Load `dist/unpacked/` as an unpacked extension in `chrome://extensions` (Developer mode).

## Build

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
cargo install --path ../crepuscularity/crates/crepuscularity-cli --force
```

Check out `anywhere/` and `crepuscularity/` side by side for path dependencies.

```bash
crepus webext build
# → dist/unpacked/
```

Reinstall the CLI with `--force` after changing framework assets in `crepuscularity`, then rebuild and reload the extension.

## Layout

| Path | Role |
| --- | --- |
| `crepus.toml` | Extension metadata, capabilities, content scripts (icons/pages auto-detected at build) |
| `runtime/src/lib.rs` | WASM: parse widgets, render shells/iframe, popup storage routes |
| `views/popup.crepus` | Popup (main / help / crepus reference) — pre-rendered at build |
| `views/ui.crepus` | In-page shells (`AnywhereShell`, `Panel`, …) |
| `resources/system_prompt.txt` | AI system-prompt snippet (injected into popup Help) |
| `src/content.css.crepus` | Extra content-script styles (appended to framework CSS) |
| `icons/` | Toolbar + extension icons (`icon16.png`, …) |
| `dist/unpacked/` | Built extension (gitignored) |

## AI prompt snippet

Use the popup **Help** view (copy button) or this shape:

```html
<ai-anywhere type="widget" title="Widget Title">
  <anywhere-ui lang="crepus">
    div app
      h1 heading
        "{title}"
      for item in {items}
        div row
          "{item.name}"
  </anywhere-ui>
  <anywhere-data>{"title":"Example","items":[{"name":"A"}]}</anywhere-data>
</ai-anywhere>
```

Crepus only — no `<anywhere-script>`. Precompute counts in JSON; put HTML attributes on the same line as the tag.

## Architecture

- Widget logic: Rust (`extract_widgets`, `render_anywhere_frame_doc`, …) + `.crepus` UI.
- DOM scan / code-block replace: framework `crepuscularity-webext/assets/content.js` (one `<pre>` → one panel).
- Popup: static HTML from `views/popup.crepus`; settings via `background.js` + `chrome.storage.sync`.
