//! The harness's chrome: one self-contained page, driven by the generated design tokens.
//!
//! # ⚠ The division this page exists to respect
//!
//! The user's standing constraint for the client platform is two sentences: *the document renderer
//! is entirely Rust, with no JavaScript graphics library; the chrome is TypeScript/HTML driven by
//! the generated design tokens.* This module is the second half and **only** the second half.
//!
//! There is no `<canvas>`, no SVG generation, no plotting library and no drawing of any kind in the
//! script below. The element under audit is an `<img>` whose bytes came out of
//! [`mod@crate::render`] — through `mjx-layout`'s fragment tree, `mjx-scene`'s display list and
//! `mjx-paint`'s software painter — and so are the ruler, the hit-test overlay and the page the
//! element sits on. The script changes a URL and sets `src`.
//!
//! The pixel inspector is the case worth being explicit about, because the shortcut is right there:
//! a browser can read a pixel out of an `<img>` with two lines of `getImageData`. It does not. The
//! probe is an endpoint, answered from the *same* `Pixels` the PNG was encoded from, because a
//! second reading of the image is a second answer to *"what colour is this"* and the whole reason
//! the canvas is in Rust is that there should be one.
//!
//! # Why the styling is generated rather than linked
//!
//! `ui/tokens/tokens.css` exists and is generated from the same source, and linking it would make
//! the page depend on a file relative to a working directory. Instead every custom property is
//! written into the page from [`mjx_tokens::TOKENS`] at request time — the *same* table
//! `ui/tokens/tokens.css` is emitted from — so the chrome and the canvas are demonstrably reading
//! one set of values, and a token edited in the panel restyles the chrome and the element together.
//!
//! # What this page is not
//!
//! It is not TypeScript. `ui/` carries no build step in this repository — `tokens.ts` is generated
//! and consumed by whatever Phase U brings — so the script here is a plain ES module with no
//! transpilation, and it is small enough (one fetch, one form, three listeners) that the types
//! would be documenting themselves. When Phase U lands a `ui/` toolchain this is the first thing
//! that should move into it.

use mjx_tokens::{Tokens, TOKENS};

use crate::inventory::{Family, INVENTORY};
use crate::state::{Density, Input, Interaction, State};

/// The whole page, for the tokens currently in force.
#[must_use]
pub fn render(tokens: &Tokens) -> String {
    let mut html = String::with_capacity(32_768);
    html.push_str(
        "<!doctype html>\n<html lang=\"en\" data-scheme=\"light\">\n<head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1, \
         viewport-fit=cover\">\n\
         <title>mjx canvas UI harness — 61 elements</title>\n<style>\n",
    );
    html.push_str(&custom_properties(tokens));
    html.push_str(STYLE);
    html.push_str("</style>\n</head>\n<body>\n");

    html.push_str(
        "<header>\n\
         <h1>In-canvas UI</h1>\n\
         <p class=\"note\">Every pixel below is drawn in Rust — <code>mjx-layout</code> → \
         <code>mjx-scene</code> → <code>mjx-paint</code>'s software painter. This page is the \
         chrome.</p>\n\
         <button id=\"panels\" class=\"ghost\" type=\"button\">Panels</button>\n\
         </header>\n",
    );

    html.push_str("<main>\n");

    // ---- the scene list -------------------------------------------------------------------
    html.push_str("<nav id=\"scenes\" aria-label=\"the sixty-one elements\">\n");
    html.push_str(
        "<label class=\"search\"><span class=\"visually-hidden\">Search the inventory</span>\
         <input id=\"search\" type=\"search\" placeholder=\"Search 61 elements…\" \
         autocomplete=\"off\"></label>\n",
    );
    for family in Family::ALL {
        html.push_str(&format!(
            "<h2 data-family=\"{}\">§{} {} <span class=\"count\">{}</span></h2>\n<ul>\n",
            family.slug(),
            family.section(),
            escape(family.label()),
            family.size()
        ));
        for entry in INVENTORY.iter().filter(|entry| entry.family == family) {
            html.push_str(&format!(
                "<li><button type=\"button\" data-entry=\"{}\" data-search=\"{}\">\
                 <span class=\"number\">{}</span> <span class=\"title\">{}</span>\
                 {}</button></li>\n",
                entry.number,
                escape(&format!("{} {}", entry.number, entry.title).to_lowercase()),
                entry.number,
                escape(entry.title),
                if entry.is_touch_sensitive() {
                    "<span class=\"touch\" title=\"only judgeable on a touch device\">touch</span>"
                } else {
                    ""
                }
            ));
        }
        html.push_str("</ul>\n");
    }
    html.push_str("</nav>\n");

    // ---- the stage ------------------------------------------------------------------------
    html.push_str(
        "<section id=\"stage\">\n\
         <div id=\"heading\"><h2 id=\"title\"></h2><p id=\"judgement\"></p></div>\n\
         <figure id=\"frame\"><img id=\"element\" alt=\"\" draggable=\"false\"></figure>\n\
         <p id=\"probe\" class=\"probe\">Move the pointer over the element to sample a \
         pixel.</p>\n\
         <dl id=\"counters\" class=\"counters\"></dl>\n\
         </section>\n",
    );

    // ---- the panels -----------------------------------------------------------------------
    html.push_str("<aside id=\"panel\">\n");
    html.push_str("<h2>State</h2>\n<div class=\"axes\">\n");
    html.push_str(&axis_group(
        "interaction",
        &Interaction::ALL
            .iter()
            .map(|value| (value.slug(), value.slug()))
            .collect::<Vec<_>>(),
        "default",
    ));
    html.push_str(&axis_group(
        "scheme",
        &[("light", "light"), ("dark", "dark")],
        "light",
    ));
    html.push_str(&axis_group(
        "density",
        &Density::ALL
            .iter()
            .map(|value| (value.slug(), value.slug()))
            .collect::<Vec<_>>(),
        "1x",
    ));
    html.push_str(&axis_group(
        "input",
        &Input::ALL
            .iter()
            .map(|value| (value.slug(), value.slug()))
            .collect::<Vec<_>>(),
        "pointer",
    ));
    html.push_str("</div>\n");

    html.push_str(
        "<h2>Overlays</h2>\n<div class=\"toggles\">\n\
         <label><input type=\"checkbox\" id=\"hit\"> Hit-test regions</label>\n\
         <label><input type=\"checkbox\" id=\"ruler\"> Ruler and page box</label>\n\
         <label><input type=\"checkbox\" id=\"zoom\"> Magnify 2×</label>\n\
         </div>\n\
         <p class=\"note\">A grab region is <code>SpatialIndex::bounds_of</code> inflated by the \
         input device's padding — the index's own answer, not a second one. Set <em>input</em> to \
         touch and watch every region grow.</p>\n",
    );

    html.push_str(
        "<h2>Design tokens</h2>\n\
         <p class=\"note\">Editing a value changes this page <em>and</em> the element, and writes \
         back to <code>docs/client-platform/data/tokens.json</code>. Run \
         <code>cargo run -p xtask -- tokens</code> afterwards to carry it into the three generated \
         artefacts.</p>\n\
         <label class=\"search\"><span class=\"visually-hidden\">Filter tokens</span>\
         <input id=\"token-search\" type=\"search\" placeholder=\"Filter 92 tokens…\" \
         autocomplete=\"off\"></label>\n\
         <div id=\"tokens\"></div>\n\
         <p id=\"token-status\" class=\"status\" role=\"status\"></p>\n",
    );
    html.push_str("</aside>\n");
    html.push_str("</main>\n");

    html.push_str("<script type=\"module\">\n");
    html.push_str(SCRIPT);
    html.push_str("\n</script>\n</body>\n</html>\n");
    html
}

/// One axis of the state matrix, as a radio group.
fn axis_group(axis: &str, values: &[(&str, &str)], selected: &str) -> String {
    let mut html = format!("<fieldset data-axis=\"{axis}\"><legend>{axis}</legend>\n");
    for (value, label) in values {
        html.push_str(&format!(
            "<label><input type=\"radio\" name=\"{axis}\" value=\"{value}\"{}> {}</label>\n",
            if *value == selected { " checked" } else { "" },
            escape(label)
        ));
    }
    html.push_str("</fieldset>\n");
    html
}

/// Every design token as a CSS custom property, from the generated table.
///
/// The same table `ui/tokens/tokens.css` is emitted from, read at request time so that a token
/// edited in the panel restyles this page on the next load without a build step.
#[must_use]
pub fn custom_properties(tokens: &Tokens) -> String {
    let mut css = String::with_capacity(4096);
    css.push_str(":root {\n");
    for identity in TOKENS {
        if let Some(value) = tokens.custom_property(identity.custom_property) {
            css.push_str(&format!("  {}: {};\n", identity.custom_property, value));
        }
    }
    css.push_str("}\n");
    css
}

/// The five characters that must not reach markup as themselves.
#[must_use]
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// The chrome's own stylesheet, written against the custom properties above.
///
/// Mobile first, because eleven of the sixty-one elements are only judgeable on a phone and a
/// harness that needed a desktop to reach them would have covered them on paper only. The panels
/// collapse into a sheet below 900 pixels and the element stays the widest thing on the screen.
const STYLE: &str = "\
* { box-sizing: border-box; }
html, body { margin: 0; height: 100%; }
body {
  font: 14px/1.5 var(--font-sans);
  background: var(--theme-light-background);
  color: var(--theme-light-text-primary);
  -webkit-text-size-adjust: 100%;
}
[data-scheme='dark'] body {
  background: var(--theme-dark-background);
  color: var(--theme-dark-text-primary);
}
header {
  display: flex; align-items: baseline; gap: 1rem; flex-wrap: wrap;
  padding: .75rem 1rem; border-bottom: 1px solid var(--theme-light-border-subtle);
  position: sticky; top: 0; z-index: 3;
  background: var(--theme-light-surface);
}
[data-scheme='dark'] header {
  background: var(--theme-dark-surface); border-bottom-color: var(--theme-dark-border-subtle);
}
h1 { font-size: 1rem; margin: 0; letter-spacing: var(--tracking-tight); }
h2 { font-size: .8rem; margin: 1.2rem 0 .4rem; text-transform: uppercase;
     letter-spacing: .06em; opacity: .7; }
.note { font-size: .78rem; opacity: .72; margin: .4rem 0; }
.status { font-size: .78rem; min-height: 1.2em; margin: .4rem 0; }
.status[data-kind='error'] { color: var(--color-honey-deep); }
.status[data-kind='ok'] { color: var(--color-green-deep); }
.status[data-kind='warn'] { color: var(--color-honey-deep); }
code { font-family: var(--font-mono); font-size: .92em; }
main { display: grid; grid-template-columns: 17rem 1fr 20rem; height: calc(100% - 3.4rem); }
nav, aside { overflow-y: auto; padding: 0 1rem 3rem; }
nav { border-right: 1px solid var(--theme-light-border-subtle); }
aside { border-left: 1px solid var(--theme-light-border-subtle); }
[data-scheme='dark'] nav { border-right-color: var(--theme-dark-border-subtle); }
[data-scheme='dark'] aside { border-left-color: var(--theme-dark-border-subtle); }
nav ul { list-style: none; margin: 0; padding: 0; }
nav li button {
  display: flex; gap: .5rem; align-items: baseline; width: 100%; text-align: left;
  border: 0; background: none; color: inherit; font: inherit; cursor: pointer;
  padding: .3rem .5rem; border-radius: var(--radius-chip);
}
nav li button:hover { background: var(--color-sage-tint); }
[data-scheme='dark'] nav li button:hover { background: var(--theme-dark-surface-raised); }
nav li button[aria-current='true'] {
  background: var(--theme-light-accent-surface); color: var(--theme-light-accent-pressed);
  font-weight: var(--font-weight-semibold);
}
[data-scheme='dark'] nav li button[aria-current='true'] {
  background: var(--theme-dark-accent-surface); color: var(--theme-dark-accent);
}
.number { font-family: var(--font-mono); font-size: .75rem; opacity: .6; min-width: 1.6em; }
.title { flex: 1; }
.touch {
  font-size: .62rem; text-transform: uppercase; letter-spacing: .08em;
  background: var(--color-honey-tint); color: var(--color-honey-deep);
  padding: .05rem .3rem; border-radius: var(--radius-chip);
}
.count { font-weight: var(--font-weight-medium); opacity: .5; }
.search input, select, input[type='text'] {
  width: 100%; padding: .35rem .5rem; font: inherit;
  border: 1px solid var(--theme-light-border); border-radius: var(--radius-control);
  background: var(--theme-light-surface); color: inherit;
}
[data-scheme='dark'] .search input, [data-scheme='dark'] input[type='text'] {
  background: var(--theme-dark-surface); border-color: var(--theme-dark-border);
}
.search { display: block; margin: .75rem 0; }
.visually-hidden {
  position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%);
}
#stage { padding: 1rem; overflow: auto; }
#heading h2 { margin: 0; font-size: 1rem; text-transform: none; letter-spacing: 0; opacity: 1; }
#judgement { font-size: .82rem; opacity: .78; margin: .3rem 0 .8rem; max-width: 56ch; }
#frame {
  margin: 0; display: inline-block; line-height: 0;
  border: 1px solid var(--theme-light-border); border-radius: var(--radius-card);
  overflow: hidden; box-shadow: var(--shadow-card);
}
[data-scheme='dark'] #frame { border-color: var(--theme-dark-border); }
#element { display: block; width: 600px; max-width: 100%; height: auto; touch-action: none; }
#element.zoom { width: 1200px; image-rendering: pixelated; }
.probe { font-family: var(--font-mono); font-size: .75rem; opacity: .8; margin: .6rem 0; }
.swatch {
  display: inline-block; width: .8em; height: .8em; vertical-align: -.05em;
  border: 1px solid var(--theme-light-border); border-radius: 2px; margin-right: .3em;
}
.counters { display: grid; grid-template-columns: max-content 1fr; gap: .1rem .75rem;
            font-size: .78rem; margin: .8rem 0 0; max-width: 40rem; }
.counters dt { opacity: .6; }
.counters dd { margin: 0; font-family: var(--font-mono); }
fieldset { border: 0; margin: 0 0 .7rem; padding: 0; }
legend { font-size: .7rem; text-transform: uppercase; letter-spacing: .07em; opacity: .6;
         padding: 0 0 .2rem; }
fieldset label, .toggles label { display: inline-flex; align-items: center; gap: .3rem;
                                 margin: 0 .7rem .25rem 0; font-size: .82rem; }
.toggles { display: flex; flex-direction: column; gap: .2rem; }
.token { display: grid; grid-template-columns: 1fr 5.5rem 2rem; gap: .35rem;
         align-items: center; margin-bottom: .25rem; }
.token span { font-family: var(--font-mono); font-size: .68rem; overflow: hidden;
              text-overflow: ellipsis; white-space: nowrap; }
.token input[type='text'] { font-family: var(--font-mono); font-size: .7rem; padding: .2rem .3rem; }
.token input[type='color'] { width: 100%; height: 1.7rem; padding: 0; border: 0; background: none; }
.ghost { border: 1px solid var(--theme-light-border); background: none; color: inherit;
         font: inherit; padding: .2rem .6rem; border-radius: var(--radius-control);
         cursor: pointer; display: none; margin-left: auto; }
@media (max-width: 900px) {
  main { grid-template-columns: 1fr; height: auto; }
  nav, aside { border: 0; max-height: none; }
  nav { order: 3; }
  aside { order: 2; }
  #stage { order: 1; }
  .ghost { display: block; }
  aside[hidden], nav[hidden] { display: none; }
  #element { width: 100%; }
  #element.zoom { width: 200%; }
}
";

/// The chrome's script. One fetch, one form, three listeners — and no drawing.
const SCRIPT: &str = r#"
const root = document.documentElement;
const image = document.getElementById('element');
const title = document.getElementById('title');
const judgement = document.getElementById('judgement');
const counters = document.getElementById('counters');
const probe = document.getElementById('probe');
const tokenList = document.getElementById('tokens');
const tokenStatus = document.getElementById('token-status');

let entry = Number(new URLSearchParams(location.search).get('entry') || 1);
let inventory = [];

const state = () => ({
  interaction: document.querySelector('input[name=interaction]:checked').value,
  scheme: document.querySelector('input[name=scheme]:checked').value,
  density: document.querySelector('input[name=density]:checked').value,
  input: document.querySelector('input[name=input]:checked').value,
  hit: document.getElementById('hit').checked,
  ruler: document.getElementById('ruler').checked,
});

function query() {
  const s = state();
  const p = new URLSearchParams({
    entry: String(entry),
    interaction: s.interaction, scheme: s.scheme, density: s.density, input: s.input,
  });
  if (s.hit) p.set('hit', '1');
  if (s.ruler) p.set('ruler', '1');
  return p;
}

async function show() {
  const p = query();
  root.dataset.scheme = state().scheme;
  image.className = document.getElementById('zoom').checked ? 'zoom' : '';
  image.src = '/render.png?' + p.toString();
  const found = inventory.find((e) => e.number === entry);
  if (found) {
    title.textContent = found.number + ' · ' + found.title;
    judgement.textContent = found.description;
    image.alt = found.title;
  }
  for (const button of document.querySelectorAll('nav button[data-entry]')) {
    button.setAttribute('aria-current', Number(button.dataset.entry) === entry ? 'true' : 'false');
  }
  history.replaceState(null, '', '?' + p.toString());
  const report = await fetch('/api/counters?' + p.toString()).then((r) => r.json());
  counters.innerHTML = Object.entries(report)
    .map(([k, v]) => '<dt>' + k + '</dt><dd>' + v + '</dd>').join('');
}

image.addEventListener('pointermove', async (event) => {
  const box = image.getBoundingClientRect();
  const x = Math.floor((event.clientX - box.left) / box.width * image.naturalWidth);
  const y = Math.floor((event.clientY - box.top) / box.height * image.naturalHeight);
  const p = query();
  p.set('x', String(x)); p.set('y', String(y));
  const answer = await fetch('/api/probe?' + p.toString()).then((r) => r.json());
  probe.innerHTML = '<span class="swatch" style="background:' + answer.css + '"></span>'
    + answer.point + '  ' + answer.css + '  ' + answer.hits;
});

for (const input of document.querySelectorAll('input[type=radio], #hit, #ruler, #zoom')) {
  input.addEventListener('change', show);
}
for (const button of document.querySelectorAll('nav button[data-entry]')) {
  button.addEventListener('click', () => { entry = Number(button.dataset.entry); show(); });
}
document.getElementById('search').addEventListener('input', (event) => {
  const needle = event.target.value.trim().toLowerCase();
  for (const button of document.querySelectorAll('nav button[data-entry]')) {
    button.parentElement.hidden = needle !== '' && !button.dataset.search.includes(needle);
  }
});
document.getElementById('panels').addEventListener('click', () => {
  const aside = document.getElementById('panel');
  const nav = document.getElementById('scenes');
  aside.hidden = !aside.hidden;
  nav.hidden = aside.hidden ? false : true;
});

async function loadTokens() {
  const tokens = await fetch('/api/tokens').then((r) => r.json());
  tokenList.innerHTML = '';
  for (const token of tokens) {
    const row = document.createElement('div');
    row.className = 'token';
    row.dataset.search = (token.path + ' ' + token.value).toLowerCase();
    const name = document.createElement('span');
    name.textContent = token.path;
    name.title = token.customProperty;
    const text = document.createElement('input');
    text.type = 'text';
    text.value = token.value;
    const swatch = document.createElement('input');
    swatch.type = 'color';
    swatch.value = token.kind === 'color' ? token.value.slice(0, 7) : '#000000';
    swatch.style.visibility = token.kind === 'color' ? 'visible' : 'hidden';
    const commit = async (value) => {
      const answer = await fetch('/api/tokens', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ property: token.customProperty, value }),
      }).then((r) => r.json());
      tokenStatus.textContent = answer.message;
      // An alias that was flattened is a change to the token system rather than to a value, so it
      // is shown in the warning colour even though the write succeeded.
      tokenStatus.dataset.kind = answer.ok ? (answer.wasAlias ? 'warn' : 'ok') : 'error';
      if (answer.ok) {
        text.value = answer.value;
        root.style.setProperty(token.customProperty, answer.value);
        show();
      }
    };
    text.addEventListener('change', () => commit(text.value));
    swatch.addEventListener('change', () => { text.value = swatch.value; commit(swatch.value); });
    row.append(name, text, swatch);
    tokenList.append(row);
  }
}
document.getElementById('token-search').addEventListener('input', (event) => {
  const needle = event.target.value.trim().toLowerCase();
  for (const row of tokenList.children) {
    row.hidden = needle !== '' && !row.dataset.search.includes(needle);
  }
});

inventory = await fetch('/api/inventory').then((r) => r.json());
await loadTokens();
await show();
"#;

/// The state a query string names, or [`State::CANONICAL`] for anything it does not.
///
/// Deliberately forgiving: a harness that answered 400 to a hand-typed URL would be a harness a
/// person stops typing URLs into. What it is *not* forgiving about is silence — [`State::slug`] is
/// echoed back in the counters panel, so a value that was ignored is visible rather than assumed.
#[must_use]
pub fn state_from_query(query: &str) -> State {
    let mut state = State::CANONICAL;
    for (key, value) in crate::server::pairs(query) {
        match key.as_str() {
            "interaction" => {
                if let Some(parsed) = Interaction::parse(&value) {
                    state.interaction = parsed;
                }
            }
            "scheme" => {
                if let Some(parsed) = crate::state::parse_scheme(&value) {
                    state.scheme = parsed;
                }
            }
            "density" => {
                if let Some(parsed) = Density::parse(&value) {
                    state.density = parsed;
                }
            }
            "input" => {
                if let Some(parsed) = Input::parse(&value) {
                    state.input = parsed;
                }
            }
            _ => {}
        }
    }
    state
}
