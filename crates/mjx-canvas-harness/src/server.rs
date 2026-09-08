//! The local server the harness is served from: a hand-written HTTP/1.1 subset over
//! `std::net::TcpListener`, with no dependency at all.
//!
//! # ⚠ Why a browser, and why a server behind it
//!
//! `docs/client-platform/CANVAS_UI_INVENTORY.md` §3 asks for a standalone runnable application; the
//! user's own audit rule asks for *every UI element, in isolation, at both mobile and desktop sizes,
//! before any of it is wired into the application*; and eleven of the sixty-one elements are about
//! **touch**. A native desktop window satisfies the first and neither of the others: it cannot be
//! opened on a phone, and this environment has no windowing crate to build one with.
//!
//! A local server satisfies all three at once. `serve --host 0.0.0.0`, the machine's address on the
//! same network, and the eleven touch entries are being judged by a thumb on a real screen at a real
//! density — which is the only judgement of a touch target that means anything. The desktop case is
//! the same page in a desktop browser. **This is not the Tauri mobile surface the ticket describes**
//! and it is not pretending to be; see the crate documentation and MJXOFF-166's report for exactly
//! what is missing and what is not.
//!
//! # Why hand-written, and what it is not
//!
//! `CLAUDE.md`'s standing habit — a PNG encoder, a SHA-256, a JSON reader, a date formatter, all
//! written rather than depended on — plus the specific fact that this crate sits above the platform
//! boundary and a web framework would drag a runtime, a TLS stack and forty transitive crates into a
//! workspace whose whole discipline is about what it links.
//!
//! It binds **127.0.0.1 by default**, speaks the subset a browser needs (`GET`, `POST`,
//! `Content-Length`, no keep-alive, no chunked encoding, no TLS), and is a developer tool that must
//! never be reachable from anywhere it is not deliberately pointed at. `--host 0.0.0.0` is
//! deliberate and prints a warning when it is used.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use mjx_render_oracle::png::Image;
use mjx_tokens::Tokens;

use crate::inventory::INVENTORY;
use crate::render::{self, Overlays, Scene};
use crate::state::State;
use crate::tokens_source;

/// How many bytes of request body the server will read.
///
/// A token write-back is a name and a value; anything larger is a mistake or a probe, and refusing
/// it is cheaper than allocating for it.
pub const MAXIMUM_BODY: usize = 64 * 1024;

/// What the server holds between requests.
struct Harness {
    /// The tokens currently in force. Edited through `POST /api/tokens`, so a scene rendered after
    /// an edit is drawn with the new value without restarting.
    tokens: Mutex<Tokens>,
    /// The last image rendered for each query, so the pixel probe answers from **the same pixels
    /// the PNG was encoded from** rather than from a second reading of the image.
    cache: Mutex<BTreeMap<String, Image>>,
    /// Where the token source is.
    source: std::path::PathBuf,
}

/// Serve the harness on `host:port` until the process is stopped.
///
/// # Errors
///
/// A sentence, when the address cannot be bound.
pub fn serve(host: &str, port: u16) -> Result<(), String> {
    let listener = TcpListener::bind((host, port))
        .map_err(|error| format!("binding {host}:{port}: {error}"))?;
    let address = listener
        .local_addr()
        .map_or_else(|_| format!("{host}:{port}"), |at| at.to_string());

    let harness = Arc::new(Harness {
        tokens: Mutex::new(Tokens::DEFAULTS.clone()),
        cache: Mutex::new(BTreeMap::new()),
        source: tokens_source::source_path(),
    });

    println!("the canvas UI harness is at  http://{address}/");
    println!(
        "  {} elements, {} states each, one page",
        INVENTORY.len(),
        State::POINTS
    );
    if host == "0.0.0.0" {
        println!(
            "\n  ⚠ bound to every interface, so this is reachable from the network. That is what \
             makes the eleven touch entries judgeable on a real phone — open the address above on \
             one — and it is a developer tool with no authentication, so stop it when the sitting \
             is over."
        );
    }
    println!("\n  ctrl-c to stop.");

    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let harness = Arc::clone(&harness);
        // A thread per connection. A browser opens several at once for the page and its images, and
        // a sequential loop would make the harness feel broken while it is only busy.
        std::thread::spawn(move || {
            if let Err(error) = handle(&harness, stream) {
                eprintln!("connection: {error}");
            }
        });
    }
    Ok(())
}

/// One connection.
fn handle(harness: &Harness, stream: TcpStream) -> Result<(), String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|error| error.to_string())?);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|error| error.to_string())?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_owned();
    let target = parts.next().unwrap_or("/").to_owned();

    let mut length = 0_usize;
    loop {
        let mut header = String::new();
        if reader
            .read_line(&mut header)
            .map_err(|error| error.to_string())?
            == 0
        {
            break;
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some(value) = header
            .split_once(':')
            .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .map(|(_, value)| value.trim())
        {
            length = value.parse().unwrap_or(0).min(MAXIMUM_BODY);
        }
    }
    let mut body = vec![0_u8; length];
    if length > 0 {
        reader
            .read_exact(&mut body)
            .map_err(|error| error.to_string())?;
    }

    let (path, query) = target.split_once('?').unwrap_or((target.as_str(), ""));
    let response = route(harness, &method, path, query, &body);
    let mut stream = stream;
    write_response(&mut stream, &response)
}

/// One answer.
struct Response {
    status: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
}

impl Response {
    fn html(body: String) -> Self {
        Self {
            status: "200 OK",
            content_type: "text/html; charset=utf-8",
            body: body.into_bytes(),
        }
    }
    fn json(body: String) -> Self {
        Self {
            status: "200 OK",
            content_type: "application/json; charset=utf-8",
            body: body.into_bytes(),
        }
    }
    fn png(body: Vec<u8>) -> Self {
        Self {
            status: "200 OK",
            content_type: "image/png",
            body,
        }
    }
    fn text(status: &'static str, body: String) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8",
            body: body.into_bytes(),
        }
    }
}

/// Which answer a request gets.
fn route(harness: &Harness, method: &str, path: &str, query: &str, body: &[u8]) -> Response {
    match (method, path) {
        ("GET", "/") => {
            let tokens = harness.tokens.lock().map_or_else(
                |poisoned| poisoned.into_inner().clone(),
                |tokens| tokens.clone(),
            );
            Response::html(crate::page::render(&tokens))
        }
        ("GET", "/render.png") => match render_for(harness, query) {
            Ok((bytes, _)) => Response::png(bytes),
            Err(error) => Response::text("500 Internal Server Error", error),
        },
        ("GET", "/api/inventory") => Response::json(inventory_json()),
        ("GET", "/api/counters") => match counters_json(harness, query) {
            Ok(json) => Response::json(json),
            Err(error) => Response::text("500 Internal Server Error", error),
        },
        ("GET", "/api/probe") => match probe_json(harness, query) {
            Ok(json) => Response::json(json),
            Err(error) => Response::text("500 Internal Server Error", error),
        },
        ("GET", "/api/tokens") => {
            let tokens = harness.tokens.lock().map_or_else(
                |poisoned| poisoned.into_inner().clone(),
                |tokens| tokens.clone(),
            );
            Response::json(tokens_json(&tokens))
        }
        ("POST", "/api/tokens") => Response::json(write_token(harness, body)),
        ("GET", "/api/checklist") => Response::text(
            "200 OK",
            crate::checklist::render(&crate::plates::baselines()),
        ),
        _ => Response::text(
            "404 Not Found",
            format!("`{path}` is not a route this harness has"),
        ),
    }
}

/// Render whatever `query` names, and remember the image for the probe.
fn render_for(harness: &Harness, query: &str) -> Result<(Vec<u8>, Image), String> {
    let state = crate::page::state_from_query(query);
    let overlays = overlays_from_query(query);
    let number = pairs(query)
        .into_iter()
        .find(|(key, _)| key == "entry")
        .and_then(|(_, value)| value.parse::<u8>().ok())
        .unwrap_or(1);
    let entry = crate::inventory::entry(number)
        .ok_or_else(|| format!("{number} is not an inventory number; they run from 1 to 61"))?;
    let tokens = harness.tokens.lock().map_or_else(
        |poisoned| poisoned.into_inner().clone(),
        |tokens| tokens.clone(),
    );
    let scene = Scene::build(entry, &tokens, state);
    let (rendered, bytes) = render::render_png(&scene, overlays)?;
    if let Ok(mut cache) = harness.cache.lock() {
        // A bounded cache: the probe only ever asks about the picture that is on screen, so one
        // entry per open tab is plenty and an unbounded map would grow for the life of a sitting.
        if cache.len() > 32 {
            cache.clear();
        }
        cache.insert(query.to_owned(), rendered.image.clone());
    }
    Ok((bytes, rendered.image))
}

/// The overlays a query string asks for.
#[must_use]
pub fn overlays_from_query(query: &str) -> Overlays {
    let mut overlays = Overlays::NONE;
    for (key, value) in pairs(query) {
        match key.as_str() {
            "hit" => overlays.hit_test = value == "1",
            "ruler" => overlays.ruler = value == "1",
            _ => {}
        }
    }
    overlays
}

/// The sixty-one entries, as JSON.
fn inventory_json() -> String {
    let rows: Vec<String> = INVENTORY
        .iter()
        .map(|entry| {
            format!(
                "{{\"number\":{},\"slug\":{},\"family\":{},\"title\":{},\"description\":{},\
                 \"touch\":{}}}",
                entry.number,
                quote(&entry.slug()),
                quote(entry.family.slug()),
                quote(entry.title),
                quote(entry.description),
                entry.is_touch_sensitive()
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}

/// The counters for one render — the four numbers the completeness gate asserts, shown to the
/// person auditing so that *"this element draws"* is visible rather than only tested.
fn counters_json(harness: &Harness, query: &str) -> Result<String, String> {
    let state = crate::page::state_from_query(query);
    let number = pairs(query)
        .into_iter()
        .find(|(key, _)| key == "entry")
        .and_then(|(_, value)| value.parse::<u8>().ok())
        .unwrap_or(1);
    let entry = crate::inventory::entry(number)
        .ok_or_else(|| format!("{number} is not an inventory number"))?;
    let tokens = harness.tokens.lock().map_or_else(
        |poisoned| poisoned.into_inner().clone(),
        |tokens| tokens.clone(),
    );
    let scene = Scene::build(entry, &tokens, state);
    let rendered = render::render(&scene, overlays_from_query(query))?;
    let kinds: Vec<String> = render::kinds_drawn(&rendered.list)
        .into_iter()
        .map(|kind| kind.label().to_owned())
        .collect();
    Ok(format!(
        "{{\"state\":{},\"placeholders\":{},\"draw calls\":{},\"covered pixels\":{},\
         \"distinct colours\":{},\"grab regions\":{},\"draws\":{},\"pixels\":{}}}",
        quote(&state.slug()),
        rendered.report.placeholders,
        rendered.report.draw_calls,
        rendered.covered,
        rendered.distinct_colours,
        scene.canvas.grabs().len(),
        quote(&kinds.join(", ")),
        quote(&format!(
            "{} × {}",
            rendered.image.width, rendered.image.height
        )),
    ))
}

/// The pixel under the pointer, and what the spatial index says is there.
///
/// **Answered from the rendered `Image`, never from the browser's copy of the PNG.** A page that
/// read the pixel back with `getImageData` would be a second answer to *"what colour is this"*, and
/// the whole reason the canvas is drawn in Rust is that there should be one.
fn probe_json(harness: &Harness, query: &str) -> Result<String, String> {
    let coordinate = |name: &str| -> u32 {
        pairs(query)
            .into_iter()
            .find(|(key, _)| key == name)
            .and_then(|(_, value)| value.parse::<u32>().ok())
            .unwrap_or(0)
    };
    let (x, y) = (coordinate("x"), coordinate("y"));

    let cached = harness
        .cache
        .lock()
        .ok()
        .and_then(|cache| cache.get(&strip_probe(query)).cloned());
    let image = match cached {
        Some(image) => image,
        None => render_for(harness, &strip_probe(query))?.1,
    };
    let pixel = image.pixel(x, y).unwrap_or([0, 0, 0, 0]);

    // What the index says is at that point, in the scene's own coordinates.
    let state = crate::page::state_from_query(query);
    let number = pairs(query)
        .into_iter()
        .find(|(key, _)| key == "entry")
        .and_then(|(_, value)| value.parse::<u8>().ok())
        .unwrap_or(1);
    let hits = crate::inventory::entry(number).map_or_else(
        || "—".to_owned(),
        |entry| {
            let tokens = harness.tokens.lock().map_or_else(
                |poisoned| poisoned.into_inner().clone(),
                |tokens| tokens.clone(),
            );
            let scene = Scene::build(entry, &tokens, state);
            let (tree, _) = scene.canvas.tree();
            let index = mjx_layout::SpatialIndex::build(&tree);
            let scale = f64::from(scene.canvas.pixels_per_point());
            let point = mjx_layout::LayoutPoint::new(
                mjx_ooxml_core::measure::Emu::from_points(f64::from(x) / scale),
                mjx_ooxml_core::measure::Emu::from_points(f64::from(y) / scale),
            );
            format!(
                "{} fragments under the pointer",
                index.fragments_at(point).len()
            )
        },
    );

    Ok(format!(
        "{{\"point\":{},\"css\":{},\"rgba\":[{},{},{},{}],\"hits\":{}}}",
        quote(&format!("({x}, {y})")),
        quote(&format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            pixel[0], pixel[1], pixel[2], pixel[3]
        )),
        pixel[0],
        pixel[1],
        pixel[2],
        pixel[3],
        quote(&hits),
    ))
}

/// The query without the probe's own coordinates, so the cache key matches the render's.
fn strip_probe(query: &str) -> String {
    pairs(query)
        .into_iter()
        .filter(|(key, _)| key != "x" && key != "y")
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

/// Every design token, as JSON.
fn tokens_json(tokens: &Tokens) -> String {
    let rows: Vec<String> = tokens_source::editable(tokens)
        .into_iter()
        .map(|token| {
            format!(
                "{{\"path\":{},\"customProperty\":{},\"value\":{},\"kind\":{}}}",
                quote(token.path),
                quote(token.custom_property),
                quote(&token.value),
                quote(token.kind)
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}

/// Apply a token edit: to the running tokens **and** to the source file.
fn write_token(harness: &Harness, body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    let Some(property) = json_string(&text, "property") else {
        return refusal("the request named no `property`");
    };
    let Some(value) = json_string(&text, "value") else {
        return refusal("the request named no `value`");
    };

    // The file first. If the running tokens were changed and the write then failed, the harness
    // would be showing a value that is nowhere on disk — which is the exact failure the write-back
    // exists to prevent, one layer in.
    let written = match tokens_source::write_back(&harness.source, &property, &value) {
        Ok(written) => written,
        Err(error) => return refusal(&error.to_string()),
    };
    let applied = harness.tokens.lock().map_or_else(
        |poisoned| {
            let mut tokens = poisoned.into_inner();
            tokens.set_custom_property(&property, &value).is_ok()
        },
        |mut tokens| tokens.set_custom_property(&property, &value).is_ok(),
    );
    if !applied {
        return refusal("the value was written to the source but the running tokens refused it");
    }
    // **The alias warning.** A third of the token source is written in the W3C alias form, and
    // replacing `{color.green-deep}` with a literal stops this role following that one — which is
    // sometimes the intended tweak and is never something a person looking at a canvas would
    // notice. See `tokens_source::Rewritten::previous_was_alias`.
    let note = if written.previous_was_alias {
        format!(
            " ⚠ it was `{}`, an alias: this role no longer follows that one, and a later change to \
             it will not reach here.",
            written.previous
        )
    } else {
        String::new()
    };
    format!(
        "{{\"ok\":true,\"value\":{},\"previous\":{},\"wasAlias\":{},\"message\":{}}}",
        quote(&value),
        quote(&written.previous),
        written.previous_was_alias,
        quote(&format!(
            "{property} = {value} — written to {}.{note} Run `cargo run -p xtask -- tokens` to \
             carry it into the three generated artefacts.",
            tokens_source::SOURCE
        ))
    )
}

/// A refusal, as the JSON the panel shows.
fn refusal(reason: &str) -> String {
    format!(
        "{{\"ok\":false,\"value\":null,\"previous\":null,\"wasAlias\":false,\"message\":{}}}",
        quote(reason)
    )
}

/// The string value of a top-level key of a flat JSON object.
///
/// Deliberately not a JSON parser: the only body this server accepts is
/// `{"property": "…", "value": "…"}`, written by the script twenty lines above, and a parser for
/// arbitrary JSON would be a hundred lines answering a question nobody asks. `mjx-render-oracle`'s
/// `json` module is the parser this workspace has, and it reads manifests rather than requests.
fn json_string(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let at = text.find(&needle)?;
    let after = at + needle.len();
    let colon = after + text[after..].find(':')? + 1;
    let open = colon + text[colon..].find('"')? + 1;
    let bytes = text.as_bytes();
    let mut index = open;
    let mut out = String::new();
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if index + 1 < bytes.len() => {
                out.push(char::from(bytes[index + 1]));
                index += 2;
            }
            b'"' => return Some(out),
            byte => {
                out.push(char::from(byte));
                index += 1;
            }
        }
    }
    None
}

/// A JSON string.
#[must_use]
pub fn quote(text: &str) -> String {
    mjx_render_oracle::json::quote(text)
}

/// The `key=value` pairs of a query string, percent-decoded.
#[must_use]
pub fn pairs(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            (decode(key), decode(value))
        })
        .collect()
}

/// One percent-decoded query component.
fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        index += 3;
                    }
                    Err(_) => {
                        out.push(bytes[index]);
                        index += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Write one answer.
fn write_response(stream: &mut TcpStream, response: &Response) -> Result<(), String> {
    let head = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\n\
         Connection: close\r\n\r\n",
        response.status,
        response.content_type,
        response.body.len()
    );
    stream
        .write_all(head.as_bytes())
        .and_then(|()| stream.write_all(&response.body))
        .and_then(|()| stream.flush())
        .map_err(|error| error.to_string())
}
