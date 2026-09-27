//! **The page draws nothing**, reaches nothing, and is driven by the generated tokens.
//!
//! The user's standing architecture constraint for the client platform is short: *the document
//! renderer is entirely Rust, with no JavaScript graphics library; the chrome is TypeScript/HTML
//! driven by the generated design tokens.* Three of those clauses are checkable by inspection of
//! one string, and this suite is that inspection — because a constraint that is only documented is
//! one the next child breaks by accident.
//!
//! * **No drawing.** No `<canvas>`, no `getContext`, no `getImageData`, no SVG generation. The
//!   element under audit is an `<img>` whose bytes came out of the software painter, and the pixel
//!   probe is an endpoint answered from the same `Pixels` the PNG was encoded from.
//! * **No network.** No `http://`, no `https://`, no CDN, no import from anywhere. The page is
//!   served from a local process to a browser on the same machine or the same phone, and a harness
//!   that fetched a library would be one that stops working on an aeroplane.
//! * **Driven by the tokens.** Every custom property is written from `mjx_tokens::TOKENS` — the
//!   same table `ui/tokens/tokens.css` is emitted from — so the chrome and the canvas are
//!   demonstrably reading one set of values.

use mjx_canvas_harness::inventory::INVENTORY;
use mjx_canvas_harness::page;
use mjx_tokens::Tokens;

fn rendered() -> String {
    page::render(&Tokens::DEFAULTS.clone())
}

/// Whether `needle` appears in `html` as a **whole identifier** rather than as a substring.
///
/// ⚠ **This exists because `d3` is also the tail of a colour, and MJXOFF-271's palette re-seed made
/// one.** `--theme-light-border-subtle` became `#eae3d3`, and a bare `html.contains("d3")` read the
/// last two characters of a hexadecimal literal as the D3 charting library and failed. Loosening
/// the ban would have been the wrong repair — the constraint it enforces is the user's own, *no
/// JavaScript graphics library* — so the check was made **precise** instead: a library reference is
/// an identifier with a boundary either side (`d3.select`, `import d3`), and a hex digit run is not.
fn contains_identifier(html: &str, needle: &str) -> bool {
    let boundary = |character: Option<char>| {
        character.is_none_or(|character| !character.is_ascii_alphanumeric() && character != '_')
    };
    let mut from = 0;
    while let Some(offset) = html[from..].find(needle) {
        let at = from + offset;
        let before = html[..at].chars().next_back();
        let after = html[at + needle.len()..].chars().next();
        if boundary(before) && boundary(after) {
            return true;
        }
        from = at + needle.len();
    }
    false
}

#[test]
fn the_page_draws_nothing_of_its_own() {
    let html = rendered();
    // Markup and member names: these cannot occur inside a token value, so a plain substring is
    // exactly the right test for them.
    for forbidden in [
        "<canvas",
        "getContext",
        "getImageData",
        "createElementNS",
        "<svg",
    ] {
        assert!(
            !html.contains(forbidden),
            "the harness page contains `{forbidden}`. The document renderer is entirely Rust: the \
             element under audit is an `<img>` whose bytes came out of `mjx-paint`'s software \
             painter, and anything the page drew for itself would be a second renderer nobody is \
             auditing."
        );
    }
    // Library names, which are identifiers — see `contains_identifier`.
    for forbidden in ["d3", "chart", "chartjs", "plotly", "echarts"] {
        assert!(
            !contains_identifier(&html, forbidden),
            "the harness page names `{forbidden}`. The document renderer is entirely Rust with no \
             JavaScript graphics library, and anything the page drew for itself would be a second \
             renderer nobody is auditing."
        );
    }
    // The check can still fail — proved rather than assumed, because an identifier-aware search
    // that never matched would pass every page ever written.
    assert!(
        contains_identifier("<script>d3.select('#x')</script>", "d3"),
        "the identifier search must still find a real library reference"
    );
    assert!(
        !contains_identifier("--theme-border-subtle: #eae3d3;", "d3"),
        "…and must not find one in the tail of a hexadecimal colour"
    );
    assert!(
        html.contains("id=\"element\""),
        "the page has no element to show"
    );
}

#[test]
fn the_page_reaches_nothing_outside_this_process() {
    let html = rendered();
    for forbidden in ["http://", "https://", "//cdn", "unpkg", "jsdelivr", "<link"] {
        assert!(
            !html.contains(forbidden),
            "the harness page references `{forbidden}`. It is served from a local process to a \
             browser that may be a phone on a home network, and a page that fetched a library is a \
             page that stops working when the network does."
        );
    }
    // Every fetch it makes is to its own origin, and every one of those routes exists.
    for route in [
        "/render.png?",
        "/api/inventory",
        "/api/counters?",
        "/api/probe?",
        "/api/tokens",
    ] {
        assert!(
            html.contains(route),
            "the page does not reach `{route}`, so either the route is unused or the panel is \
             showing something stale"
        );
    }
}

/// **The page says the sixty-one designs are proposals** (audit pass 10, G9).
///
/// The gallery says it at the top, `docs/client-platform/CANVAS_UI_AUDIT.md` says
/// `Human review: 0 of 61` on its own face, the CI job's comment says it, and `check` says it since
/// this same pass — and the one surface a person actually *audits on* said nothing. That is the
/// surface where it matters most: a reviewer who assumes these are settled designs will audit them
/// as though they were, and the whole point of MJXOFF-166 is that the approval is theirs to give.
#[test]
fn the_page_says_nobody_has_approved_these_designs() {
    let html = rendered();
    for phrase in [
        "proposals awaiting your pass",
        "nobody has looked at",
        "approver = generator",
        "approve",
    ] {
        assert!(
            html.contains(phrase),
            "the harness page no longer says `{phrase}`. The gallery, the checklist, the CI job \
             and `check` all disclose that the sixty-one plates carry no human review; this is the \
             page the review actually happens on, and it may not be the one that stays quiet."
        );
    }
    assert!(
        html.contains("id=\"unapproved\""),
        "the banner has no stable id, so nothing can refer to it"
    );
    // And it names the real count, from the inventory rather than from a literal in the markup.
    assert!(
        html.contains(&format!("These {} designs", INVENTORY.len())),
        "the banner should name the inventory's own count"
    );
}

#[test]
fn the_page_is_driven_by_the_generated_tokens() {
    let html = rendered();
    let tokens = Tokens::DEFAULTS.clone();
    let mut declared = 0_usize;
    for identity in mjx_tokens::TOKENS {
        let Some(value) = tokens.custom_property(identity.custom_property) else {
            continue;
        };
        let declaration = format!("  {}: {};", identity.custom_property, value);
        assert!(
            html.contains(&declaration),
            "the page does not declare `{}`, so the chrome and the canvas are not reading one set \
             of values",
            identity.custom_property
        );
        declared += 1;
    }
    assert_eq!(
        declared,
        mjx_tokens::TOKENS.len(),
        "the page declares {declared} of the platform's {} tokens",
        mjx_tokens::TOKENS.len()
    );
    // And it *uses* them: a page that declared every custom property and then wrote its own
    // colours would pass the assertion above and be exactly as wrong.
    for used in [
        "var(--theme-light-background)",
        "var(--theme-dark-background)",
        "var(--font-sans)",
        "var(--radius-control)",
    ] {
        assert!(
            html.contains(used),
            "the page declares tokens and does not use `{used}`"
        );
    }
}

#[test]
fn every_element_is_reachable_from_the_page() {
    let html = rendered();
    for entry in &INVENTORY {
        assert!(
            html.contains(&format!("data-entry=\"{}\"", entry.number)),
            "entry {} (`{}`) has no button in the scene list, so it cannot be audited",
            entry.number,
            entry.title
        );
        assert!(
            html.contains(&mjx_canvas_harness::page::escape(entry.title)),
            "entry {}'s title is not on the page",
            entry.number
        );
    }
    assert_eq!(
        html.matches("data-entry=").count(),
        INVENTORY.len(),
        "the scene list should carry exactly one button per element"
    );
    // The state panel offers every value of every axis, or a state is one nobody can reach by
    // clicking — which is the thing `CANVAS_UI_INVENTORY.md` §3 asks the panel to make possible.
    for value in [
        "default", "hover", "active", "focused", "disabled", "light", "dark", "1x", "2x", "3x",
        "pointer", "touch",
    ] {
        assert!(
            html.contains(&format!("value=\"{value}\"")),
            "the state panel has no control for `{value}`"
        );
    }
}

#[test]
fn a_query_string_names_a_state_and_a_wrong_one_does_not_pretend_to() {
    use mjx_canvas_harness::state::{Density, Input, Interaction, State};
    let state =
        page::state_from_query("entry=3&interaction=active&scheme=dark&density=3x&input=touch");
    assert_eq!(state.interaction, Interaction::Active);
    assert_eq!(state.density, Density::Three);
    assert_eq!(state.input, Input::Touch);
    assert_eq!(mjx_canvas_harness::state::scheme_slug(state.scheme), "dark");
    assert_eq!(state.slug(), "active-dark-3x-touch");

    // A value the harness does not know falls back rather than failing — and the slug it echoes
    // back into the counters panel is what makes the fallback visible instead of silent.
    let fallback = page::state_from_query("interaction=nonsense&density=11x");
    assert_eq!(fallback, State::CANONICAL);
    assert_eq!(fallback.slug(), "default-light-1x-pointer");
}
