//! The external readers, and the loud named skip that stands in for one that is absent.
//!
//! # Why the tools are external
//!
//! MJXOFF-164 states it: *"comparing a PDF export against a raster render by rasterising the PDF
//! with our own code is self-referential and proves nothing."* Two of our own components agreeing
//! says our two components were written by the same person on the same afternoon. So the rasteriser
//! that turns a PDF into pixels is **poppler's**, the text extractor is poppler's, and the thing
//! that converts a `.pptx` to a PDF for the preliminary pass is LibreOffice's.
//!
//! That also buys the one property that makes *"pixel perfect against PowerPoint"* a coherent
//! phrase: `pdftoppm` rasterises **both** PDFs, so antialiasing, hinting and subpixel positioning
//! cancel, and a remaining difference is real.
//!
//! # The skip, and why it is loud
//!
//! A quietly-skipped case is indistinguishable from a check that never existed. So [`tool`] prints a
//! named line when a reader is missing and **fails** when [`REQUIRE_TOOLS`] is set, which is what
//! continuous integration sets — the same shape `MJX_REQUIRE_SCHEMA`, `MJX_REQUIRE_GPU` and
//! `MJX_REQUIRE_SOFFICE` already use in this repository.
//!
//! [`which`] searches `PATH` itself rather than trusting `command -v`, because `command` is a shell
//! builtin and not always an executable: a case that skipped because the *check* failed would be the
//! same defect as one that skipped because the tool was missing and said nothing.
//!
//! # The image format is PPM, on purpose
//!
//! `pdftoppm` writes a binary `P6` netpbm by default and a PNG with `-png`. This crate reads the
//! PPM. A PNG would need a decoder, and `mjx-paint` deliberately hand-writes a PNG **encoder** so
//! that `tiny-skia`'s would not put a decoder in the tree of a crate that decodes nothing; pulling
//! one in here to read our own rasteriser's output would undo that for no gain. A `P6` header is
//! three integers and a magic number.

use std::path::Path;
use std::process::Command;

/// Setting this turns a missing external reader into a failure rather than a named skip.
pub const REQUIRE_TOOLS: &str = "MJX_REQUIRE_TOOLS";

/// Setting this turns a missing LibreOffice into a failure rather than a named skip.
pub const REQUIRE_SOFFICE: &str = "MJX_REQUIRE_SOFFICE";

/// The dots per inch every raster in this crate is taken at.
///
/// Ninety-six, so a 960 × 540 point page becomes 1280 × 720 pixels — an integer in both axes, which
/// means a plate window whose corners are integer points has integer pixel corners too and no crop
/// straddles a pixel. A DPI that did not divide cleanly would put a fractional column at the edge of
/// every window and charge it to the renderers.
pub const RASTER_DPI: f64 = 96.0;

/// Whether `name` resolves on `PATH`.
#[must_use]
pub fn which(name: &str) -> bool {
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|directory| directory.join(name).is_file())
}

/// Whether `name` is available, announcing a loud named skip when it is not.
///
/// # Panics
///
/// When the tool is absent and the environment variable `require` is set — which is what turns an
/// absence into a failure in continuous integration.
#[must_use]
pub fn tool(case: &str, name: &str, require: &str) -> bool {
    let found = which(name);
    if !found {
        let message = format!(
            "SKIPPED {case}: `{name}` is not on the path. This needs a reader **this workspace did \
             not write** — checking our own export with our own reader proves nothing — so set \
             {require}=1 to make its absence a failure instead of a skip."
        );
        assert!(
            std::env::var(require).is_err(),
            "{require} is set, so a missing external tool is a failure and not a skip. {message}"
        );
        println!("{message}");
    }
    found
}

/// A raster: straight (not premultiplied) `RGB`, three bytes a pixel, row zero at the top.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Raster {
    /// Pixels across.
    pub width: u32,
    /// Pixels down.
    pub height: u32,
    /// `width * height * 3` bytes.
    pub rgb: Vec<u8>,
}

impl Raster {
    /// The pixel at (`x`, `y`), or `None` off the edge.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 3]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let offset = ((y as usize) * (self.width as usize) + (x as usize)) * 3;
        self.rgb
            .get(offset..offset + 3)
            .map(|slice| [slice[0], slice[1], slice[2]])
    }

    /// How many pixels are not white — the raster's *ink*.
    ///
    /// This is what stops a comparison going vacuous: two blank crops agree perfectly, so a caller
    /// asserts that at least one side has ink before believing an agreement.
    #[must_use]
    pub fn ink(&self) -> usize {
        self.rgb
            .chunks_exact(3)
            .filter(|pixel| pixel.iter().any(|channel| *channel < 250))
            .count()
    }
}

/// Parse a binary `P6` netpbm.
///
/// # Errors
///
/// A sentence naming what was wrong with the header or the payload. `pdftoppm`'s output is not
/// untrusted input in the security sense, but it *is* another program's output, and a silent
/// mis-parse would be charged to a renderer.
pub fn parse_ppm(bytes: &[u8]) -> Result<Raster, String> {
    let mut cursor = 0usize;
    let mut token = || -> Result<String, String> {
        // A netpbm header is whitespace-separated tokens with `#` comments to end of line.
        loop {
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor] == b'#' {
                while cursor < bytes.len() && bytes[cursor] != b'\n' {
                    cursor += 1;
                }
                continue;
            }
            break;
        }
        let start = cursor;
        while cursor < bytes.len() && !bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if start == cursor {
            return Err("the PPM header ended early".to_owned());
        }
        String::from_utf8(bytes[start..cursor].to_vec())
            .map_err(|_| "the PPM header is not ASCII".to_owned())
    };

    let magic = token()?;
    if magic != "P6" {
        return Err(format!("expected a binary PPM (`P6`), got `{magic}`"));
    }
    let width: u32 = token()?
        .parse()
        .map_err(|_| "the PPM width is not a number".to_owned())?;
    let height: u32 = token()?
        .parse()
        .map_err(|_| "the PPM height is not a number".to_owned())?;
    let maximum: u32 = token()?
        .parse()
        .map_err(|_| "the PPM maximum is not a number".to_owned())?;
    if maximum != 255 {
        return Err(format!("expected an 8-bit PPM, got a maximum of {maximum}"));
    }
    // Exactly one whitespace byte separates the header from the payload.
    let start = cursor + 1;
    let wanted = (width as usize) * (height as usize) * 3;
    let payload = bytes.get(start..start + wanted).ok_or_else(|| {
        format!(
            "the PPM claims {width}x{height} and carries {} bytes",
            bytes.len().saturating_sub(start)
        )
    })?;
    Ok(Raster {
        width,
        height,
        rgb: payload.to_vec(),
    })
}

/// Rasterise one page of `pdf` with poppler, at [`RASTER_DPI`].
///
/// # Errors
///
/// A sentence naming what `pdftoppm` said. `page` is one-based, as poppler numbers pages.
pub fn rasterise(pdf: &Path, page: usize) -> Result<Raster, String> {
    let output = Command::new("pdftoppm")
        .arg("-r")
        .arg(format!("{RASTER_DPI}"))
        .arg("-f")
        .arg(page.to_string())
        .arg("-l")
        .arg(page.to_string())
        .arg(pdf)
        .output()
        .map_err(|error| format!("running pdftoppm: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "pdftoppm refused {} page {page}: {}",
            pdf.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    parse_ppm(&output.stdout).map_err(|reason| {
        format!(
            "pdftoppm's output for {} page {page} could not be read: {reason}",
            pdf.display()
        )
    })
}

/// How many pages `pdf` has, according to `pdfinfo`.
///
/// # Errors
///
/// A sentence naming what `pdfinfo` said.
pub fn page_count(pdf: &Path) -> Result<usize, String> {
    let output = Command::new("pdfinfo")
        .arg(pdf)
        .output()
        .map_err(|error| format!("running pdfinfo: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "pdfinfo refused {}: {}",
            pdf.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("Pages:"))
        .and_then(|rest| rest.trim().parse().ok())
        .ok_or_else(|| format!("pdfinfo reported no page count for {}", pdf.display()))
}

/// Convert `source` to a PDF beside itself in `directory`, using LibreOffice.
///
/// Answers the path of the PDF it wrote.
///
/// # Errors
///
/// A sentence naming what `soffice` said, or that it wrote nothing.
pub fn convert_to_pdf(source: &Path, directory: &Path) -> Result<std::path::PathBuf, String> {
    let output = Command::new("soffice")
        .arg("--headless")
        .arg("--convert-to")
        .arg("pdf")
        .arg("--outdir")
        .arg(directory)
        .arg(source)
        .output()
        .map_err(|error| format!("running soffice: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "soffice refused {}: {}",
            source.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let written = directory.join(
        Path::new(source.file_stem().ok_or("the source has no file name")?).with_extension("pdf"),
    );
    if !written.is_file() {
        return Err(format!(
            "soffice reported success and wrote no {}: {}",
            written.display(),
            String::from_utf8_lossy(&output.stdout)
        ));
    }
    Ok(written)
}

/// One word, and where poppler says it landed, in PDF points with the origin at the page's top-left.
#[derive(Clone, PartialEq, Debug)]
pub struct WordBox {
    /// The page it is on, one-based.
    pub page: usize,
    /// The text.
    pub text: String,
    /// Points from the left edge.
    pub x_min: f64,
    /// Points from the top edge.
    pub y_min: f64,
    /// Points from the left edge to the word's right side.
    pub x_max: f64,
    /// Points from the top edge to the word's bottom.
    pub y_max: f64,
}

impl WordBox {
    /// Whether the word's centre falls inside `rect`.
    ///
    /// Centre rather than overlap: a probe rectangle abuts its neighbour, and a word that grazed the
    /// boundary would otherwise be counted twice.
    #[must_use]
    pub fn is_inside(&self, rect: crate::layout::Rect) -> bool {
        let (x, y) = (
            (self.x_min + self.x_max) / 2.0,
            (self.y_min + self.y_max) / 2.0,
        );
        #[allow(
            clippy::cast_precision_loss,
            reason = "layout rectangles are small integer point counts"
        )]
        let (left, top) = (rect.x as f64, rect.y as f64);
        #[allow(
            clippy::cast_precision_loss,
            reason = "layout rectangles are small integer point counts"
        )]
        let (right, bottom) = ((rect.x + rect.width) as f64, (rect.y + rect.height) as f64);
        x >= left && x < right && y >= top && y < bottom
    }
}

/// Every word in `pdf`, with its bounding box, via `pdftotext -bbox-layout`.
///
/// # The tier this is
///
/// **The strong one**, and the one no exclusion touches. Word boxes are exact, rasteriser-independent
/// and diagnosable: a line-break divergence names the word rather than smearing grey over a diff
/// image, and a word's box says nothing whatever about how the shape behind it is filled — so it
/// stays meaningful on exactly the content whose pixel tier is compromised.
///
/// # Errors
///
/// A sentence naming what `pdftotext` said, or what was wrong with its output.
pub fn word_boxes(pdf: &Path) -> Result<Vec<WordBox>, String> {
    let output = Command::new("pdftotext")
        .arg("-bbox-layout")
        .arg(pdf)
        .arg("-")
        .output()
        .map_err(|error| format!("running pdftotext: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "pdftotext refused {}: {}",
            pdf.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(parse_bbox_layout(&String::from_utf8_lossy(&output.stdout)))
}

/// Pull `<word …>` elements out of `pdftotext -bbox-layout`'s XHTML.
///
/// A hand-rolled scan rather than an XML parser, for the reason `CLAUDE.md` gives: `quick-xml` lives
/// only behind `mjx-xml`, and reaching for `mjx-xml`'s *fidelity* parser to read another program's
/// diagnostic output would put a document model in front of six numbers. The shape being read is
/// poppler's own generated markup — one element per line, four numeric attributes in a fixed order —
/// and [`crate::compare`]'s own tests hold it to a recorded sample so the scan cannot quietly stop
/// matching.
#[must_use]
pub fn parse_bbox_layout(xhtml: &str) -> Vec<WordBox> {
    let mut words = Vec::new();
    let mut page = 0usize;
    for line in xhtml.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("<page ") {
            page += 1;
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("<word ") else {
            continue;
        };
        let Some(x_min) = attribute(rest, "xMin") else {
            continue;
        };
        let (Some(y_min), Some(x_max), Some(y_max)) = (
            attribute(rest, "yMin"),
            attribute(rest, "xMax"),
            attribute(rest, "yMax"),
        ) else {
            continue;
        };
        let text = rest
            .split_once('>')
            .map(|(_, tail)| tail)
            .and_then(|tail| tail.split_once("</word>"))
            .map(|(text, _)| unescape(text))
            .unwrap_or_default();
        words.push(WordBox {
            page: page.max(1),
            text,
            x_min,
            y_min,
            x_max,
            y_max,
        });
    }
    words
}

/// One `name="12.34"` attribute out of an element's attribute run.
fn attribute(source: &str, name: &str) -> Option<f64> {
    let needle = format!("{name}=\"");
    let start = source.find(&needle)? + needle.len();
    let rest = &source[start..];
    let end = rest.find('"')?;
    rest[..end].parse().ok()
}

/// The five XML entities poppler writes.
fn unescape(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}
