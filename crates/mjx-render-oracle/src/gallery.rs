//! The document plate gallery: every plate beside its reference and its diff, **labelled with what
//! produced the reference and whether the comparison is evidence at all**.
//!
//! # Why a page rather than a log
//!
//! MJXOFF-165's constraint is short: *a failure a reviewer cannot see is a failure nobody fixes*.
//! Continuous integration can attach a directory; it cannot attach a person's attention. So the
//! gallery is one self-contained HTML file beside the PNGs, with no script, no stylesheet and no
//! network reference — a file that opens from a downloaded artefact directory on a machine with
//! nothing installed.
//!
//! It is also the **human approval surface**. `crate::baseline` can record that a person approved an
//! image; it cannot make looking at one possible. This is what a person looks at before running
//! `approve`, and every plate says on its own face whether anybody has.
//!
//! # ⚠ What the banner says, and why the wording is load-bearing
//!
//! Two claims this gallery must never make, and both were nearly true of it:
//!
//! * **Not parity.** Every reference available today is non-authoritative, so no row may be read as
//!   *"this matches PowerPoint"*. The banner says so, `parity` is `false` in the manifest beside it,
//!   and [`crate::authority::Baseline::may_be_called_parity`] is what refuses it in code.
//! * **Not "the shapes are wrong on purpose" any more.** The ticket that specified this gallery was
//!   written while every preset shape in the workspace was a stand-in, and told this child to say so
//!   in the gallery. **Phase G landed and that is no longer true**: all 186 published preset
//!   geometries resolve and draw, and the star on this page is the document's own outline. The
//!   banner therefore states the *measured* number — `DrawReport::placeholders`, per plate, summed —
//!   rather than a sentence that would have to be remembered and edited. A label that a gate does
//!   not compute is a label that is eventually wrong in one direction or the other.

use crate::authority::ReferenceProvider;
use crate::plate::PlateSet;

/// The gallery's file name inside the output directory.
pub const GALLERY_FILE: &str = "index.html";

/// Render `set` as one self-contained page.
#[must_use]
pub fn render(set: &PlateSet) -> String {
    let placeholders: usize = set.plates.iter().map(|plate| plate.placeholders).sum();
    let reviewed = set.plates.iter().filter(|plate| plate.reviewed).count();
    let regressed = set
        .plates
        .iter()
        .filter(|plate| plate.diff_file.is_some())
        .count();
    let excluded = set
        .plates
        .iter()
        .filter(|plate| plate.exclusion.is_some())
        .count();

    let mut html = String::with_capacity(8192);
    html.push_str(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <title>mjx-render-oracle — plate gallery</title>\n<style>\n",
    );
    html.push_str(STYLE);
    html.push_str("</style>\n</head>\n<body>\n");
    html.push_str("<h1>Plate gallery</h1>\n");

    html.push_str("<section class=\"banner\">\n");
    html.push_str(&format!(
        "<p><strong>This gallery is not a parity claim.</strong> Every reference available today \
         is non-authoritative — this run was made against <em>{}</em> — so no image below may be \
         read as “this matches PowerPoint”. Parity is judged against Microsoft Office on Windows, \
         later, and the manifest beside this file records <code>parity: false</code> for every \
         plate.</p>\n",
        escape(set.provider.label())
    ));
    html.push_str(&format!(
        "<p><strong>Stand-in geometry: {placeholders}.</strong> That is \
         <code>DrawReport::placeholders</code> summed over every plate, measured on this run rather \
         than asserted in prose. Zero means every shape below is the document's own outline; \
         anything else means at least one plate is a framed crossed rectangle standing in for a \
         shape, and should be read as such.</p>\n"
    ));
    html.push_str(&format!(
        "<p><strong>Human review: {reviewed} of {}.</strong> A baseline approved by the generator \
         is a real approval record — the digest binding is live, so a silent regeneration is \
         caught — and it is <em>not</em> a person having looked at the image. Look at a plate, then \
         run <code>MJX_ORACLE_APPROVED_BY='Your Name' cargo run -p mjx-render-oracle -- approve \
         &lt;plate&gt; \"why\"</code>.</p>\n",
        set.plates.len()
    ));
    if regressed > 0 {
        html.push_str(&format!(
            "<p><strong>Does not match its approved baseline: {regressed}.</strong> Each carries \
             the approved image and the amplified difference beside it, below. A difference is not \
             by itself a defect — a renderer improvement moves an image too — but it is something \
             somebody has to look at and then either approve or fix.</p>\n"
        ));
    }
    if excluded > 0 {
        html.push_str(&format!(
            "<p><strong>Excluded from the pixel tier: {excluded}.</strong> An excluded plate is \
             <em>not evidence</em> — neither a pass nor a failure. The exclusion belongs to the \
             provider, not to the plate, so it lifts by itself the day an Office export \
             arrives.</p>\n"
        ));
    }
    html.push_str("</section>\n");

    html.push_str("<h2>Plates</h2>\n<div class=\"plates\">\n");
    for plate in &set.plates {
        html.push_str("<figure class=\"plate\">\n");
        html.push_str(&format!(
            "<img src=\"{}\" width=\"{}\" height=\"{}\" alt=\"{}\">\n",
            escape(&plate.file),
            plate.width,
            plate.height,
            escape(&plate.description)
        ));
        html.push_str("<figcaption>\n");
        html.push_str(&format!("<h3>{}</h3>\n", escape(&plate.name)));
        html.push_str(&format!("<p>{}</p>\n", escape(&plate.description)));
        html.push_str("<dl>\n");
        row(&mut html, "content", plate.content.label());
        row(&mut html, "reference", plate.provider.label());
        row(
            &mut html,
            "placeholders",
            &format!(
                "{} (draw calls {}, covered pixels {})",
                plate.placeholders, plate.draw_calls, plate.covered
            ),
        );
        row(&mut html, "approved by", &plate.approver);
        row(
            &mut html,
            "human review",
            if plate.reviewed {
                "yes"
            } else {
                "no — nobody has looked at this image"
            },
        );
        row(&mut html, "parity", if plate.parity { "yes" } else { "no" });
        html.push_str("</dl>\n");
        if let Some(reason) = plate.exclusion {
            html.push_str(&format!(
                "<p class=\"excluded\"><strong>Not evidence.</strong> {}</p>\n",
                escape(reason)
            ));
        }
        if let (Some(reference), Some(diff), Some(difference)) = (
            plate.reference_file.as_deref(),
            plate.diff_file.as_deref(),
            plate.difference.as_deref(),
        ) {
            // **The three pictures, together, exactly when there is something to see.** A number in
            // a log is not a finding; the approved image beside the current one beside the amplified
            // difference is.
            html.push_str(&format!(
                "<p class=\"regressed\"><strong>This does not match the approved baseline.</strong> \
                 {}</p>\n\
                 <div class=\"pair\">\n\
                 <figure><img src=\"{}\" alt=\"the approved baseline\">\
                 <figcaption>approved</figcaption></figure>\n\
                 <figure><img src=\"{}\" alt=\"the difference, amplified\">\
                 <figcaption>difference &times;4</figcaption></figure>\n\
                 </div>\n",
                escape(difference),
                escape(reference),
                escape(diff),
            ));
        }
        html.push_str("</figcaption>\n</figure>\n");
    }
    html.push_str("</div>\n");

    html.push_str("<h2>Document fixtures</h2>\n");
    html.push_str(&format!(
        "<p>Every package in <code>mjx-fixtures</code>, derived from the corpus rather than listed \
         here — {} of them. None has a plate yet.</p>\n",
        set.documents.len()
    ));
    html.push_str(
        "<table>\n<tr><th>fixture</th><th>format</th><th>verdict</th><th>why</th></tr>\n",
    );
    for document in &set.documents {
        html.push_str(&format!(
            "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            escape(&document.fixture),
            escape(&document.format),
            escape(&document.verdict),
            escape(&document.reason)
        ));
    }
    html.push_str("</table>\n");

    if set.provider == ReferenceProvider::None {
        html.push_str(
            "<p class=\"footnote\">This run had no reference at all, so the plates are our own \
             output beside nothing. That is the honest state of the project: the reference is the \
             one thing an agent cannot produce.</p>\n",
        );
    }
    html.push_str("</body>\n</html>\n");
    html
}

/// One `<dt>`/`<dd>` pair.
fn row(html: &mut String, key: &str, value: &str) {
    html.push_str(&format!(
        "<dt>{}</dt><dd>{}</dd>\n",
        escape(key),
        escape(value)
    ));
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

/// The whole stylesheet, inline, so the page opens from a downloaded artefact directory with
/// nothing installed and no network.
const STYLE: &str = "\
:root { color-scheme: light dark; }
body { font: 15px/1.5 system-ui, sans-serif; margin: 0 auto; max-width: 60rem; padding: 2rem 1rem; }
h1 { font-size: 1.6rem; }
h2 { font-size: 1.2rem; margin-top: 2.5rem; }
h3 { font-size: 1rem; margin: 0 0 .3rem; }
.banner { border-left: 4px solid currentColor; padding: .25rem 0 .25rem 1rem; opacity: .95; }
.banner p { margin: .6rem 0; }
.plates { display: grid; gap: 1.5rem; grid-template-columns: repeat(auto-fill, minmax(20rem, 1fr)); }
.plate { margin: 0; border: 1px solid rgba(128,128,128,.4); border-radius: 6px; padding: .75rem; }
.plate img { display: block; width: 100%; height: auto; background: #fff;
  border: 1px solid rgba(128,128,128,.3); }
figcaption { margin-top: .6rem; }
figcaption p { margin: .3rem 0; }
dl { display: grid; grid-template-columns: max-content 1fr; gap: .1rem .6rem; margin: .5rem 0 0; }
dt { opacity: .7; }
dd { margin: 0; }
.excluded { border-left: 3px solid currentColor; padding-left: .6rem; opacity: .85; }
.regressed { border-left: 3px solid #c0392b; padding-left: .6rem; }
.pair { display: grid; gap: .5rem; grid-template-columns: 1fr 1fr; margin-top: .5rem; }
.pair figure { margin: 0; }
.pair figcaption { font-size: .8rem; opacity: .7; text-align: center; }
.pair img { display: block; width: 100%; height: auto; background: #fff;
  border: 1px solid rgba(128,128,128,.3); }
table { border-collapse: collapse; width: 100%; font-size: .9rem; }
th, td { border-bottom: 1px solid rgba(128,128,128,.3); padding: .35rem .5rem;
  text-align: left; vertical-align: top; }
.footnote { opacity: .75; margin-top: 2rem; }
";
