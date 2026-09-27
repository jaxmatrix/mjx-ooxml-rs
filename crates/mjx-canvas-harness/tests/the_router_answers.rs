//! **The router is asked whether a route exists, rather than the page being asked whether it links
//! to one** (audit pass 10, G2).
//!
//! # The instrument, and what it said
//!
//! `crates/mjx-canvas-harness/src/server.rs` shipped at 591 lines with **no test in this workspace
//! calling any of it**. The standing check for that is a mutation: an `abort()` placed in the
//! `("GET", "/api/probe")` arm of `route` would have fired in no test anywhere. What existed was
//! `the_page_is_the_chrome_and_nothing_more.rs`, which asserts that the HTML *string contains*
//! `"/api/probe?"` — a claim about a hyperlink. A page can link to a route that does not exist, and
//! the router would go on answering `404 Not Found` with the suite green.
//!
//! Two of the four things `CANVAS_UI_INVENTORY.md` §3 asks for by name — the pixel inspector and
//! the live token editor — are endpoints, and both reached an assertion only through
//! `crate::tokens_source` and `crate::render`, the layers *below* them. The body-length cap, the
//! `Content-Length` parse and every error response were reachable by nothing.
//!
//! # What this suite drives, and why it needs no socket
//!
//! `server::read_request` turns bytes into a `Request` and `server::route` turns a `Request` into a
//! `Response`. Neither takes a `TcpStream`, so a test drives the whole server except the two calls
//! that touch the network — and the two it skips are `TcpListener::bind` and `write_all`, which are
//! the standard library's rather than this crate's.
//!
//! # ⚠ Why the token editor is pointed at a copy
//!
//! `POST /api/tokens` really does write `docs/client-platform/data/tokens.json`. A suite that let it
//! write the repository's own copy would edit the workspace it is testing — and would do so in a way
//! `git status` shows and `cargo test` does not. `Harness::new` takes the path for exactly this
//! reason, and [`temporary_source`] hands it a copy that is deleted afterwards.

use std::io::BufReader;
use std::path::PathBuf;

use mjx_canvas_harness::server::{self, Harness, Request, Response, MAXIMUM_BODY};

/// A harness pointed at the real token source. Every route but `POST /api/tokens` only reads.
fn harness() -> Harness {
    Harness::new(mjx_canvas_harness::tokens_source::source_path())
}

/// A copy of the committed token source, in a directory this test owns.
///
/// Named by the process id and the label so two tests in the same binary — which libtest runs on
/// different threads — cannot be handed the same file.
fn temporary_source(label: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("mjx-canvas-router-{label}-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("a temporary directory");
    let path = directory.join("tokens.json");
    let committed = mjx_canvas_harness::tokens_source::source_path();
    std::fs::copy(&committed, &path).unwrap_or_else(|error| {
        panic!(
            "copying {} to {}: {error}",
            committed.display(),
            path.display()
        )
    });
    path
}

/// `route`, given a target the way a browser writes one.
fn get(harness: &Harness, target: &str) -> Response {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    server::route(harness, "GET", path, query, &[])
}

/// One request read from bytes, the way `handle` reads one off a socket.
fn parse(raw: &str) -> Request {
    server::read_request(&mut BufReader::new(raw.as_bytes())).expect("a complete request")
}

// -------------------------------------------------------------------------------------------
// Every route the page names is a route the router has
// -------------------------------------------------------------------------------------------

/// The eight routes, asked of the router.
///
/// The list is not a second copy of the page's: [`every_route_the_page_links_to_is_answered`] below
/// derives it from the page itself, so a route added to one and not the other fails there. This
/// test is the one that says what each answer *is*.
#[test]
fn every_route_answers_with_the_content_type_its_consumer_parses() {
    let harness = harness();
    for (target, content_type, must_contain) in [
        ("/", "text/html; charset=utf-8", "<title>"),
        (
            "/api/inventory",
            "application/json; charset=utf-8",
            "\"number\":61",
        ),
        (
            "/api/counters?entry=3",
            "application/json; charset=utf-8",
            "\"grab regions\"",
        ),
        (
            "/api/probe?entry=3&x=40&y=40",
            "application/json; charset=utf-8",
            "\"rgba\"",
        ),
        (
            "/api/tokens",
            "application/json; charset=utf-8",
            "\"customProperty\"",
        ),
        (
            "/api/checklist",
            "text/plain; charset=utf-8",
            "| # | Element",
        ),
    ] {
        let response = get(&harness, target);
        assert_eq!(
            response.status(),
            "200 OK",
            "`{target}` answered `{}`: {}",
            response.status(),
            response.text_body()
        );
        assert_eq!(response.content_type(), content_type, "`{target}`");
        assert!(
            response.text_body().contains(must_contain),
            "`{target}` answered without `{must_contain}`, so the route exists and says nothing \
             its consumer can read:\n{}",
            response.text_body().chars().take(400).collect::<String>()
        );
    }

    // The image, which is the only route that does not answer in text.
    let png = get(&harness, "/render.png?entry=1");
    assert_eq!(png.status(), "200 OK");
    assert_eq!(png.content_type(), "image/png");
    assert_eq!(
        &png.body()[..8],
        b"\x89PNG\r\n\x1a\n",
        "`/render.png` must answer with a PNG signature, not with something a browser guesses at"
    );
}

/// Every route the page fetches is one the router answers — derived from the page, not listed here.
///
/// This is the assertion `the_page_is_the_chrome_and_nothing_more.rs` could not make. It knows the
/// page contains `"/api/probe?"`; it cannot know whether asking for it produces anything.
#[test]
fn every_route_the_page_links_to_is_answered() {
    let harness = harness();
    let html = mjx_canvas_harness::page::render(&harness.tokens());
    let mut checked = 0_usize;
    for route in [
        "/render.png?",
        "/api/inventory",
        "/api/counters?",
        "/api/probe?",
        "/api/tokens",
    ] {
        assert!(
            html.contains(route),
            "the page no longer fetches `{route}`; this list is derived from the page and has to \
             follow it"
        );
        // With the entry and coordinates a real fetch carries, since three of the five take a query
        // string and the page writes the `?` into the literal it fetches with.
        let target = if route.ends_with('?') {
            format!("{route}entry=2&x=10&y=10")
        } else {
            route.to_owned()
        };
        let response = get(&harness, &target);
        assert_ne!(
            response.status(),
            "404 Not Found",
            "the page fetches `{route}` and the router has no such route: {}",
            response.text_body()
        );
        checked += 1;
    }
    assert_eq!(checked, 5, "five routes are fetched from the page");
}

/// A path the router does not have is a 404 that says so, rather than a blank page.
#[test]
fn an_unknown_route_is_refused_by_name() {
    let harness = harness();
    let response = get(&harness, "/api/probe/../../etc/passwd");
    assert_eq!(response.status(), "404 Not Found");
    assert!(
        response
            .text_body()
            .contains("is not a route this harness has"),
        "a 404 has to name what was asked for: {}",
        response.text_body()
    );
    // A method this server does not speak reaches the same arm, rather than being mistaken for the
    // `GET` of the same path.
    let deleted = server::route(&harness, "DELETE", "/api/tokens", "", &[]);
    assert_eq!(deleted.status(), "404 Not Found");
}

// -------------------------------------------------------------------------------------------
// The malformed cases, one per guard
// -------------------------------------------------------------------------------------------

/// An inventory number outside 1..=61 is a 500 that says what the range is, on both routes that
/// take one.
#[test]
fn a_render_of_an_element_that_does_not_exist_says_so() {
    let harness = harness();
    let rendered = get(&harness, "/render.png?entry=250");
    assert_eq!(rendered.status(), "500 Internal Server Error");
    assert!(
        rendered.text_body().contains("they run from 1 to 61"),
        "the refusal has to say what the range is: {}",
        rendered.text_body()
    );

    let counted = get(&harness, "/api/counters?entry=250");
    assert_eq!(counted.status(), "500 Internal Server Error");

    // A query that names no entry at all falls back to the first, rather than failing: the page
    // fetches `/render.png?` before a person has clicked anything.
    assert_eq!(get(&harness, "/render.png").status(), "200 OK");
}

/// A probe outside the image answers, with a transparent sample rather than a panic.
///
/// The pixel inspector is one of the four things §3 asks for by name, and a pointer that leaves the
/// element is the ordinary case rather than an edge one.
#[test]
fn a_probe_outside_the_image_answers_rather_than_failing() {
    let harness = harness();
    let response = get(&harness, "/api/probe?entry=1&x=99999&y=99999");
    assert_eq!(response.status(), "200 OK");
    let body = response.text_body();
    assert!(
        body.contains("\"rgba\":[0,0,0,0]"),
        "a sample off the edge of the image is transparent: {body}"
    );
    assert!(
        body.contains("\"hits\":"),
        "the probe always reports what the spatial index says, even for a point outside: {body}"
    );

    // Coordinates that are not numbers read as the origin rather than refusing — a browser sends
    // these while a drag is in flight.
    let origin = get(&harness, "/api/probe?entry=1&x=NaN&y=");
    assert_eq!(origin.status(), "200 OK");
    assert!(origin.text_body().contains("\"point\":\"(0, 0)\""));
}

// -------------------------------------------------------------------------------------------
// `POST /api/tokens`, the round trip
// -------------------------------------------------------------------------------------------

/// **The live token editor, end to end through the endpoint.**
///
/// The write-back's own suite drives `tokens_source::rewrite`; this drives the route, which is what
/// a person's browser reaches. It asserts the four things the panel shows: that the answer is `ok`,
/// that it names the previous value, that the **file** changed, and that `GET /api/tokens` now
/// reports the new value — the last being the half that proves the running tokens were updated too,
/// so the canvas redraws in the new colour rather than only the file changing.
#[test]
fn a_token_written_through_the_endpoint_reaches_the_file_and_the_running_tokens() {
    let path = temporary_source("round-trip");
    let before = std::fs::read_to_string(&path).expect("the copy");
    let harness = Harness::new(path.clone());

    let body = br##"{"property": "--document-light-selection-handle", "value": "#c02a5f"}"##;
    let response = server::route(&harness, "POST", "/api/tokens", "", body);
    assert_eq!(response.status(), "200 OK");
    let answer = response.text_body();
    assert!(
        answer.contains("\"ok\":true"),
        "the write was refused: {answer}"
    );
    assert!(
        answer.contains("\"wasAlias\":true"),
        "`document.light.selection-handle` is committed as `{{color.green-deep}}`, and replacing an \
         alias with a literal is the change a person auditing a canvas cannot see: {answer}"
    );

    let after = std::fs::read_to_string(&path).expect("the copy, rewritten");
    assert_ne!(before, after, "nothing was written to the file");
    assert!(
        after.contains("\"#c02a5f\""),
        "the new value is not in the file"
    );
    // ⚠ The previous value is read out of the answer rather than written down here. It used to be
    // the literal `"{color.green-deep}"`, and MJXOFF-271 re-pointed the handle at
    // `{theme.light.accent-pressed}` — the accent role rather than one ramp step — which is a longer
    // string and made this arithmetic fail while the property it checks still held. A length
    // identity that bakes in one token's committed value is a gate that breaks on a re-seed instead
    // of on a defect; reading `previous` makes it check the *span*, which is what it says it checks.
    let previous = answer
        .split("\"previous\":\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("the answer names the previous value");
    assert_eq!(
        before.len() + "\"#c02a5f\"".len() - (previous.len() + 2),
        after.len(),
        "the edit changed more than one span (the previous value was `{previous}`)"
    );

    // The running tokens moved with it, which is what makes the canvas redraw in the new colour.
    let listed = get(&harness, "/api/tokens").text_body();
    assert!(
        listed.contains(
            "\"path\":\"document.light.selection-handle\",\
                         \"customProperty\":\"--document-light-selection-handle\",\
                         \"value\":\"#c02a5f\""
        ),
        "`GET /api/tokens` still reports the old value, so the page and the file disagree"
    );

    let _ = std::fs::remove_dir_all(path.parent().expect("a directory"));
}

/// Every refusal the endpoint can make, including the contrast rule G4 lifted into `mjx-tokens`.
///
/// Each one leaves the file **byte for byte** what it was, which is the property that matters: a
/// refusal that had already written is not a refusal.
#[test]
fn every_bad_write_is_refused_and_the_file_is_untouched() {
    let path = temporary_source("refusals");
    let before = std::fs::read_to_string(&path).expect("the copy");
    let harness = Harness::new(path.clone());

    for (body, expected) in [
        (
            r##"{"value": "#000000"}"##,
            "the request named no `property`",
        ),
        (
            r##"{"property": "--color-ink"}"##,
            "the request named no `value`",
        ),
        (
            r##"{"property": "--not-a-token", "value": "#000000"}"##,
            "is not a design token this platform defines",
        ),
        (
            r##"{"property": "--color-ink", "value": "not a colour"}"##,
            "is not one",
        ),
        // **G4.** A text colour below 4.5 : 1 would be written by the editor and refused by
        // `cargo run -p xtask -- tokens` a build later, until the rule moved into `mjx-tokens`.
        (
            r##"{"property": "--color-ink", "value": "#9a9a9a"}"##,
            "WCAG AA minimum for body text",
        ),
        // The other half of the same rule: the background moving under a text colour that did not.
        (
            r##"{"property": "--color-paper", "value": "#111111"}"##,
            "The tag names the wrong surface",
        ),
        // Not JSON at all. The reader is deliberately not a JSON parser, and what it must do with
        // nonsense is refuse rather than find a key by accident.
        ("not json at all", "the request named no `property`"),
        ("", "the request named no `property`"),
    ] {
        let response = server::route(&harness, "POST", "/api/tokens", "", body.as_bytes());
        let answer = response.text_body();
        assert_eq!(
            response.status(),
            "200 OK",
            "a refusal is an answer, not a status"
        );
        assert!(
            answer.contains("\"ok\":false"),
            "`{body}` was accepted: {answer}"
        );
        assert!(
            answer.contains(expected),
            "`{body}` was refused without saying `{expected}`: {answer}"
        );
        assert_eq!(
            std::fs::read_to_string(&path).expect("the copy"),
            before,
            "`{body}` was refused and the file changed anyway"
        );
    }

    let _ = std::fs::remove_dir_all(path.parent().expect("a directory"));
}

// -------------------------------------------------------------------------------------------
// `read_request`, and the three leniencies it is written with
// -------------------------------------------------------------------------------------------

#[test]
fn a_request_is_read_as_a_method_a_target_and_a_body() {
    let request =
        parse("POST /api/tokens HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 5\r\n\r\nhello");
    assert_eq!(request.method, "POST");
    assert_eq!(request.target, "/api/tokens");
    assert_eq!(request.body, b"hello");

    let with_query = parse("GET /api/probe?entry=3&x=1 HTTP/1.1\r\n\r\n");
    assert_eq!(with_query.path_and_query(), ("/api/probe", "entry=3&x=1"));
    assert_eq!(
        parse("GET / HTTP/1.1\r\n\r\n").path_and_query(),
        ("/", ""),
        "a target with no query has an empty one rather than none"
    );
}

/// A `Content-Length` that is not a number reads as zero rather than refusing to answer.
#[test]
fn an_unreadable_content_length_reads_as_an_empty_body() {
    for header in [
        "Content-Length: banana",
        "Content-Length:",
        "Content-Length: -4",
    ] {
        let request = parse(&format!(
            "POST /api/tokens HTTP/1.1\r\n{header}\r\n\r\nbody"
        ));
        assert!(
            request.body.is_empty(),
            "`{header}` produced a {}-byte body",
            request.body.len()
        );
    }
    // The header name is matched case-insensitively, because HTTP says it is and a browser is free
    // to send any spelling.
    assert_eq!(
        parse("POST /x HTTP/1.1\r\ncOnTeNt-LeNgTh: 2\r\n\r\nhi").body,
        b"hi"
    );
}

/// **The body-length cap.** A declared body larger than `MAXIMUM_BODY` allocates the cap, not the
/// declaration — which is the whole reason the constant exists.
#[test]
fn a_body_larger_than_the_cap_is_capped_rather_than_allocated() {
    let oversized = "x".repeat(MAXIMUM_BODY + 4_096);
    let raw = format!("POST /api/tokens HTTP/1.1\r\nContent-Length: 4294967296\r\n\r\n{oversized}");
    let request = parse(&raw);
    assert_eq!(
        request.body.len(),
        MAXIMUM_BODY,
        "a four-gigabyte `Content-Length` must allocate the cap and nothing more"
    );
    // And a body at the cap is read whole, so the cap is a ceiling rather than a truncation of
    // everything.
    let exact = "y".repeat(64);
    let read = parse(&format!(
        "POST /x HTTP/1.1\r\nContent-Length: 64\r\n\r\n{exact}"
    ));
    assert_eq!(read.body.len(), 64);
}

/// A request line this server cannot read routes to the 404 rather than erroring.
///
/// A port scanner, a stray `\r\n` and a browser's speculative connection all arrive here, and an
/// error would print a line into the sitting's terminal for each of them.
#[test]
fn an_unreadable_request_line_becomes_a_404_rather_than_an_error() {
    let harness = harness();
    for raw in ["", "\r\n", "GARBAGE\r\n\r\n", "🙂\r\n\r\n"] {
        let request = server::read_request(&mut BufReader::new(raw.as_bytes()))
            .expect("no error is reported");
        let (path, query) = request.path_and_query();
        let response = server::route(&harness, &request.method, path, query, &request.body);
        assert_eq!(
            response.status(),
            "404 Not Found",
            "`{raw:?}` produced `{}`",
            response.status()
        );
    }
}

/// A body that stops short of its declared length is the one real input/output failure here.
#[test]
fn a_truncated_body_is_reported_rather_than_silently_short() {
    let raw = "POST /api/tokens HTTP/1.1\r\nContent-Length: 64\r\n\r\nonly ten..";
    let read = server::read_request(&mut BufReader::new(raw.as_bytes()));
    assert!(
        read.is_err(),
        "a connection that closed mid-body has to be reported, or the route would parse half a \
         request as a whole one: {read:?}"
    );
}
