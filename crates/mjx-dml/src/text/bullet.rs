//! Bullets and numbering — the four `EG_TextBullet*` choice groups of a paragraph's properties.
//!
//! A deck's structure is paragraphs at indent levels, and what a reader sees as a bulleted list is a
//! paragraph whose level and bullet were set. So bullets live with [paragraph
//! properties](super::paragraph_properties), not off to one side.
//!
//! # Four groups, inherited independently
//!
//! The schema keeps the bullet, its colour, its size and its typeface in **separate choice groups**,
//! and each inherits on its own: a list style may set the character at one level while leaving the
//! colour to be inherited from another. They are therefore four independent optional fields, never one
//! bundled value.
//!
//! # Absent is not "follow the text"
//!
//! Each group has a *"follow the text"* arm — `a:buClrTx`, `a:buSzTx`, `a:buFontTx` — which says
//! "match whatever the text does". That is a **decision**, and quite different from the group being
//! absent, which means "inherit whatever the level above decided". Hence [`BulletColor::FollowText`]
//! and friends are real variants rather than `None`.

use mjx_ooxml_core::{Interner, RawElement, RawNode};

use crate::build::{attr_by_local, attr_str, dml_attr, dml_child, dml_element, prefixed_attr};
use crate::color::{Color, ColorSpec};
use crate::geometry::{FontSize, Fraction};
use crate::text::font::TextFont;

pub use mjx_ooxml_types::drawingml::AutonumberScheme;

/// What marks a paragraph (`EG_TextBullet`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bullet {
    /// `a:buNone` — no bullet at all. An explicit decision that overrides an inherited bullet, which
    /// is why it is a variant rather than the absence of one.
    None,
    /// `a:buChar` — a literal character, the common case (`•`, `–`, `▪`).
    Character(BulletCharacter),
    /// `a:buAutoNum` — an automatically maintained number or letter.
    AutoNumber(AutoNumberBullet),
    /// `a:buBlip` — an image.
    Picture(BulletPicture),
}

/// A literal bullet character (`a:buChar@char`).
///
/// Holds a `String`, not a `char`: the attribute is an `xsd:string`, and real bullets include glyphs
/// that are more than one code point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulletCharacter {
    /// The character(s) drawn as the bullet.
    pub character: String,
}

impl BulletCharacter {
    /// A bullet drawn as `character`.
    #[must_use]
    pub fn new(character: &str) -> Self {
        Self {
            character: character.to_owned(),
        }
    }
}

/// An automatically numbered bullet (`a:buAutoNum`) — the scheme, and where its sequence starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoNumberBullet {
    /// The numbering scheme (`@type`) — which numerals, and how they are punctuated.
    pub scheme: AutonumberScheme,
    /// The number the sequence starts at (`@startAt`, 1..=32767). The schema's default is `1`.
    pub start_at: u32,
}

impl AutoNumberBullet {
    /// The schema default for `@startAt`.
    pub const DEFAULT_START: u32 = 1;

    /// A sequence in `scheme`, starting at 1.
    #[must_use]
    pub fn new(scheme: AutonumberScheme) -> Self {
        Self {
            scheme,
            start_at: Self::DEFAULT_START,
        }
    }

    /// The same sequence, starting at `start_at` instead — for a list continuing an earlier one.
    #[must_use]
    pub fn starting_at(mut self, start_at: u32) -> Self {
        self.start_at = start_at;
        self
    }
}

/// An image bullet (`a:buBlip` → `a:blip`), referenced by relationship id.
///
/// As with a picture fill, the relationship and the image part are the caller's to add — this only
/// records the reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulletPicture {
    /// The embedded image's relationship id (`a:blip@r:embed`), resolved against the containing part.
    pub image_rel_id: String,
}

impl BulletPicture {
    /// A bullet drawn from the image at `image_rel_id`.
    #[must_use]
    pub fn new(image_rel_id: &str) -> Self {
        Self {
            image_rel_id: image_rel_id.to_owned(),
        }
    }
}

/// What colour a bullet is drawn in (`EG_TextBulletColor`).
#[derive(Debug, Clone, PartialEq)]
pub enum BulletColor {
    /// `a:buClrTx` — whatever colour the text is.
    FollowText,
    /// `a:buClr` — a colour of its own.
    Explicit(ColorSpec),
}

/// How large a bullet is drawn (`EG_TextBulletSize`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BulletSize {
    /// `a:buSzTx` — whatever size the text is.
    FollowText,
    /// `a:buSzPct` — a proportion of the text size, between 25% and 400%.
    Percentage(Fraction),
    /// `a:buSzPts` — a fixed size, independent of the text.
    Points(FontSize),
}

impl BulletSize {
    /// A size given as a proportion of the text size (`1.11` = 111%).
    #[must_use]
    pub fn percentage(proportion: f64) -> Self {
        Self::Percentage(Fraction::from_ratio(proportion))
    }

    /// A fixed size in points.
    #[must_use]
    pub fn points(points: f64) -> Self {
        Self::Points(FontSize::from_points(points))
    }
}

/// What font a bullet is drawn in (`EG_TextBulletTypeface`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BulletTypeface {
    /// `a:buFontTx` — whatever font the text uses.
    FollowText,
    /// `a:buFont` — a font of its own. Character bullets usually name one (`Wingdings`, `Arial`),
    /// since the glyph has to exist in it.
    Explicit(TextFont),
}

impl BulletTypeface {
    /// A named typeface for the bullet.
    #[must_use]
    pub fn named(typeface: &str) -> Self {
        Self::Explicit(TextFont::named(typeface))
    }
}

// ---------------------------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------------------------

/// Whether `local` names one of the `EG_TextBullet` elements.
pub(crate) fn is_bullet_local(local: &str) -> bool {
    matches!(local, "buNone" | "buChar" | "buAutoNum" | "buBlip")
}

/// Whether `local` names one of the `EG_TextBulletColor` elements.
pub(crate) fn is_bullet_color_local(local: &str) -> bool {
    matches!(local, "buClrTx" | "buClr")
}

/// Whether `local` names one of the `EG_TextBulletSize` elements.
pub(crate) fn is_bullet_size_local(local: &str) -> bool {
    matches!(local, "buSzTx" | "buSzPct" | "buSzPts")
}

/// Whether `local` names one of the `EG_TextBulletTypeface` elements.
pub(crate) fn is_bullet_typeface_local(local: &str) -> bool {
    matches!(local, "buFontTx" | "buFont")
}

/// Reads the `EG_TextBullet` group from a paragraph properties element's children.
pub(crate) fn read_bullet(children: &[RawNode], interner: &Interner) -> Option<Bullet> {
    let element = find_group(children, interner, is_bullet_local)?;
    Some(match interner.resolve(element.name.local) {
        "buNone" => Bullet::None,
        "buChar" => Bullet::Character(BulletCharacter {
            character: attr_str(&element.attributes, interner, "char")
                .unwrap_or_default()
                .to_owned(),
        }),
        "buAutoNum" => {
            let scheme = attr_str(&element.attributes, interner, "type")
                .and_then(AutonumberScheme::from_wire)?;
            let start_at = attr_str(&element.attributes, interner, "startAt")
                .and_then(|s| s.trim().parse::<u32>().ok())
                .unwrap_or(AutoNumberBullet::DEFAULT_START);
            Bullet::AutoNumber(AutoNumberBullet { scheme, start_at })
        }
        "buBlip" => {
            let blip = dml_child(&element.children, interner, "blip")?;
            Bullet::Picture(BulletPicture {
                image_rel_id: attr_by_local(&blip.attributes, interner, "embed")
                    .unwrap_or_default()
                    .to_owned(),
            })
        }
        _ => return None,
    })
}

/// Reads the `EG_TextBulletColor` group.
pub(crate) fn read_bullet_color(children: &[RawNode], interner: &Interner) -> Option<BulletColor> {
    let element = find_group(children, interner, is_bullet_color_local)?;
    match interner.resolve(element.name.local) {
        "buClrTx" => Some(BulletColor::FollowText),
        "buClr" => crate::build::first_color_child(element, interner)
            .map(|color| BulletColor::Explicit(color.spec(interner))),
        _ => None,
    }
}

/// Reads the `EG_TextBulletSize` group.
///
/// `a:buSzPct@val` is spelled `"111%"` by the schema; the integer form (`111000`) is accepted too,
/// since it appears in the wild. [`crate::build::parse_percentage`] handles both.
pub(crate) fn read_bullet_size(children: &[RawNode], interner: &Interner) -> Option<BulletSize> {
    let element = find_group(children, interner, is_bullet_size_local)?;
    match interner.resolve(element.name.local) {
        "buSzTx" => Some(BulletSize::FollowText),
        "buSzPct" => attr_str(&element.attributes, interner, "val")
            .and_then(crate::build::parse_percentage)
            .map(BulletSize::Percentage),
        "buSzPts" => attr_str(&element.attributes, interner, "val")
            .and_then(|s| s.trim().parse::<i32>().ok())
            .map(|value| BulletSize::Points(FontSize::from_wire(value))),
        _ => None,
    }
}

/// Reads the `EG_TextBulletTypeface` group.
pub(crate) fn read_bullet_typeface(
    children: &[RawNode],
    interner: &Interner,
) -> Option<BulletTypeface> {
    let element = find_group(children, interner, is_bullet_typeface_local)?;
    match interner.resolve(element.name.local) {
        "buFontTx" => Some(BulletTypeface::FollowText),
        "buFont" => Some(BulletTypeface::Explicit(TextFont::read(element, interner))),
        _ => None,
    }
}

/// The first child element whose local name satisfies `matches`.
fn find_group<'a>(
    children: &'a [RawNode],
    interner: &Interner,
    matches: impl Fn(&str) -> bool,
) -> Option<&'a RawElement> {
    children.iter().find_map(|node| match node {
        RawNode::Element(child)
            if crate::build::is_dml(&child.name, interner)
                && matches(interner.resolve(child.name.local)) =>
        {
            Some(child)
        }
        _ => None,
    })
}

// ---------------------------------------------------------------------------------------------
// Building
// ---------------------------------------------------------------------------------------------

/// Builds the `EG_TextBullet` element for a bullet.
pub(crate) fn build_bullet(interner: &mut Interner, bullet: &Bullet) -> RawElement {
    match bullet {
        Bullet::None => dml_element(interner, "buNone", Vec::new(), Vec::new()),
        Bullet::Character(character) => {
            let attributes = vec![dml_attr(interner, "char", &character.character)];
            dml_element(interner, "buChar", attributes, Vec::new())
        }
        Bullet::AutoNumber(auto) => {
            let mut attributes = vec![dml_attr(interner, "type", auto.scheme.to_wire())];
            // The schema's default is 1, so writing it would be noise.
            if auto.start_at != AutoNumberBullet::DEFAULT_START {
                attributes.push(dml_attr(interner, "startAt", &auto.start_at.to_string()));
            }
            dml_element(interner, "buAutoNum", attributes, Vec::new())
        }
        Bullet::Picture(picture) => {
            let embed = prefixed_attr(interner, "r", "embed", &picture.image_rel_id);
            let blip = dml_element(interner, "blip", vec![embed], Vec::new());
            dml_element(interner, "buBlip", Vec::new(), vec![RawNode::Element(blip)])
        }
    }
}

/// Builds the `EG_TextBulletColor` element, or `None` if the colour cannot be rebuilt.
pub(crate) fn build_bullet_color(
    interner: &mut Interner,
    color: &BulletColor,
) -> Option<RawElement> {
    match color {
        BulletColor::FollowText => Some(dml_element(interner, "buClrTx", Vec::new(), Vec::new())),
        BulletColor::Explicit(spec) => {
            let color = Color::from_spec(interner, spec)?;
            let child = mjx_ooxml_core::ToXml::to_xml(&color, interner);
            Some(dml_element(
                interner,
                "buClr",
                Vec::new(),
                vec![RawNode::Element(child)],
            ))
        }
    }
}

/// Builds the `EG_TextBulletSize` element.
///
/// A percentage is written in the form the schema specifies and ECMA §21.1.2.4.9 illustrates —
/// `val="111%"` — not the integer spelling.
pub(crate) fn build_bullet_size(interner: &mut Interner, size: BulletSize) -> RawElement {
    match size {
        BulletSize::FollowText => dml_element(interner, "buSzTx", Vec::new(), Vec::new()),
        BulletSize::Percentage(fraction) => {
            let percent = (fraction.ratio() * 100.0).round() as i64;
            let attributes = vec![dml_attr(interner, "val", &format!("{percent}%"))];
            dml_element(interner, "buSzPct", attributes, Vec::new())
        }
        BulletSize::Points(size) => {
            let attributes = vec![dml_attr(interner, "val", &size.to_wire().to_string())];
            dml_element(interner, "buSzPts", attributes, Vec::new())
        }
    }
}

/// Builds the `EG_TextBulletTypeface` element.
pub(crate) fn build_bullet_typeface(
    interner: &mut Interner,
    typeface: &BulletTypeface,
) -> RawElement {
    match typeface {
        BulletTypeface::FollowText => dml_element(interner, "buFontTx", Vec::new(), Vec::new()),
        BulletTypeface::Explicit(font) => font.build(interner, "buFont"),
    }
}
