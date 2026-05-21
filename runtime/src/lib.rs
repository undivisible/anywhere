use std::collections::{BTreeMap, HashMap};

use crepuscularity_core::context::TemplateContext;
use crepuscularity_web::render_from_files;
use crepuscularity_webext::{
    build_frame_doc, json_to_template, BrowserProgram, JsExpr, MessagePayload, StorageArea,
};
use js_sys::{Object, Reflect};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

fn wasm_object(pairs: &[(&str, JsValue)]) -> Result<JsValue, JsValue> {
    let obj = Object::new();
    for (key, value) in pairs {
        Reflect::set(&obj, &JsValue::from_str(key), value)
            .map_err(|_| JsValue::from_str("failed to build wasm response object"))?;
    }
    Ok(obj.into())
}

fn set_opt_str(obj: &Object, key: &str, value: &Option<String>) -> Result<(), JsValue> {
    if let Some(v) = value {
        Reflect::set(obj, &JsValue::from_str(key), &JsValue::from_str(v))
            .map_err(|_| JsValue::from_str("failed to set wasm object field"))?;
    }
    Ok(())
}

fn ui_lang_str(lang: UiLang) -> &'static str {
    match lang {
        UiLang::Crepus => "crepus",
        UiLang::Html => "html",
        UiLang::Builtin => "builtin",
    }
}

fn script_lang_str(lang: ScriptLang) -> &'static str {
    match lang {
        ScriptLang::Js => "js",
        ScriptLang::Mermaid => "mermaid",
        ScriptLang::Latex => "latex",
        ScriptLang::Other => "other",
    }
}

fn widget_to_js(w: &AnywhereWidget) -> Result<JsValue, JsValue> {
    let obj = Object::new();
    set_opt_str(&obj, "id", &w.id)?;
    set_opt_str(&obj, "widget_type", &w.widget_type)?;
    set_opt_str(&obj, "title", &w.title)?;
    set_opt_str(&obj, "data", &w.data)?;
    if let Some(ui) = &w.ui {
        let ui_obj = Object::new();
        Reflect::set(
            &ui_obj,
            &JsValue::from_str("lang"),
            &JsValue::from_str(ui_lang_str(ui.lang.clone())),
        )
        .map_err(|_| JsValue::from_str("failed to set ui.lang"))?;
        Reflect::set(
            &ui_obj,
            &JsValue::from_str("source"),
            &JsValue::from_str(&ui.source),
        )
        .map_err(|_| JsValue::from_str("failed to set ui.source"))?;
        Reflect::set(&obj, &JsValue::from_str("ui"), &ui_obj.into())
            .map_err(|_| JsValue::from_str("failed to set ui"))?;
    }
    if let Some(script) = &w.script {
        let script_obj = Object::new();
        Reflect::set(
            &script_obj,
            &JsValue::from_str("lang"),
            &JsValue::from_str(script_lang_str(script.lang.clone())),
        )
        .map_err(|_| JsValue::from_str("failed to set script.lang"))?;
        Reflect::set(
            &script_obj,
            &JsValue::from_str("source"),
            &JsValue::from_str(&script.source),
        )
        .map_err(|_| JsValue::from_str("failed to set script.source"))?;
        Reflect::set(&obj, &JsValue::from_str("script"), &script_obj.into())
            .map_err(|_| JsValue::from_str("failed to set script"))?;
    }
    Ok(obj.into())
}

fn widgets_to_js(widgets: &[AnywhereWidget]) -> Result<JsValue, JsValue> {
    let arr = js_sys::Array::new();
    for w in widgets {
        arr.push(&widget_to_js(w)?);
    }
    Ok(arr.into())
}

// ---------------------------------------------------------------------------
// App-specific embedded assets (compiled into the WASM binary)
// ---------------------------------------------------------------------------

const POPUP_VIEW: &str = include_str!("../../views/popup.crepus");
const UI_VIEW: &str = include_str!("../../views/ui.crepus");

const POPUP_CSS: &str = r#"
*,*::before,*::after{box-sizing:border-box}
body{margin:0;min-width:300px;font-family:system-ui,"Segoe UI",sans-serif;background:#0f0f17;color:#e0e0f0}
.popup{display:flex;flex-direction:column;padding:14px 16px 16px;gap:10px}
.popup--help{padding:14px 16px 20px;gap:8px}
.popup__eyebrow{font-size:10px;font-weight:700;text-transform:uppercase;letter-spacing:.12em;color:#5a5a8a}
.popup__header{display:flex;align-items:center;justify-content:space-between}
.popup__title{margin:0;font-size:15px;font-weight:700;color:#e8e8ff}
.popup__copy{margin:0;font-size:12px;line-height:1.5;color:#8888aa}
.popup__toggle{display:flex;align-items:center;gap:8px;cursor:pointer;font-size:13px;color:#c0c0e0}
.popup__toggle input{accent-color:#7b7bde;width:15px;height:15px;cursor:pointer}
.popup__help-btn{background:none;border:1px solid #333355;border-radius:50%;width:22px;height:22px;color:#7777aa;font-size:13px;font-weight:700;cursor:pointer;line-height:1;padding:0;display:flex;align-items:center;justify-content:center;flex-shrink:0}
.popup__help-btn:hover{background:#1a1a2e;color:#9999cc}
.popup__nav{margin-bottom:2px}
.popup__back{background:none;border:none;color:#7b7bde;font-size:12px;cursor:pointer;padding:0;font-weight:600}
.popup__back:hover{color:#9999ff}
.popup__help-section{display:flex;flex-direction:column;gap:4px}
.popup__help-heading{margin:0;font-size:11px;font-weight:700;text-transform:uppercase;letter-spacing:.08em;color:#7b7bde}
.popup__prompt{margin:0;padding:10px 12px;background:#0a0a14;border:1px solid #222240;border-radius:6px;font-size:10.5px;line-height:1.55;color:#a0a0c8;white-space:pre-wrap;word-break:break-all;font-family:"IBM Plex Mono",ui-monospace,monospace;overflow-y:auto;max-height:320px}
.popup--help{overflow-y:auto;max-height:520px}
.popup__crepus-btn{width:100%;padding:8px 12px;background:#1a1a30;border:1px solid #333360;border-radius:7px;color:#9090dd;font-size:12px;font-weight:600;cursor:pointer;text-align:left}
.popup__crepus-btn:hover{background:#20203a;color:#b0b0ff}
"#;

/// All `.crepus` views embedded at compile time — no filesystem access at runtime.
fn embedded_files() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("views/ui.crepus".to_string(), UI_VIEW.to_string());
    map.insert("views/popup.crepus".to_string(), POPUP_VIEW.to_string());
    map
}

// ---------------------------------------------------------------------------
// Anywhere widget types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WidgetSpec {
    pub id: String,
    pub title: String,
    pub html: String,
    pub css: String,
    pub js: String,
    pub source: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UiLang {
    Crepus,
    Html,
    Builtin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScriptLang {
    Js,
    Mermaid,
    Latex,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnywhereUi {
    pub lang: UiLang,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnywhereScript {
    pub lang: ScriptLang,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnywhereWidget {
    pub id: Option<String>,
    pub widget_type: Option<String>,
    pub title: Option<String>,
    pub ui: Option<AnywhereUi>,
    pub script: Option<AnywhereScript>,
    pub data: Option<String>,
}

// ---------------------------------------------------------------------------
// WASM input types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct RenderRequest {
    entry: String,
    #[serde(default)] files: HashMap<String, String>,
    #[serde(default)] props: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
struct FrameDocRequest {
    #[serde(default)] html: String,
    #[serde(default)] css: String,
    #[serde(default)] js: String,
    #[serde(default)] unocss: String,
}

#[derive(Deserialize)]
struct AnywhereFrameDocRequest {
    widget: AnywhereWidget,
    #[serde(default)] unocss: String,
}

// ---------------------------------------------------------------------------
// Anywhere widget parsing — <ai-anywhere> tags
// ---------------------------------------------------------------------------

fn parse_anywhere_tags(text: &str) -> Vec<AnywhereWidget> {
    let mut widgets = Vec::new();
    let mut pos = 0;
    while pos < text.len() {
        let remaining = &text[pos..];
        let Some(rel_start) = remaining.find("<ai-anywhere") else { break };
        let abs_start = pos + rel_start;
        let after_open = &text[abs_start + "<ai-anywhere".len()..];
        let Some(tag_close) = find_tag_close(after_open) else { pos = abs_start + 1; continue };
        let attrs = &after_open[..tag_close];
        let id = attr_value(attrs, "id");
        let widget_type = attr_value(attrs, "type");
        let title = attr_value(attrs, "title");
        let inner_start = abs_start + "<ai-anywhere".len() + tag_close + 1;
        let Some(inner_len) = find_close_tag(&text[inner_start..], "ai-anywhere") else {
            pos = abs_start + 1; continue
        };
        let inner = &text[inner_start..inner_start + inner_len];
        let ui = extract_section(inner, "anywhere-ui").map(|(a, c)| AnywhereUi {
            lang: match attr_value(&a, "lang").as_deref() {
                Some("html") => UiLang::Html,
                _ => UiLang::Crepus,
            },
            source: c,
        });
        let script = extract_section(inner, "anywhere-script").map(|(a, c)| AnywhereScript {
            lang: match attr_value(&a, "lang").as_deref() {
                Some("mermaid") => ScriptLang::Mermaid,
                Some("latex" | "katex" | "tex") => ScriptLang::Latex,
                _ => ScriptLang::Js,
            },
            source: c,
        });
        let data = extract_section(inner, "anywhere-data").map(|(_, c)| c);
        widgets.push(AnywhereWidget { id, widget_type, title, ui, script, data });
        pos = inner_start + inner_len + "</ai-anywhere>".len();
    }
    widgets
}

fn find_tag_close(s: &str) -> Option<usize> {
    let mut in_quote: Option<char> = None;
    for (i, c) in s.char_indices() {
        match (in_quote, c) {
            (None, '"' | '\'') => in_quote = Some(c),
            (Some(q), c) if c == q => in_quote = None,
            (None, '>') => return Some(i),
            _ => {}
        }
    }
    None
}

fn find_close_tag(s: &str, tag: &str) -> Option<usize> {
    let open_pat = format!("<{tag}");
    let close_pat = format!("</{tag}>");
    let mut depth = 1usize;
    let mut pos = 0;
    while pos < s.len() {
        let rest = &s[pos..];
        match (rest.find(&open_pat), rest.find(&close_pat)) {
            (None, None) => break,
            (None, Some(c)) => {
                depth -= 1;
                if depth == 0 { return Some(pos + c); }
                pos += c + close_pat.len();
            }
            (Some(o), None) => { depth += 1; pos += o + open_pat.len(); }
            (Some(o), Some(c)) => {
                if o < c { depth += 1; pos += o + open_pat.len(); }
                else {
                    depth -= 1;
                    if depth == 0 { return Some(pos + c); }
                    pos += c + close_pat.len();
                }
            }
        }
    }
    None
}

fn extract_section(s: &str, tag: &str) -> Option<(String, String)> {
    let open_pat = format!("<{tag}");
    let start = s.find(&open_pat)?;
    let after = &s[start + open_pat.len()..];
    let tag_end = find_tag_close(after)?;
    let attrs = after[..tag_end].trim().to_string();
    let content_start = start + open_pat.len() + tag_end + 1;
    let close_pat = format!("</{tag}>");
    let content_end = s[content_start..].find(&close_pat)?;
    Some((attrs, s[content_start..content_start + content_end].trim().to_string()))
}

fn attr_value(attrs: &str, name: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let pat = format!("{name}={quote}");
        if let Some(pos) = attrs.find(&pat) {
            let after = &attrs[pos + pat.len()..];
            if let Some(end) = after.find(quote) {
                return Some(after[..end].to_string());
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Anywhere widget parsing — legacy code-block format
// ---------------------------------------------------------------------------

fn extract_widget_specs(message: &str) -> Vec<WidgetSpec> {
    let fences = parse_fences(message);
    let mut specs = extract_json_specs(&fences);
    specs.extend(extract_triplet_specs(&fences));
    specs
}

struct Fence { lang: String, body: String }

fn parse_fences(message: &str) -> Vec<Fence> {
    let mut fences = Vec::new();
    let mut lines = message.lines();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if !trimmed.starts_with("```") { continue; }
        let lang = trimmed.trim_start_matches("```").trim().to_lowercase();
        let mut body = Vec::new();
        for inner in lines.by_ref() {
            if inner.trim() == "```" { break; }
            body.push(inner);
        }
        fences.push(Fence { lang, body: body.join("\n") });
    }
    fences
}

fn extract_json_specs(fences: &[Fence]) -> Vec<WidgetSpec> {
    fences.iter().enumerate().filter_map(|(idx, f)| {
        if !matches!(f.lang.as_str(), "aiwidget" | "widget" | "widget-json") { return None; }
        #[derive(Deserialize)]
        struct Input {
            #[serde(default)] id: Option<String>,
            #[serde(default)] title: Option<String>,
            #[serde(default)] html: Option<String>,
            #[serde(default)] css: Option<String>,
            #[serde(default)] js: Option<String>,
        }
        let input: Input = serde_json::from_str(&f.body).ok()?;
        Some(WidgetSpec {
            id: input.id.unwrap_or_else(|| format!("widget-json-{idx}")),
            title: input.title.unwrap_or_else(|| format!("Widget {}", idx + 1)),
            html: input.html.unwrap_or_default(),
            css: input.css.unwrap_or_default(),
            js: input.js.unwrap_or_default(),
            source: format!("json:{idx}"),
            format: "json".to_string(),
        })
    }).collect()
}

fn extract_triplet_specs(fences: &[Fence]) -> Vec<WidgetSpec> {
    let mut out = Vec::new();
    let mut pending = WidgetSpec::default();
    let mut found = false;
    for (idx, f) in fences.iter().enumerate() {
        match f.lang.as_str() {
            "widget-html" | "aiwidget-html" => {
                if found && !pending.html.is_empty() {
                    out.push(finalize(&pending, out.len()));
                    pending = WidgetSpec::default();
                }
                pending.html = f.body.clone();
                pending.source = format!("fence:{idx}");
                pending.format = "triplet".to_string();
                found = true;
            }
            "widget-css" | "aiwidget-css" => { pending.css = f.body.clone(); found = true; }
            "widget-js" | "aiwidget-js" | "javascript" | "js" => {
                if found && pending.js.is_empty() && !pending.html.is_empty() {
                    pending.js = f.body.clone();
                }
            }
            _ => {}
        }
    }
    if found && !pending.html.is_empty() { out.push(finalize(&pending, out.len())); }
    out
}

fn finalize(spec: &WidgetSpec, idx: usize) -> WidgetSpec {
    let mut s = spec.clone();
    if s.id.is_empty() { s.id = format!("widget-triplet-{idx}"); }
    if s.title.is_empty() { s.title = format!("Widget {}", idx + 1); }
    s
}

// ---------------------------------------------------------------------------
// Anywhere widget frame doc
// ---------------------------------------------------------------------------

const WIDGET_FRAME_CSS: &str = r#"
.app { max-width: 52rem; }
.heading { font-size: 1.35rem; font-weight: 700; margin: 0 0 0.5rem; }
.stats { margin-bottom: 0.75rem; opacity: 0.85; }
.card { border: 1px solid #e5e7eb; border-radius: 12px; padding: 12px 14px; margin: 10px 0; }
.row { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
.name { font-weight: 600; }
.badge { font-size: 11px; font-weight: 700; padding: 3px 8px; border-radius: 999px; }
.badge.green { background: #dcfce7; color: #166534; }
.badge.yellow { background: #fef9c3; color: #854d0e; }
button.primary, .primary { display: inline-block; margin-top: 8px; padding: 8px 14px; border-radius: 999px; border: 2px solid #174c63; background: #174c63; color: #fff; font-size: 13px; font-weight: 600; cursor: pointer; }
.notice, .success { margin-top: 12px; padding: 10px 12px; border-radius: 10px; font-size: 13px; }
.notice { background: #f3f4f6; }
.success { background: #dcfce7; color: #166534; }
"#;

fn enrich_widget_context(ctx: &mut TemplateContext, data: &Value) {
    if let Some(workouts) = data.get("workouts").and_then(|w| w.as_array()) {
        let completed = workouts
            .iter()
            .filter(|w| w.get("done").and_then(|d| d.as_bool()) == Some(true))
            .count();
        ctx.set("completed", completed as i64);
        ctx.set("total", workouts.len() as i64);
    }
}

fn anywhere_frame_doc_parts(widget: &AnywhereWidget) -> (String, String, String) {
    let data: Value = widget.data
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_else(|| json!({}));

    let mut html = String::new();
    let css = WIDGET_FRAME_CSS.to_string();
    let mut js = String::new();

    if let Some(ui) = &widget.ui {
        match ui.lang {
            UiLang::Html => { html = ui.source.clone(); }
            UiLang::Crepus => {
                let mut files = HashMap::new();
                files.insert("__ai_widget__".to_string(), ui.source.clone());
                let mut ctx = TemplateContext::new();
                if let Value::Object(obj) = &data {
                    for (k, v) in obj {
                        ctx.set(k, json_to_template(v.clone()));
                    }
                }
                enrich_widget_context(&mut ctx, &data);
                match render_from_files(&files, "__ai_widget__", &ctx) {
                    Ok(h) => html = h,
                    Err(e) => {
                        html = format!("<pre style=\"color:red\">Crepus render error: {e}</pre>");
                    }
                }
            }
            UiLang::Builtin => {}
        }
    }

    if let Some(script) = &widget.script {
        match script.lang {
            ScriptLang::Mermaid => {
                html = format!("<div class=\"mermaid\">{}</div>", script.source);
                js = "import('https://cdn.jsdelivr.net/npm/mermaid/dist/mermaid.esm.min.mjs').then(m=>m.default.initialize({startOnLoad:true}));".to_string();
            }
            ScriptLang::Latex => {
                html = format!("<div class=\"latex\">{}</div>", script.source);
                js = "import('https://cdn.jsdelivr.net/npm/katex/dist/katex.mjs').then(m=>{document.querySelectorAll('.latex').forEach(el=>m.default.render(el.textContent,el,{throwOnError:false}));});".to_string();
            }
            ScriptLang::Js | ScriptLang::Other => {
                // Widgets are crepus-only; user JS blocks are ignored at runtime.
            }
        }
    }

    (html, css, js)
}

// ---------------------------------------------------------------------------
// WASM exports
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn runtime_version() -> String {
    format!("{}+plain-js", env!("CARGO_PKG_VERSION"))
}

/// Render a `.crepus` template. Views in `views/` are embedded in the WASM
/// binary. The optional `files` map can supply additional or override templates.
/// Input: `{ entry: "path#Component", props: {...}, files?: {path: content} }`
/// Returns: `{ html: string }`
#[wasm_bindgen]
pub fn render_frontend(request: JsValue) -> Result<JsValue, JsValue> {
    let req: RenderRequest = serde_wasm_bindgen::from_value(request)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut ctx = TemplateContext::new();
    for (k, v) in req.props {
        ctx.set(k, json_to_template(v));
    }
    let mut files = embedded_files();
    files.extend(req.files);
    let html = render_from_files(&files, &req.entry, &ctx)
        .map_err(|e| JsValue::from_str(&e))?;
    wasm_object(&[("html", JsValue::from_str(&html))])
}

/// Build the srcdoc for a widget iframe.
/// Input: `{ html, css, js, unocss? }`  Returns: `{ srcdoc: string }`
#[wasm_bindgen]
pub fn render_frame_doc(request: JsValue) -> Result<JsValue, JsValue> {
    let req: FrameDocRequest = serde_wasm_bindgen::from_value(request)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let srcdoc = build_frame_doc(
        &req.html, &req.css, &req.js, &req.unocss,
        "<p>Widget had no HTML payload.</p>",
    );
    wasm_object(&[("srcdoc", JsValue::from_str(&srcdoc))])
}

/// Render widget body parts for inline (shadow DOM) or iframe mounting.
/// Input: `{ widget, unocss? }`. Returns: `{ html, css, js, needs_iframe }`.
#[wasm_bindgen]
pub fn render_anywhere_parts(request: JsValue) -> Result<JsValue, JsValue> {
    let req: AnywhereFrameDocRequest = serde_wasm_bindgen::from_value(request)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let (html, css, js) = anywhere_frame_doc_parts(&req.widget);
    let needs_iframe = !js.is_empty();
    wasm_object(&[
        ("html", JsValue::from_str(&html)),
        ("css", JsValue::from_str(&css)),
        ("js", JsValue::from_str(&js)),
        ("needs_iframe", JsValue::from_bool(needs_iframe)),
    ])
}

/// Build the srcdoc for an `<ai-anywhere>` widget iframe.
/// Input: widget object + optional `unocss` field.  Returns: `{ srcdoc: string }`
#[wasm_bindgen]
pub fn render_anywhere_frame_doc(request: JsValue) -> Result<JsValue, JsValue> {
    let req: AnywhereFrameDocRequest = serde_wasm_bindgen::from_value(request)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let (html, css, js) = anywhere_frame_doc_parts(&req.widget);
    let srcdoc = build_frame_doc(&html, &css, &js, &req.unocss, "<p>Widget had no content.</p>");
    wasm_object(&[("srcdoc", JsValue::from_str(&srcdoc))])
}

/// Parse `<ai-anywhere>` widgets from an AI message.
#[wasm_bindgen]
pub fn extract_widgets(message: &str) -> Result<JsValue, JsValue> {
    let widgets = parse_anywhere_tags(message);
    widgets_to_js(&widgets)
}

/// Parse legacy ``` code-block widget specs from an AI message.
#[wasm_bindgen]
pub fn extract_specs(message: &str) -> Result<JsValue, JsValue> {
    let specs: Vec<WidgetSpec> = extract_widget_specs(message);
    serde_wasm_bindgen::to_value(&specs)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Render the settings popup from storage state.
/// Input: `{ enabled: bool, autoRender: bool }`  Returns: `{ html: string, css: string }`
#[wasm_bindgen]
pub fn render_popup(state: JsValue) -> Result<JsValue, JsValue> {
    let state: Value = serde_wasm_bindgen::from_value(state)
        .unwrap_or_else(|_| json!({}));
    let enabled = state.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
    let auto_render = state.get("autoRender").and_then(|v| v.as_bool()).unwrap_or(false);
    let show_help = state.get("showHelp").and_then(|v| v.as_bool()).unwrap_or(false);
    let show_crepus = state.get("showCrepus").and_then(|v| v.as_bool()).unwrap_or(false);
    let mut ctx = TemplateContext::new();
    ctx.set("enabled", enabled);
    ctx.set("auto_render", auto_render);
    ctx.set("show_help", show_help);
    ctx.set("show_crepus", show_crepus);
    let files = embedded_files();
    let html = render_from_files(&files, "views/popup.crepus", &ctx)
        .map_err(|e| JsValue::from_str(&e))?;
    wasm_object(&[
        ("html", JsValue::from_str(&html)),
        ("css", JsValue::from_str(POPUP_CSS)),
    ])
}

/// Route a popup action.  Returns `{ storage_op? }`.
#[wasm_bindgen]
pub fn handle_popup_action(action: &str, data: JsValue) -> Result<JsValue, JsValue> {
    let data: Value = serde_wasm_bindgen::from_value(data)
        .unwrap_or_else(|_| json!({}));
    let checked = data.get("checked").and_then(|v| v.as_str()) == Some("true");
    let storage_op = match action {
        "set-enabled"     => Some(json!({ "type": "set", "key": "enabled",    "value": checked, "area": "sync" })),
        "set-auto-render" => Some(json!({ "type": "set", "key": "autoRender", "value": checked, "area": "sync" })),
        "show-help"       => Some(json!({ "type": "set", "key": "showHelp",   "value": true  })),
        "hide-help"       => Some(json!({ "type": "set", "key": "showHelp",   "value": false })),
        "show-crepus"     => Some(json!({ "type": "set", "key": "showCrepus", "value": true  })),
        "hide-crepus"     => Some(json!({ "type": "set", "key": "showCrepus", "value": false })),
        _ => None,
    };
    serde_wasm_bindgen::to_value(&match storage_op {
        Some(op) => json!({ "storage_op": op }),
        None => json!({}),
    })
    .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Entry point for popup UI initialization.
/// The popup is pre-rendered at build time, so this is minimal.
#[wasm_bindgen]
pub fn popup_main() {
    // Popup UI is pre-rendered from popup.crepus at build time.
    // No WASM initialization needed here.
}

/// Content-script entry (called from framework `content.js` after WASM init).
///
/// DOM scan + code-block replacement: `crepuscularity-webext/assets/content.js`
/// (copied to `dist/unpacked/src/content.js` on `crepus webext build`).
///
/// Rust WASM API used by that script:
/// - `extract_widgets` / `extract_specs` — parse AI message text
/// - `render_frontend` — shell HTML from `views/ui.crepus` (Panel, AnywhereShell)
/// - `render_anywhere_frame_doc` / `render_frame_doc` — iframe srcdoc
///
/// Widget UI templates: `views/ui.crepus`. Popup: `views/popup.crepus`.
#[wasm_bindgen]
pub fn content_main() {}

/// Handle background messages not handled in framework `background.js`
/// (`settings:get` / `settings:set` stay in JS for sync storage access).
#[wasm_bindgen]
pub fn handle_background_message(message: JsValue) -> Result<JsValue, JsValue> {
    let msg: Value = serde_wasm_bindgen::from_value(message)
        .unwrap_or_else(|_| json!({}));
    let _ = msg;
    wasm_object(&[
        ("ok", JsValue::from_bool(false)),
        (
            "error",
            JsValue::from_str("unknown message type (use framework background.js routes)"),
        ),
    ])
}

/// Initialize default extension settings on install.
#[wasm_bindgen]
pub fn settings_seed() -> Result<(), JsValue> {
    // Default settings are initialized by the browser's storage API.
    // This is a no-op since the app uses browser.storage.sync with defaults.
    Ok(())
}

/// App metadata for the settings popup.
#[wasm_bindgen]
pub fn app_manifest() -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&json!({
        "manifest": {
            "name": env!("CARGO_PKG_NAME"),
            "description": env!("CARGO_PKG_DESCRIPTION")
        }
    }))
    .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Browser API interaction program as a JSON data structure (CSP-safe).
#[wasm_bindgen]
pub fn browser_program_data() -> String {
    BrowserProgram::new()
        .bind_storage("auto_render", StorageArea::Sync, "autoRender")
        .bind_runtime_message(
            "settings",
            MessagePayload::new().with_field("type", JsExpr::string("settings:get")),
        )
        .set_storage(StorageArea::Local, "anywhereBooted", JsExpr::bool(true))
        .console_log([
            JsExpr::string("anywhere booted"),
            JsExpr::var("settings"),
            JsExpr::var("auto_render"),
            JsExpr::Literal(json!({ "framework": "crepuscularity", "product": "anywhere" })),
        ])
        .emit_data()
}

/// Browser API interaction program as a self-contained ES module string.
#[wasm_bindgen]
pub fn browser_program() -> String {
    BrowserProgram::new()
        .bind_storage("auto_render", StorageArea::Sync, "autoRender")
        .bind_runtime_message(
            "settings",
            MessagePayload::new().with_field("type", JsExpr::string("settings:get")),
        )
        .set_storage(StorageArea::Local, "anywhereBooted", JsExpr::bool(true))
        .console_log([
            JsExpr::string("anywhere booted"),
            JsExpr::var("settings"),
            JsExpr::var("auto_render"),
            JsExpr::Literal(json!({ "framework": "crepuscularity", "product": "anywhere" })),
        ])
        .emit_module()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_anywhere_tags ──────────────────────────────────────────────────

    #[test]
    fn parses_simple_anywhere_tag() {
        let input = r#"<ai-anywhere id="w1" type="chart" title="My Chart">
<anywhere-ui lang="crepus">div foo</anywhere-ui>
</ai-anywhere>"#;
        let widgets = parse_anywhere_tags(input);
        assert_eq!(widgets.len(), 1);
        let w = &widgets[0];
        assert_eq!(w.id.as_deref(), Some("w1"));
        assert_eq!(w.widget_type.as_deref(), Some("chart"));
        let ui = w.ui.as_ref().unwrap();
        assert_eq!(ui.lang, UiLang::Crepus);
    }

    #[test]
    fn parses_multiple_widgets() {
        let input = r#"<ai-anywhere id="a"></ai-anywhere> <ai-anywhere id="b"></ai-anywhere>"#;
        let widgets = parse_anywhere_tags(input);
        assert_eq!(widgets.len(), 2);
    }

    #[test]
    fn ignores_unclosed_tag() {
        assert!(parse_anywhere_tags("<ai-anywhere id=\"x\">no close").is_empty());
    }

    #[test]
    fn parses_workout_tracker_widget() {
        let input = r#"<ai-anywhere type="widget" title="Workout Tracker">
  <anywhere-ui lang="crepus">
div app
  $: let completed = {workouts.filter(w => w.done).length}
  for workout in {workouts}
    div card data-id={workout.id}
      "{workout.name}"
  </anywhere-ui>
  <anywhere-data>
{"title": "Weekly Workout Tracker", "workouts": [{"id": 1, "name": "Push Day", "done": true}]}
  </anywhere-data>
  <anywhere-script lang="js">
document.addEventListener("click", (e) => {
  const btn = e.target.closest("[data-action='toggle']");
});
  </anywhere-script>
</ai-anywhere>"#;
        let widgets = parse_anywhere_tags(input);
        assert_eq!(widgets.len(), 1, "expected one widget");
        let w = &widgets[0];
        assert_eq!(w.title.as_deref(), Some("Workout Tracker"));
        assert!(w.ui.is_some());
        assert!(w.data.is_some());
        assert!(w.script.is_some());
    }

    // ── extract_widget_specs ─────────────────────────────────────────────────

    #[test]
    fn extracts_json_widget_spec() {
        let input = "```aiwidget\n{\"title\":\"Foo\",\"html\":\"<b>hi</b>\"}\n```";
        let specs = extract_widget_specs(input);
        assert_eq!(specs[0].title, "Foo");
        assert_eq!(specs[0].html, "<b>hi</b>");
    }

    #[test]
    fn no_specs_from_plain_text() {
        assert!(extract_widget_specs("just text").is_empty());
    }

    // ── handle_popup_action (app logic) ─────────────────────────────────────

    fn call_action(action: &str, data: Value) -> Value {
        let checked = data.get("checked").and_then(|v| v.as_str()) == Some("true");
        let op = match action {
            "set-enabled"     => Some(json!({ "type": "set", "key": "enabled",    "value": checked, "area": "sync" })),
            "set-auto-render" => Some(json!({ "type": "set", "key": "autoRender", "value": checked, "area": "sync" })),
            "show-help"       => Some(json!({ "type": "set", "key": "showHelp",   "value": true  })),
            "hide-help"       => Some(json!({ "type": "set", "key": "showHelp",   "value": false })),
            "show-crepus"     => Some(json!({ "type": "set", "key": "showCrepus", "value": true  })),
            "hide-crepus"     => Some(json!({ "type": "set", "key": "showCrepus", "value": false })),
            _ => None,
        };
        match op { Some(op) => json!({ "storage_op": op }), None => json!({}) }
    }

    #[test]
    fn show_help_sets_true()  { assert_eq!(call_action("show-help", json!({}))["storage_op"]["value"], true); }
    #[test]
    fn hide_help_sets_false() { assert_eq!(call_action("hide-help", json!({}))["storage_op"]["value"], false); }
    #[test]
    fn set_enabled_reflects_checked() {
        let r = call_action("set-enabled", json!({ "checked": "true" }));
        assert_eq!(r["storage_op"]["value"], true);
        assert_eq!(r["storage_op"]["area"], "sync");
    }
    #[test]
    fn unknown_action_returns_empty() {
        assert!(call_action("bogus", json!({})).get("storage_op").is_none());
    }

    #[test]
    fn render_anywhere_shell_from_embedded_ui() {
        let ctx = TemplateContext::new();
        let files = embedded_files();
        let html = render_from_files(&files, "views/ui.crepus#AnywhereShell", &ctx)
            .expect("AnywhereShell should render");
        assert!(html.contains("data-role=\"frame-host\""));
        assert!(!html.contains("data-action=\"render\""));
        assert!(!html.contains("aa-title"));
    }

    #[test]
    fn render_inline_crepus_widget_uses_template_not_component_file() {
        let ui = r#"div app
  for workout in {workouts}
    div card
      "{workout.name}""#;
        let mut files = HashMap::new();
        files.insert("__ai_widget__".to_string(), ui.to_string());
        let mut ctx = TemplateContext::new();
        ctx.set(
            "workouts",
            json_to_template(json!([{"name": "Push Day"}])),
        );
        let html = render_from_files(&files, "__ai_widget__", &ctx)
            .expect("inline crepus should render as template");
        assert!(
            html.contains("Push Day") || html.contains("card"),
            "unexpected html: {html}"
        );
    }

    #[test]
    fn render_anywhere_parts_returns_html() {
        let input = include_str!("../tests/fixtures/workout_widget.html");
        let widgets = parse_anywhere_tags(input);
        let w = &widgets[0];
        let (html, css, js) = anywhere_frame_doc_parts(w);
        assert!(html.contains("Toggle"));
        assert!(!css.is_empty());
        assert!(js.is_empty());
    }

    #[test]
    fn frame_doc_survives_json_roundtrip() {
        let input = include_str!("../tests/fixtures/workout_widget.html");
        let widgets = parse_anywhere_tags(input);
        let json = serde_json::to_string(&widgets[0]).expect("json");
        let widget: AnywhereWidget = serde_json::from_str(&json).expect("from json");
        let (html, _, _) = anywhere_frame_doc_parts(&widget);
        assert!(
            html.contains("Toggle"),
            "json roundtrip widget lost ui; html={html}"
        );
    }

    #[test]
    fn workout_frame_renders_cards_and_buttons() {
        let input = r#"<ai-anywhere type="widget" title="Workout Tracker">
  <anywhere-ui lang="crepus">
div app
  $: let completed = {workouts.filter(w => w.done).length}
  $: let total = {workouts.length}

  h1 heading
    "{title}"

  div stats
    span
      "Completed {completed}/{total}"

  for workout in {workouts}
    div card data-id={workout.id}
      div row
        span name
          "{workout.name}"

        if {workout.done}
          span badge green
            "Done"
        else
          span badge yellow
            "Pending"

      button primary
        type="button"
        data-action="toggle"
        data-id={workout.id}
        "Toggle"

  match {completed === total}
    true =>
      div success
        "All workouts completed."
    _ =>
      div notice
        "Keep going."
  </anywhere-ui>
  <anywhere-data>
{
  "title": "Weekly Workout Tracker",
  "workouts": [
    { "id": 1, "name": "Push Day", "done": true },
    { "id": 2, "name": "Pull Day", "done": false },
    { "id": 3, "name": "Leg Day", "done": false }
  ]
}
  </anywhere-data>
</ai-anywhere>"#;
        let widgets = parse_anywhere_tags(input);
        let w = &widgets[0];
        let (html, _, _) = anywhere_frame_doc_parts(w);
        eprintln!("FRAME HTML:\n{html}");
        assert!(
            html.contains("Toggle") && html.contains("Push Day"),
            "expected workout UI in frame, got: {html}"
        );
    }

    #[test]
    fn ignores_anywhere_script_js_blocks() {
        let widget = AnywhereWidget {
            id: None,
            widget_type: Some("widget".into()),
            title: Some("Test".into()),
            ui: Some(AnywhereUi {
                lang: UiLang::Html,
                source: "<p>hi</p>".into(),
            }),
            script: Some(AnywhereScript {
                lang: ScriptLang::Js,
                source: "console.log('nope')".into(),
            }),
            data: None,
        };
        let (_, _, js) = anywhere_frame_doc_parts(&widget);
        assert!(js.is_empty());
    }
}
