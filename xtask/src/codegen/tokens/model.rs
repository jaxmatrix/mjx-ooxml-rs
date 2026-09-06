//! The design-token source, read and checked.
//!
//! `docs/client-platform/data/tokens.json` is the only hand-edited file in the token pipeline. This
//! module turns it into a tree the three emitters walk, and — far more importantly — it is where
//! every rule that the source could break is a *hard error* rather than a comment nobody reads.
//!
//! # The rules, and why each one is a failure and not a warning
//!
//! - **Every colour token declares its usage.** `DESIGN_TOKENS.md` §2.2 measures `--color-green` at
//!   3.39 : 1 on white — legal for a fill, illegal for body text — and calls getting it backwards
//!   *"the most likely accessibility defect in the chrome"*. A default of `fill-only` for an
//!   untagged colour would make the omission invisible, so there is no default.
//! - **A token tagged for text meets 4.5 : 1 against its declared background**, and is opaque. This
//!   is the rule the ticket exists to enforce: tag `--color-green` `on-light-text` and this
//!   generator refuses, quoting the measured ratio.
//! - **`on-light-text` really is on a light background** (and `on-dark-text` on a dark one). A tag
//!   that named the wrong surface would pass the ratio check against the wrong colour.
//! - **Two groups that claim the same Rust type carry the same members.** `theme.light` and
//!   `theme.dark` both become `ThemeColors`; a semantic role present in one scheme and absent from
//!   the other is a component that cannot be themed, and it would otherwise surface as a confusing
//!   duplicate-definition error in the emitted Rust instead of here.
//! - **No two tokens reach the same CSS custom property**, the scheme layer's aliases included.
//!   Silent collision is how one token would quietly overwrite another in exactly one of the three
//!   artefacts.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;

use anyhow::{bail, Context, Result};

use crate::json;

/// The extension key this project's metadata lives under, inside a token's or group's
/// `$extensions`. The W3C format reserves `$extensions` for exactly this.
const EXTENSION_KEY: &str = "mjx";

/// The WCAG 2.2 AA contrast minimum for body text. Large text and UI components have lower minima,
/// and this generator deliberately does not model them: a token tagged for *text* is tagged for the
/// hardest case it will be put to, and a second, laxer tier would only be a way to pass.
const TEXT_CONTRAST_MINIMUM: f64 = 4.5;

/// The relative luminance either side of which a surface counts as light or dark, used to check
/// that an `on-light-text` tag actually names a light background.
const LIGHT_SURFACE_LUMINANCE: f64 = 0.5;

/// What a colour token may be used for. There is no default: see the module docs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Usage {
    /// Legal as text on the declared light background, which means it meets 4.5 : 1 against it.
    OnLightText,
    /// Legal as text on the declared dark background.
    OnDarkText,
    /// Fills, borders, indicators and icons — never the colour of a glyph.
    FillOnly,
}

impl Usage {
    fn parse(text: &str) -> Result<Self> {
        Ok(match text {
            "on-light-text" => Self::OnLightText,
            "on-dark-text" => Self::OnDarkText,
            "fill-only" => Self::FillOnly,
            other => bail!(
                "unknown usage `{other}`; expected `on-light-text`, `on-dark-text` or `fill-only`"
            ),
        })
    }

    /// The spelling used in the source, echoed back into the generated docs.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::OnLightText => "on-light-text",
            Self::OnDarkText => "on-dark-text",
            Self::FillOnly => "fill-only",
        }
    }

    /// Whether the token is allowed to colour text, which is what makes the contrast minimum
    /// binding.
    fn colours_text(self) -> bool {
        matches!(self, Self::OnLightText | Self::OnDarkText)
    }
}

/// A colour, straight 8-bit-per-channel sRGB with an alpha channel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Rgba {
    pub(crate) red: u8,
    pub(crate) green: u8,
    pub(crate) blue: u8,
    pub(crate) alpha: u8,
}

impl Rgba {
    fn parse(text: &str) -> Result<Self> {
        let digits = text
            .strip_prefix('#')
            .with_context(|| format!("`{text}` is not a colour: it does not start with `#`"))?;
        if !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            bail!("`{text}` is not a colour: `{digits}` is not hexadecimal");
        }
        // Lower case only, so that one colour has exactly one spelling in the source and the three
        // artefacts cannot disagree about a value merely by disagreeing about its case.
        if digits.bytes().any(|byte| byte.is_ascii_uppercase()) {
            bail!("`{text}` must be written in lower case");
        }
        let channel = |at: usize| -> Result<u8> {
            u8::from_str_radix(&digits[at..at + 2], 16).context("hex pair")
        };
        match digits.len() {
            6 => Ok(Self {
                red: channel(0)?,
                green: channel(2)?,
                blue: channel(4)?,
                alpha: 0xff,
            }),
            8 => Ok(Self {
                red: channel(0)?,
                green: channel(2)?,
                blue: channel(4)?,
                alpha: channel(6)?,
            }),
            other => {
                bail!("`{text}` is not a colour: expected 6 or 8 hexadecimal digits, found {other}")
            }
        }
    }

    /// CSS text: `#rrggbb`, or `#rrggbbaa` when the colour is not opaque.
    pub(crate) fn css(self) -> String {
        let Self {
            red,
            green,
            blue,
            alpha,
        } = self;
        if alpha == 0xff {
            format!("#{red:02x}{green:02x}{blue:02x}")
        } else {
            format!("#{red:02x}{green:02x}{blue:02x}{alpha:02x}")
        }
    }

    /// This colour composited over `background`, which is what a partially transparent colour
    /// actually looks like and therefore what its contrast has to be measured on.
    fn over(self, background: Self) -> Self {
        if self.alpha == 0xff {
            return self;
        }
        let mix = |over: u8, under: u8| -> u8 {
            let alpha = f64::from(self.alpha) / 255.0;
            let blended = f64::from(over).mul_add(alpha, f64::from(under) * (1.0 - alpha));
            // `blended` is a convex combination of two values in 0..=255, so it is in range and the
            // rounded result fits a `u8`.
            blended.round().clamp(0.0, 255.0) as u8
        };
        Self {
            red: mix(self.red, background.red),
            green: mix(self.green, background.green),
            blue: mix(self.blue, background.blue),
            alpha: 0xff,
        }
    }

    /// WCAG 2.2 relative luminance.
    fn relative_luminance(self) -> f64 {
        let linear = |channel: u8| -> f64 {
            let value = f64::from(channel) / 255.0;
            if value <= 0.040_45 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126_f64.mul_add(
            linear(self.red),
            0.7152_f64.mul_add(linear(self.green), 0.0722 * linear(self.blue)),
        )
    }
}

/// The WCAG 2.2 contrast ratio between a foreground and a background, with the foreground
/// composited over the background first if it is not opaque.
pub(crate) fn contrast_ratio(foreground: Rgba, background: Rgba) -> f64 {
    let foreground = foreground.over(background).relative_luminance();
    let background = background.relative_luminance();
    let (lighter, darker) = if foreground >= background {
        (foreground, background)
    } else {
        (background, foreground)
    };
    (lighter + 0.05) / (darker + 0.05)
}

/// A CSS length unit. Three, because the source uses three.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LengthUnit {
    Pixels,
    Rem,
    Em,
}

impl LengthUnit {
    fn suffix(self) -> &'static str {
        match self {
            Self::Pixels => "px",
            Self::Rem => "rem",
            Self::Em => "em",
        }
    }

    /// The variant's name in the emitted Rust.
    pub(crate) fn rust_variant(self) -> &'static str {
        match self {
            Self::Pixels => "Pixels",
            Self::Rem => "Rem",
            Self::Em => "Em",
        }
    }
}

/// A CSS length: a magnitude and the unit it is in.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Length {
    pub(crate) magnitude: f64,
    pub(crate) unit: LengthUnit,
}

impl Length {
    fn parse(text: &str) -> Result<Self> {
        for unit in [LengthUnit::Rem, LengthUnit::Em, LengthUnit::Pixels] {
            if let Some(magnitude) = text.strip_suffix(unit.suffix()) {
                let magnitude: f64 = magnitude.parse().with_context(|| {
                    format!("`{text}` is not a dimension: `{magnitude}` is not a number")
                })?;
                return Ok(Self { magnitude, unit });
            }
        }
        bail!("`{text}` is not a dimension: it ends in none of `px`, `rem`, `em`")
    }

    pub(crate) fn css(self) -> String {
        format!("{}{}", number(self.magnitude), self.unit.suffix())
    }
}

/// A drop shadow, in the five parts CSS writes it in.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Shadow {
    pub(crate) offset_x: Length,
    pub(crate) offset_y: Length,
    pub(crate) blur: Length,
    pub(crate) spread: Length,
    pub(crate) color: Rgba,
}

impl Shadow {
    fn parse(value: &json::Value) -> Result<Self> {
        let length = |key: &str| -> Result<Length> {
            let text = value
                .get(key)
                .and_then(json::Value::string)
                .with_context(|| format!("a shadow needs a `{key}` dimension"))?;
            Length::parse(text)
        };
        let color = value
            .get("color")
            .and_then(json::Value::string)
            .context("a shadow needs a `color`")?;
        Ok(Self {
            offset_x: length("offsetX")?,
            offset_y: length("offsetY")?,
            blur: length("blur")?,
            spread: length("spread")?,
            color: Rgba::parse(color)?,
        })
    }

    pub(crate) fn css(self) -> String {
        // A zero spread is CSS's default and writing it out adds a term a reader has to check
        // against the other four. The generated Rust keeps it either way, so nothing is lost.
        if self.spread.magnitude == 0.0 {
            format!(
                "{} {} {} {}",
                self.offset_x.css(),
                self.offset_y.css(),
                self.blur.css(),
                self.color.css()
            )
        } else {
            format!(
                "{} {} {} {} {}",
                self.offset_x.css(),
                self.offset_y.css(),
                self.blur.css(),
                self.spread.css(),
                self.color.css()
            )
        }
    }
}

/// One token's value, typed by its `$type`.
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Value {
    Color(Rgba),
    Dimension(Length),
    Duration(f64),
    Number(f64),
    FontWeight(u16),
    FontStack(Vec<String>),
    CubicBezier([f64; 4]),
    Shadow(Shadow),
}

impl Value {
    /// The `$type` this value was read from, for error messages and for the type-agreement check
    /// between an alias and its target.
    pub(crate) fn type_name(&self) -> &'static str {
        match self {
            Self::Color(_) => "color",
            Self::Dimension(_) => "dimension",
            Self::Duration(_) => "duration",
            Self::Number(_) => "number",
            Self::FontWeight(_) => "fontWeight",
            Self::FontStack(_) => "fontFamily",
            Self::CubicBezier(_) => "cubicBezier",
            Self::Shadow(_) => "shadow",
        }
    }

    /// The value as CSS text — what `tokens.css` declares and what `tokens.ts` holds for every
    /// type CSS spells as a string.
    pub(crate) fn css(&self) -> String {
        match self {
            Self::Color(color) => color.css(),
            Self::Dimension(length) => length.css(),
            Self::Duration(milliseconds) => format!("{}ms", number(*milliseconds)),
            Self::Number(value) => number(*value),
            Self::FontWeight(weight) => weight.to_string(),
            Self::FontStack(faces) => font_stack_css(faces),
            Self::CubicBezier([x1, y1, x2, y2]) => format!(
                "cubic-bezier({}, {}, {}, {})",
                number(*x1),
                number(*y1),
                number(*x2),
                number(*y2)
            ),
            Self::Shadow(shadow) => shadow.css(),
        }
    }

    /// Whether `tokens.ts` holds this value as a JavaScript number rather than a string. Line
    /// heights and font weights are numbers in CSS too, so quoting them would make arithmetic on
    /// them a parse.
    pub(crate) fn is_typescript_number(&self) -> bool {
        matches!(self, Self::Number(_) | Self::FontWeight(_))
    }
}

/// A CSS font stack: each face quoted only when it has to be.
fn font_stack_css(faces: &[String]) -> String {
    let mut out = String::new();
    for (at, face) in faces.iter().enumerate() {
        if at > 0 {
            out.push_str(", ");
        }
        // A family name that is not a single CSS identifier (it has a space, or a digit first) must
        // be quoted; a generic family such as `sans-serif` must NOT be, or it stops being generic.
        let needs_quotes = face.contains(' ') || face.starts_with(|c: char| c.is_ascii_digit());
        if needs_quotes {
            let _ = write!(out, "\"{face}\"");
        } else {
            out.push_str(face);
        }
    }
    out
}

/// Renders a number the way all three artefacts render it: Rust's shortest round-tripping form,
/// through `f32`, which is the width the generated Rust table stores. Going through `f32` here is
/// what stops `tokens.css` and `tokens.ts` from carrying a precision the Rust table cannot hold.
pub(crate) fn number(value: f64) -> String {
    // Every magnitude in this source is a small decimal; `as f32` is exact for all of them and
    // lossy only for inputs no design token has.
    let narrowed = value as f32;
    let mut text = format!("{narrowed}");
    // `1` and `1.0` are the same CSS number; Rust's `Display` already writes the shorter form, and
    // this keeps a `-0` from ever reaching an artefact.
    if text == "-0" {
        text = "0".to_owned();
    }
    text
}

/// A group of tokens: one struct in the emitted Rust, one nested object in the emitted TypeScript,
/// and one name segment in the emitted CSS custom properties.
#[derive(Debug)]
pub(crate) struct Group {
    pub(crate) path: Vec<String>,
    pub(crate) rust_type: String,
    /// True when this group's children are colour schemes rather than tokens — the `theme` and
    /// `document` groups. It is what makes the emitters produce a scheme layer.
    pub(crate) schemes: bool,
    pub(crate) description: Option<String>,
    pub(crate) entries: Vec<Entry>,
}

/// One token.
#[derive(Debug)]
pub(crate) struct Token {
    pub(crate) path: Vec<String>,
    pub(crate) description: Option<String>,
    pub(crate) value: Value,
    /// `None` for every non-colour token; required on every colour token.
    pub(crate) usage: Option<Usage>,
    /// The token path of the surface this colour was measured against, when one is declared.
    pub(crate) background: Option<String>,
    /// The measured WCAG contrast ratio against that surface. Recorded in the generated docs for
    /// every colour that declares a background, and *enforced* for the ones tagged for text.
    pub(crate) contrast: Option<f64>,
}

/// A group's child: another group, or a token.
#[derive(Debug)]
pub(crate) enum Entry {
    Group(Group),
    Token(Token),
}

/// The whole source, read and checked.
#[derive(Debug)]
pub(crate) struct TokenSet {
    pub(crate) description: String,
    pub(crate) entries: Vec<Entry>,
    /// The colour schemes every `schemes` group declares, in source order — `["light", "dark"]`.
    /// Empty when the source declares no scheme group at all.
    pub(crate) scheme_names: Vec<String>,
}

impl TokenSet {
    /// Every token, depth-first in source order.
    pub(crate) fn tokens(&self) -> Vec<&Token> {
        let mut out = Vec::new();
        collect_tokens(&self.entries, &mut out);
        out
    }

    /// Every group, depth-first in source order.
    pub(crate) fn groups(&self) -> Vec<&Group> {
        let mut out = Vec::new();
        collect_groups(&self.entries, &mut out);
        out
    }
}

fn collect_tokens<'a>(entries: &'a [Entry], out: &mut Vec<&'a Token>) {
    for entry in entries {
        match entry {
            Entry::Token(token) => out.push(token),
            Entry::Group(group) => collect_tokens(&group.entries, out),
        }
    }
}

fn collect_groups<'a>(entries: &'a [Entry], out: &mut Vec<&'a Group>) {
    for entry in entries {
        if let Entry::Group(group) = entry {
            out.push(group);
            collect_groups(&group.entries, out);
        }
    }
}

/// The CSS custom property a token or a scheme-layer alias is written as: `--` and the path,
/// joined with hyphens. `color.ink-soft` is `--color-ink-soft`, which is exactly the name the
/// source stylesheet declares — that identity is what lets a host that already defines these
/// properties re-theme the platform with no code change.
pub(crate) fn custom_property(path: &[String]) -> String {
    format!("--{}", path.join("-"))
}

/// A path segment as a Rust field name: `ink-soft` becomes `ink_soft`.
pub(crate) fn rust_field(segment: &str) -> String {
    segment.replace('-', "_")
}

/// A path segment as a TypeScript property: `ink-soft` becomes `inkSoft`. A `snake_case` API is an
/// immediate smell to a TypeScript consumer, which is the same reason `bindings/mjx-wasm` renames
/// every one of its methods.
pub(crate) fn typescript_property(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    let mut capitalise = false;
    for character in segment.chars() {
        if character == '-' {
            capitalise = true;
        } else if capitalise {
            out.extend(character.to_uppercase());
            capitalise = false;
        } else {
            out.push(character);
        }
    }
    out
}

/// A dotted path, as `tokens.json` writes an alias and as the emitted `TOKENS` table records it.
pub(crate) fn dotted(path: &[String]) -> String {
    path.join(".")
}

/// The TypeScript path of a token: the dotted path with each segment camel-cased.
pub(crate) fn typescript_path(path: &[String]) -> String {
    path.iter()
        .map(|segment| typescript_property(segment))
        .collect::<Vec<_>>()
        .join(".")
}

/// Rust type names `mjx-tokens` writes by hand. A group may not claim one: the emitted file would
/// define a second type with the same name in the same module.
const RESERVED_RUST_TYPES: &[&str] = &[
    "Color",
    "ColorScheme",
    "CubicBezier",
    "Dimension",
    "Duration",
    "FontStack",
    "LengthUnit",
    "Resolution",
    "Shadow",
    "TokenError",
    "TokenIdentity",
    "TokenSource",
    "TokenValue",
    "Tokens",
];

/// Reads and checks the source. Every failure mode in the module docs lands here.
pub(crate) fn read(source: &str) -> Result<TokenSet> {
    let root = json::parse(source).map_err(|error| anyhow::anyhow!("tokens.json: {error}"))?;
    let members = root
        .object()
        .context("tokens.json: the document must be a JSON object")?;

    let description = root
        .get("$description")
        .and_then(json::Value::string)
        .context("tokens.json: the document needs a `$description`")?
        .to_owned();

    // Aliases are resolved against the raw tree, so a token may reference one declared after it.
    let mut raw_values = HashMap::new();
    index_raw_values(members, &mut Vec::new(), &mut raw_values);

    let mut entries = Vec::new();
    for (key, value) in members {
        if key.starts_with('$') {
            continue;
        }
        entries.push(read_entry(key, value, &[], None, &raw_values)?);
    }

    let scheme_names = scheme_names(&entries)?;
    let set = TokenSet {
        description,
        entries,
        scheme_names,
    };
    check_group_shapes_agree(&set)?;
    check_custom_properties_are_unique(&set)?;
    Ok(set)
}

/// Indexes every `$value` in the document by its dotted path, so an alias can be followed.
fn index_raw_values<'a>(
    members: &'a [(String, json::Value)],
    path: &mut Vec<String>,
    out: &mut HashMap<String, &'a json::Value>,
) {
    for (key, value) in members {
        if key.starts_with('$') {
            continue;
        }
        path.push(key.clone());
        if let Some(raw) = value.get("$value") {
            out.insert(path.join("."), raw);
        }
        if let Some(children) = value.object() {
            index_raw_values(children, path, out);
        }
        path.pop();
    }
}

/// Reads one entry, which is a token when it has a `$value` and a group otherwise.
fn read_entry(
    key: &str,
    value: &json::Value,
    parent: &[String],
    inherited_type: Option<&str>,
    raw_values: &HashMap<String, &json::Value>,
) -> Result<Entry> {
    check_key(key)?;
    let mut path = parent.to_vec();
    path.push(key.to_owned());
    let declared_type = value.get("$type").and_then(json::Value::string);
    let effective_type = declared_type.or(inherited_type);
    let description = value
        .get("$description")
        .and_then(json::Value::string)
        .map(str::to_owned);
    let extension = value
        .get("$extensions")
        .and_then(|it| it.get(EXTENSION_KEY));

    if value.get("$value").is_some() {
        return read_token(
            &path,
            value,
            effective_type,
            description,
            extension,
            raw_values,
        )
        .map(Entry::Token)
        .with_context(|| format!("token `{}`", dotted(&path)));
    }

    let rust_type = extension
        .and_then(|it| it.get("rustType"))
        .and_then(json::Value::string)
        .with_context(|| {
            format!(
                "group `{}` needs `$extensions.{EXTENSION_KEY}.rustType` — the name of the struct \
                 it becomes in Rust. Nothing derives it: `color` becomes `Palette`, and a name a \
                 reader has to decode is the thing this project's naming convention forbids",
                dotted(&path)
            )
        })?
        .to_owned();
    if RESERVED_RUST_TYPES.contains(&rust_type.as_str()) {
        bail!(
            "group `{}` claims the Rust type `{rust_type}`, which `mjx-tokens` writes by hand; the \
             emitted module would define it twice",
            dotted(&path)
        );
    }
    let schemes = extension
        .and_then(|it| it.get("schemes"))
        .and_then(json::Value::boolean)
        .unwrap_or(false);

    let members = value
        .object()
        .with_context(|| format!("group `{}` must be a JSON object", dotted(&path)))?;
    let mut entries = Vec::new();
    for (child_key, child) in members {
        if child_key.starts_with('$') {
            continue;
        }
        entries.push(read_entry(
            child_key,
            child,
            &path,
            effective_type,
            raw_values,
        )?);
    }
    if entries.is_empty() {
        bail!("group `{}` is empty", dotted(&path));
    }
    if schemes && entries.iter().any(|it| matches!(it, Entry::Token(_))) {
        bail!(
            "group `{}` declares `schemes`, so every one of its children must be a colour scheme, \
             not a token",
            dotted(&path)
        );
    }

    Ok(Entry::Group(Group {
        path,
        rust_type,
        schemes,
        description,
        entries,
    }))
}

fn read_token(
    path: &[String],
    value: &json::Value,
    effective_type: Option<&str>,
    description: Option<String>,
    extension: Option<&json::Value>,
    raw_values: &HashMap<String, &json::Value>,
) -> Result<Token> {
    let type_name = effective_type.context(
        "no `$type`, and no group above it declares one — a token whose type is a guess is a token \
         three artefacts can disagree about",
    )?;
    let raw = value.get("$value").context("no `$value`")?;
    let raw = follow_aliases(raw, raw_values, &mut Vec::new())?;
    let parsed = parse_value(type_name, raw)?;

    let usage = extension
        .and_then(|it| it.get("usage"))
        .and_then(json::Value::string)
        .map(Usage::parse)
        .transpose()?;
    let background_path = extension
        .and_then(|it| it.get("background"))
        .and_then(json::Value::string)
        .map(|text| {
            alias_target(text).map(str::to_owned).with_context(|| {
                format!("`background` must be a token reference such as `{{color.white}}`, not `{text}`")
            })
        })
        .transpose()?;

    let is_colour = matches!(parsed, Value::Color(_));
    if is_colour && usage.is_none() {
        bail!(
            "every colour token declares `$extensions.{EXTENSION_KEY}.usage` — `on-light-text`, \
             `on-dark-text` or `fill-only`. There is no default: DESIGN_TOKENS.md §2.2 calls \
             colouring text with a fill-only accent \"the most likely accessibility defect in the \
             chrome\", and a default would make the omission invisible"
        );
    }
    if !is_colour && usage.is_some() {
        bail!("`usage` is contrast metadata and only a colour token carries it");
    }

    let mut contrast = None;
    if let (Value::Color(colour), Some(background_path)) = (&parsed, background_path.as_ref()) {
        let background_raw = raw_values.get(background_path.as_str()).with_context(|| {
            format!("declares its background as `{background_path}`, which is not a token")
        })?;
        let background_raw = follow_aliases(background_raw, raw_values, &mut Vec::new())?;
        let Value::Color(background) = parse_value("color", background_raw)
            .with_context(|| format!("reading the background `{background_path}`"))?
        else {
            bail!("the background `{background_path}` is not a colour");
        };
        contrast = Some(contrast_ratio(*colour, background));
        check_usage(usage, *colour, background, background_path, contrast)?;
    }
    if let Some(usage) = usage {
        if usage.colours_text() && background_path.is_none() {
            bail!(
                "is tagged `{}`, so it must declare the `background` it was measured against — an \
                 unmeasured text colour is the rule stated rather than enforced",
                usage.as_str()
            );
        }
    }

    Ok(Token {
        path: path.to_vec(),
        description,
        value: parsed,
        usage,
        background: background_path,
        contrast,
    })
}

/// The contrast rule itself. `DESIGN_TOKENS.md` §2.2 in code.
fn check_usage(
    usage: Option<Usage>,
    colour: Rgba,
    background: Rgba,
    background_path: &str,
    contrast: Option<f64>,
) -> Result<()> {
    let Some(usage) = usage else { return Ok(()) };
    if !usage.colours_text() {
        return Ok(());
    }
    if colour.alpha != 0xff {
        bail!(
            "is tagged `{}` but is not opaque; text is drawn at full alpha, so a translucent text \
             colour is measured against a surface it is never seen on",
            usage.as_str()
        );
    }
    let luminance = background.relative_luminance();
    let background_is_light = luminance >= LIGHT_SURFACE_LUMINANCE;
    let expected_light = usage == Usage::OnLightText;
    if background_is_light != expected_light {
        bail!(
            "is tagged `{}`, but its declared background `{background_path}` ({}) has a relative \
             luminance of {luminance:.3}, which is {}. The tag names the wrong surface",
            usage.as_str(),
            background.css(),
            if background_is_light { "light" } else { "dark" }
        );
    }
    let ratio = contrast.unwrap_or(1.0);
    if ratio < TEXT_CONTRAST_MINIMUM {
        bail!(
            "is tagged `{}`, but {} on `{background_path}` ({}) measures {ratio:.2} : 1, below the \
             {TEXT_CONTRAST_MINIMUM} : 1 WCAG AA minimum for body text. DESIGN_TOKENS.md §2.2: use \
             this colour for fills, borders, indicators and icons, and the deep step of the same \
             ramp for accent-coloured text",
            usage.as_str(),
            colour.css(),
            background.css()
        );
    }
    Ok(())
}

/// `"{color.white}"` → `Some("color.white")`.
fn alias_target(text: &str) -> Option<&str> {
    text.strip_prefix('{')?.strip_suffix('}')
}

/// Follows an alias chain to the value it names, refusing a cycle and a dangling reference.
fn follow_aliases<'a>(
    value: &'a json::Value,
    raw_values: &HashMap<String, &'a json::Value>,
    visited: &mut Vec<String>,
) -> Result<&'a json::Value> {
    let Some(text) = value.string() else {
        return Ok(value);
    };
    let Some(target) = alias_target(text) else {
        return Ok(value);
    };
    if visited.iter().any(|seen| seen == target) {
        visited.push(target.to_owned());
        bail!("the alias chain {} is a cycle", visited.join(" -> "));
    }
    visited.push(target.to_owned());
    let next = raw_values
        .get(target)
        .with_context(|| format!("the alias `{{{target}}}` names no token"))?;
    follow_aliases(next, raw_values, visited)
}

fn parse_value(type_name: &str, raw: &json::Value) -> Result<Value> {
    let text = raw.string();
    Ok(match type_name {
        "color" => Value::Color(Rgba::parse(text.context("a colour is a string")?)?),
        "dimension" => Value::Dimension(Length::parse(text.context("a dimension is a string")?)?),
        "duration" => {
            let text = text.context("a duration is a string")?;
            let milliseconds = text
                .strip_suffix("ms")
                .with_context(|| format!("`{text}` is not a duration: it does not end in `ms`"))?;
            Value::Duration(
                milliseconds
                    .parse()
                    .with_context(|| format!("`{text}` is not a duration"))?,
            )
        }
        "number" => Value::Number(raw.number().context("a number is a JSON number")?),
        "fontWeight" => {
            let weight = raw.number().context("a font weight is a JSON number")?;
            if weight.fract() != 0.0 || !(1.0..=1000.0).contains(&weight) {
                bail!("`{weight}` is not a font weight: expected a whole number in 1..=1000");
            }
            // Bounded to 1..=1000 immediately above, so the cast is exact.
            Value::FontWeight(weight as u16)
        }
        "fontFamily" => {
            let faces = raw.array().context("a font family is an array of names")?;
            let mut names = Vec::with_capacity(faces.len());
            for face in faces {
                names.push(
                    face.string()
                        .context("every family name is a string")?
                        .to_owned(),
                );
            }
            if names.is_empty() {
                bail!("a font family needs at least one name");
            }
            Value::FontStack(names)
        }
        "cubicBezier" => {
            let points = raw
                .array()
                .context("a cubic bezier is an array of four numbers")?;
            if points.len() != 4 {
                bail!("a cubic bezier has four numbers, not {}", points.len());
            }
            let mut values = [0.0_f64; 4];
            for (slot, point) in values.iter_mut().zip(points) {
                *slot = point.number().context("a control point is a number")?;
            }
            Value::CubicBezier(values)
        }
        "shadow" => Value::Shadow(Shadow::parse(raw)?),
        other => bail!(
            "unknown `$type` `{other}`; this pipeline reads color, dimension, duration, number, \
             fontWeight, fontFamily, cubicBezier and shadow"
        ),
    })
}

/// A key has to be a valid segment of all three names at once: a CSS ident, a Rust field and a
/// TypeScript property.
fn check_key(key: &str) -> Result<()> {
    let mut characters = key.chars();
    let first = characters.next().context("an empty key")?;
    if !first.is_ascii_lowercase() {
        bail!("`{key}` must start with a lower-case ASCII letter");
    }
    if !key
        .chars()
        .all(|it| it.is_ascii_lowercase() || it.is_ascii_digit() || it == '-')
    {
        bail!("`{key}` must be lower-case ASCII letters, digits and hyphens (kebab-case)");
    }
    if key.ends_with('-') || key.contains("--") {
        bail!("`{key}` has an empty path segment");
    }
    if RUST_KEYWORDS.contains(&rust_field(key).as_str()) {
        bail!("`{key}` is a Rust keyword once written as a field name");
    }
    Ok(())
}

/// The Rust keywords a token key could collide with once hyphens become underscores. Only the ones
/// a design token might plausibly be called — the check is a guard, not a parser.
const RUST_KEYWORDS: &[&str] = &[
    "as", "box", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use",
    "where", "while", "async", "await", "dyn", "abstract", "final", "override", "priv", "typeof",
    "unsized", "virtual", "yield", "try",
];

/// The scheme names, checked to be the same for every `schemes` group in the source.
fn scheme_names(entries: &[Entry]) -> Result<Vec<String>> {
    let mut groups = Vec::new();
    collect_groups(entries, &mut groups);
    let mut names: Option<(Vec<String>, String)> = None;
    for group in groups.iter().filter(|group| group.schemes) {
        let mine: Vec<String> = group
            .entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Group(child) => child.path.last().cloned(),
                Entry::Token(_) => None,
            })
            .collect();
        match &names {
            None => names = Some((mine, dotted(&group.path))),
            Some((first, first_path)) if *first != mine => bail!(
                "`{}` declares the colour schemes {mine:?} but `{first_path}` declares {first:?} — \
                 one `ColorScheme` is emitted for the whole platform, so every scheme group must \
                 declare the same schemes in the same order",
                dotted(&group.path)
            ),
            Some(_) => {}
        }
    }
    Ok(names.map(|(names, _)| names).unwrap_or_default())
}

/// Two groups that claim the same Rust type must carry the same members, or the emitted module
/// would define one type twice with two different shapes.
fn check_group_shapes_agree(set: &TokenSet) -> Result<()> {
    let mut seen: BTreeMap<&str, (&Group, BTreeSet<String>)> = BTreeMap::new();
    for group in set.groups() {
        let members: BTreeSet<String> = group
            .entries
            .iter()
            .map(|entry| match entry {
                Entry::Group(child) => child.path.last().cloned().unwrap_or_default(),
                Entry::Token(token) => token.path.last().cloned().unwrap_or_default(),
            })
            .collect();
        match seen.entry(group.rust_type.as_str()) {
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert((group, members));
            }
            std::collections::btree_map::Entry::Occupied(slot) => {
                let (first, first_members) = slot.get();
                if *first_members != members {
                    let missing: Vec<&String> = first_members.difference(&members).collect();
                    let extra: Vec<&String> = members.difference(first_members).collect();
                    bail!(
                        "`{}` and `{}` both become `{}`, but they do not carry the same members: \
                         `{}` is missing {missing:?} and has {extra:?} the other has not. A \
                         semantic role that exists in one colour scheme and not the other is a \
                         component that cannot be themed",
                        dotted(&first.path),
                        dotted(&group.path),
                        group.rust_type,
                        dotted(&group.path)
                    );
                }
            }
        }
    }
    Ok(())
}

/// No two emitted custom properties may collide — the scheme layer's aliases included, because an
/// alias and a token are declared in the same `:root` cascade.
fn check_custom_properties_are_unique(set: &TokenSet) -> Result<()> {
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    let mut claim = |name: String, owner: String| -> Result<()> {
        if let Some(first) = seen.insert(name.clone(), owner.clone()) {
            bail!("`{first}` and `{owner}` both emit the custom property `{name}`");
        }
        Ok(())
    };
    for token in set.tokens() {
        claim(custom_property(&token.path), dotted(&token.path))?;
    }
    for alias in scheme_aliases(set) {
        claim(
            alias.custom_property.clone(),
            format!("the scheme layer of `{}`", alias.group_path),
        )?;
    }
    Ok(())
}

/// One entry in the emitted colour-scheme layer: a custom property that resolves to whichever
/// scheme is in force, and the per-scheme property it points at in each.
pub(crate) struct SchemeAlias {
    /// The dotted path of the `schemes` group this alias belongs to, for messages.
    pub(crate) group_path: String,
    /// The scheme-independent property, e.g. `--theme-background`.
    pub(crate) custom_property: String,
    /// The per-scheme property for each scheme, in `TokenSet::scheme_names` order, e.g.
    /// `["--theme-light-background", "--theme-dark-background"]`.
    pub(crate) per_scheme: Vec<String>,
}

/// The scheme layer: for every token under the *first* scheme of a `schemes` group, the
/// scheme-independent custom property that stands in for it.
///
/// The first scheme is the shape the others are checked against by
/// [`check_group_shapes_agree`], so walking it is walking all of them.
pub(crate) fn scheme_aliases(set: &TokenSet) -> Vec<SchemeAlias> {
    let mut out = Vec::new();
    for group in set.groups().into_iter().filter(|group| group.schemes) {
        let Some(Entry::Group(first_scheme)) = group.entries.first() else {
            continue;
        };
        let mut tokens = Vec::new();
        collect_tokens(&first_scheme.entries, &mut tokens);
        for token in tokens {
            // `theme.light.text-primary` → the alias is `theme` + `text-primary`, the scheme
            // segment dropped.
            let mut alias_path = group.path.clone();
            alias_path.extend_from_slice(&token.path[group.path.len() + 1..]);
            let per_scheme = set
                .scheme_names
                .iter()
                .map(|scheme| {
                    let mut path = group.path.clone();
                    path.push(scheme.clone());
                    path.extend_from_slice(&token.path[group.path.len() + 1..]);
                    custom_property(&path)
                })
                .collect();
            out.push(SchemeAlias {
                group_path: dotted(&group.path),
                custom_property: custom_property(&alias_path),
                per_scheme,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two ratios `DESIGN_TOKENS.md` §2.2 measured, recomputed here. If this drifts, every
    /// contrast decision in the source was taken against a different arithmetic.
    #[test]
    fn the_measured_ratios_in_the_specification_are_reproduced() {
        let white = Rgba::parse("#ffffff").expect("white");
        let green = Rgba::parse("#2e9e63").expect("green");
        let green_deep = Rgba::parse("#1e7a49").expect("green-deep");
        assert_eq!(format!("{:.2}", contrast_ratio(green, white)), "3.39");
        assert_eq!(format!("{:.2}", contrast_ratio(green_deep, white)), "5.34");
    }

    /// A translucent colour is measured on the surface it is actually seen on, not on its own
    /// nominal value. Half-alpha black over white is a mid grey, not black.
    #[test]
    fn a_translucent_colour_is_measured_composited() {
        let white = Rgba::parse("#ffffff").expect("white");
        let half_black = Rgba::parse("#00000080").expect("half black");
        let opaque = contrast_ratio(Rgba::parse("#000000").expect("black"), white);
        let composited = contrast_ratio(half_black, white);
        assert!(
            composited < opaque,
            "compositing must lighten: {composited} should be below {opaque}"
        );
        assert_eq!(format!("{composited:.2}"), "4.00");
    }

    #[test]
    fn names_are_derived_the_same_way_for_all_three_artefacts() {
        let path = vec![
            "theme".to_owned(),
            "light".to_owned(),
            "text-primary".to_owned(),
        ];
        assert_eq!(custom_property(&path), "--theme-light-text-primary");
        assert_eq!(typescript_path(&path), "theme.light.textPrimary");
        assert_eq!(rust_field("text-primary"), "text_primary");
        assert_eq!(dotted(&path), "theme.light.text-primary");
    }

    #[test]
    fn a_font_stack_quotes_only_the_names_that_need_it() {
        assert_eq!(
            font_stack_css(&[
                "Nunito Sans".to_owned(),
                "system-ui".to_owned(),
                "sans-serif".to_owned()
            ]),
            "\"Nunito Sans\", system-ui, sans-serif"
        );
    }

    /// A minimal but complete source, used by the rule tests below. It is deliberately *not* the
    /// real file: a fixture that already agrees with the assertion tests nothing.
    fn source_with(green_usage: &str) -> String {
        format!(
            r##"{{
              "$description": "a test source",
              "color": {{
                "$type": "color",
                "$extensions": {{ "mjx": {{ "rustType": "Palette" }} }},
                "white": {{ "$value": "#ffffff", "$extensions": {{ "mjx": {{ "usage": "fill-only" }} }} }},
                "green": {{
                  "$value": "#2e9e63",
                  "$extensions": {{ "mjx": {{ "usage": "{green_usage}", "background": "{{color.white}}" }} }}
                }}
              }}
            }}"##
        )
    }

    #[test]
    fn a_fill_only_accent_reads_and_records_its_measured_ratio() {
        let set = read(&source_with("fill-only")).expect("the source is valid");
        let green = set
            .tokens()
            .into_iter()
            .find(|token| token.path.last().map(String::as_str) == Some("green"))
            .expect("green is a token");
        assert_eq!(green.usage, Some(Usage::FillOnly));
        assert_eq!(
            format!("{:.2}", green.contrast.expect("measured")),
            "3.39",
            "a fill-only colour still records the ratio, so the decision is auditable"
        );
    }

    /// The gate the ticket exists for: tag the accent for text and the generator refuses, quoting
    /// the number `DESIGN_TOKENS.md` §2.2 measured.
    #[test]
    fn tagging_the_accent_for_text_is_refused_with_its_measured_ratio() {
        let error = read(&source_with("on-light-text")).expect_err("must be refused");
        let message = format!("{error:#}");
        assert!(
            message.contains("3.39 : 1"),
            "the refusal must quote the measured ratio: {message}"
        );
        assert!(
            message.contains("color.green"),
            "the refusal must name the token: {message}"
        );
        assert!(
            message.contains("4.5 : 1"),
            "the refusal must quote the minimum it failed: {message}"
        );
    }

    #[test]
    fn a_colour_with_no_usage_is_refused() {
        let source = r##"{
          "$description": "a test source",
          "color": {
            "$type": "color",
            "$extensions": { "mjx": { "rustType": "Palette" } },
            "white": { "$value": "#ffffff" }
          }
        }"##;
        let message = format!("{:#}", read(source).expect_err("must be refused"));
        assert!(message.contains("usage"), "{message}");
    }

    #[test]
    fn a_text_tag_naming_the_wrong_surface_is_refused() {
        let source = r##"{
          "$description": "a test source",
          "color": {
            "$type": "color",
            "$extensions": { "mjx": { "rustType": "Palette" } },
            "night": { "$value": "#131a17", "$extensions": { "mjx": { "usage": "fill-only" } } },
            "chalk": {
              "$value": "#ffffff",
              "$extensions": { "mjx": { "usage": "on-light-text", "background": "{color.night}" } }
            }
          }
        }"##;
        let message = format!("{:#}", read(source).expect_err("must be refused"));
        assert!(
            message.contains("names the wrong surface"),
            "white on a near-black surface is not `on-light-text`: {message}"
        );
    }

    #[test]
    fn two_schemes_that_disagree_about_their_members_are_refused() {
        let source = r##"{
          "$description": "a test source",
          "theme": {
            "$type": "color",
            "$extensions": { "mjx": { "rustType": "Themes", "schemes": true } },
            "light": {
              "$extensions": { "mjx": { "rustType": "ThemeColors" } },
              "surface": { "$value": "#ffffff", "$extensions": { "mjx": { "usage": "fill-only" } } },
              "border": { "$value": "#e7e0d2", "$extensions": { "mjx": { "usage": "fill-only" } } }
            },
            "dark": {
              "$extensions": { "mjx": { "rustType": "ThemeColors" } },
              "surface": { "$value": "#1b2420", "$extensions": { "mjx": { "usage": "fill-only" } } }
            }
          }
        }"##;
        let message = format!("{:#}", read(source).expect_err("must be refused"));
        assert!(message.contains("theme.dark"), "{message}");
        assert!(message.contains("border"), "{message}");
    }

    #[test]
    fn an_alias_cycle_is_refused_rather_than_hung_on() {
        let source = r##"{
          "$description": "a test source",
          "color": {
            "$type": "color",
            "$extensions": { "mjx": { "rustType": "Palette" } },
            "one": { "$value": "{color.two}", "$extensions": { "mjx": { "usage": "fill-only" } } },
            "two": { "$value": "{color.one}", "$extensions": { "mjx": { "usage": "fill-only" } } }
          }
        }"##;
        let message = format!("{:#}", read(source).expect_err("must be refused"));
        assert!(message.contains("cycle"), "{message}");
    }

    #[test]
    fn a_dangling_alias_is_refused() {
        let source = r##"{
          "$description": "a test source",
          "color": {
            "$type": "color",
            "$extensions": { "mjx": { "rustType": "Palette" } },
            "one": { "$value": "{color.absent}", "$extensions": { "mjx": { "usage": "fill-only" } } }
          }
        }"##;
        let message = format!("{:#}", read(source).expect_err("must be refused"));
        assert!(message.contains("names no token"), "{message}");
    }

    #[test]
    fn a_group_claiming_a_hand_written_rust_type_is_refused() {
        let source = r##"{
          "$description": "a test source",
          "color": {
            "$type": "color",
            "$extensions": { "mjx": { "rustType": "Shadow" } },
            "one": { "$value": "#ffffff", "$extensions": { "mjx": { "usage": "fill-only" } } }
          }
        }"##;
        let message = format!("{:#}", read(source).expect_err("must be refused"));
        assert!(message.contains("writes by hand"), "{message}");
    }
}
