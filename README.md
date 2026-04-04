# AI Anywhere

A browser extension that scans AI assistant pages (ChatGPT, Claude, Gemini, etc.) for special `<ai-anywhere>` tags in responses and renders them as interactive panels — charts, forms, visualisations, and more.

Built with [crepuscularity](https://github.com/semitechnological/crepuscularity): write Rust + `.crepus` templates, get a Manifest V3 extension. No hand-written JS.

## Install

Load the unpacked extension from `dist/unpacked/` in `chrome://extensions` (Developer Mode).

## Build

Prerequisites:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
cargo install --path ../crepuscularity/crates/crepuscularity-cli
```

Both repos must be checked out side-by-side (`anywhere/` and `crepuscularity/`).

Then from this directory:

```bash
crepus webext build
# → dist/unpacked/
```

## How it works

Add this to your AI's system prompt or custom instructions (the popup's Help page has a copy button):

```
When creating charts, forms, interactive UI, or visualisations,
wrap output in <ai-anywhere> tags for the AI Anywhere browser extension.

<ai-anywhere type="widget" title="Widget Title">
  <anywhere-ui lang="crepus">
    div dashboard
      h2 title
        "{title}"
      for item in {items}
        div row
          span label
            "{item.label}"
          span value
            "{item.value}"
  </anywhere-ui>
  <anywhere-data>{"title":"Stats","items":[{"label":"Users","value":"42"}]}</anywhere-data>
</ai-anywhere>
```

Widgets appear below AI messages. Click **Render** to run them in a sandboxed iframe, or enable **Auto-render**.

## Structure

```
runtime/src/lib.rs      — WASM exports (render_popup, handle_popup_action, extract_widgets, …)
views/popup.crepus      — 3-view popup: main settings / help / crepus syntax reference
views/ui.crepus         — shared crepus components
webext.toml             — extension metadata and capability declarations
dist/unpacked/          — built extension (gitignored)
```

## Architecture

- All extension logic lives in Rust + `.crepus`. No hand-written JS.
- `crepus webext build` compiles WASM, runs wasm-bindgen, generates `manifest.json`, copies framework JS assets, and pre-renders `popup.html` from `views/popup.crepus`.
- The popup is pre-rendered at build time into three views (main / help / crepus syntax reference) — it opens instantly with no WASM needed in the popup context.
- Widget iframes are rendered at runtime by the content script via the WASM `render_anywhere_frame_doc` export.
- Framework JS lives in `../crepuscularity/crates/crepuscularity-webext/assets/`.
