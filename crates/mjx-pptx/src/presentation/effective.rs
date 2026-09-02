//! The effective readers: what a renderer *shows*, rather than what the part *states*.
//!
//! Every reader here walks the inheritance chain — shape, layout placeholder, master placeholder,
//! master text styles, theme, `presentation.xml` — and stops at the first tier that states the
//! property, baking every colour to a concrete `RRGGBB`.

use mjx_dml::{
    applicable_parts, resolve_character_properties, resolve_color, resolve_effects, resolve_fill,
    resolve_line, CellBorder, CharacterPropertiesSpec, ColorMap, ColorSpec, EffectList,
    EffectListSpec, Fill, FillSpec, FontSlot, IndentLevel, LineProperties, LineSpec, OnOffStyle,
    ParagraphPropertiesSpec, ResolvedColor, SchemeColors, TableStyleBorder, TableStyleCellStyle,
    TableStylePart, TableStyleTextStyle, TextBody, TextFont, TextListStyle, Theme,
    ThemeableLineStyle, Transform2D,
};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, RawElement};
use mjx_ooxml_types::namespaces::PML;

use crate::address::ShapePath;
use crate::error::PptxError;
use crate::geometry::ShapeBounds;
use crate::surface::Surface;
use crate::{nav, placement, slide};

use super::cells::cell_at;
use super::tables::table_flags;
use super::text::{
    list_style_tier, master_style_local, non_placeholder_style_local, nth_paragraph, nth_run,
    paragraph_level, resolved_paragraph_spec,
};
use super::Presentation;

impl Presentation {
    /// The **effective** fill of shape `shape_idx` on `surface`, as an interner-free
    /// [`FillSpec`] whose colors are resolved to concrete `RRGGBB` values — the fill the shape actually
    /// renders. Three sources are tried, in order: an explicit `p:spPr` fill; a `p:style > a:fillRef`
    /// (the theme fill-style at that index, `phClr` substituted by the reference's color); and, for a
    /// placeholder shape (`p:ph`), **inheritance** from the same-slot placeholder on the layout
    /// then the master. Scheme colors and color transforms are baked against the surface's theme + map.
    ///
    /// Returns `Ok(None)` when no source yields a fill. Reading does not dirty any part.
    ///
    /// See [the effective-properties guide](crate::effective_properties).
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, a relationship points
    /// outside the package, or a part is not well-formed.
    pub fn effective_shape_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<FillSpec>, PptxError> {
        let surface = surface.into();
        let map = self.color_map(surface)?.unwrap_or_else(ColorMap::identity);
        let theme_part = self.theme_part(surface)?;

        // The resolved color scheme (interner-free) — bridges the theme-part vs shape-part interners.
        let scheme = match &theme_part {
            Some(part) => {
                let doc = self.package.part_tree(part)?;
                let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                theme
                    .color_scheme()
                    .map(|cs| SchemeColors::from_scheme(cs, &doc.interner))
                    .unwrap_or_default()
            }
            None => SchemeColors::default(),
        };

        let candidates = self.placeholder_candidates(surface, &shape_idx.into())?;

        for (part, candidate) in candidates {
            // Extract the candidate's own fill while holding its part's borrow (fully owned).
            let own = {
                let doc = self.package.part_tree(&part)?;
                match candidate_shape(doc, candidate)? {
                    Some(shape) => shape_own_fill(shape, &doc.interner, &scheme, &map)?,
                    None => OwnFill::Absent,
                }
            };

            match own {
                OwnFill::Resolved(spec) => return Ok(Some(spec)),
                OwnFill::StyleRef(idx, color) => {
                    // Resolve the referenced theme fill-style (theme-part interner), substituting phClr.
                    if let Some(theme_part) = &theme_part {
                        let doc = self.package.part_tree(theme_part)?;
                        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                        if let Some(style) = theme.fill_style(idx) {
                            return Ok(Some(resolve_fill(
                                style,
                                &scheme,
                                &map,
                                color,
                                &doc.interner,
                            )));
                        }
                    }
                }
                OwnFill::Absent => {}
            }
        }

        Ok(None)
    }

    /// The **effective** outline of shape `shape_idx` on `surface`, as an interner-free
    /// [`LineSpec`] whose stroke color is resolved to a concrete `RRGGBB` value — the outline the shape
    /// actually renders. Three sources are tried, in order: an explicit `p:spPr > a:ln`; a
    /// `p:style > a:lnRef` (the theme line-style at that index, `phClr` substituted by the reference's
    /// color); and, for a placeholder shape (`p:ph`), **inheritance** from the same-slot placeholder on
    /// the slide layout then the master. Scheme colors and color transforms are baked against the
    /// slide's theme + map.
    ///
    /// Returns `Ok(None)` when no source yields an outline. Reading does not dirty any part.
    ///
    /// See [the effective-properties guide](crate::effective_properties).
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, a relationship points
    /// outside the package, or a part is not well-formed.
    pub fn effective_shape_outline(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<LineSpec>, PptxError> {
        let surface = surface.into();
        let map = self.color_map(surface)?.unwrap_or_else(ColorMap::identity);
        let theme_part = self.theme_part(surface)?;

        // The resolved color scheme (interner-free) — bridges the theme-part vs shape-part interners.
        let scheme = match &theme_part {
            Some(part) => {
                let doc = self.package.part_tree(part)?;
                let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                theme
                    .color_scheme()
                    .map(|cs| SchemeColors::from_scheme(cs, &doc.interner))
                    .unwrap_or_default()
            }
            None => SchemeColors::default(),
        };

        let candidates = self.placeholder_candidates(surface, &shape_idx.into())?;

        for (part, candidate) in candidates {
            // Extract the candidate's own outline while holding its part's borrow (fully owned).
            let own = {
                let doc = self.package.part_tree(&part)?;
                match candidate_shape(doc, candidate)? {
                    Some(shape) => shape_own_line(shape, &doc.interner, &scheme, &map)?,
                    None => OwnLine::Absent,
                }
            };

            match own {
                OwnLine::Resolved(spec) => return Ok(Some(spec)),
                OwnLine::StyleRef(idx, color) => {
                    // Resolve the referenced theme line-style (theme-part interner), substituting phClr.
                    if let Some(theme_part) = &theme_part {
                        let doc = self.package.part_tree(theme_part)?;
                        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                        if let Some(style) = theme.line_style(idx) {
                            return Ok(Some(resolve_line(
                                style,
                                &scheme,
                                &map,
                                color,
                                &doc.interner,
                            )));
                        }
                    }
                }
                OwnLine::Absent => {}
            }
        }

        Ok(None)
    }

    /// The **effective** effects of shape `shape_idx` on `surface`, as an interner-free
    /// [`EffectListSpec`] whose colors are resolved to concrete `RRGGBB` values — the effects the shape
    /// actually renders. Three sources are tried, in order: an explicit `p:spPr > a:effectLst`; a
    /// `p:style > a:effectRef` (the theme effect-style at that index, `phClr` substituted by the
    /// reference's color); and, for a placeholder shape (`p:ph`), **inheritance** from the same-slot
    /// placeholder on the slide layout then the master. Scheme colors and color transforms are baked
    /// against the slide's theme + map.
    ///
    /// Returns `Ok(None)` when no source yields effects. Reading does not dirty any part.
    ///
    /// See [the effective-properties guide](crate::effective_properties).
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, a relationship points
    /// outside the package, or a part is not well-formed.
    pub fn effective_shape_effects(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<EffectListSpec>, PptxError> {
        let surface = surface.into();
        let map = self.color_map(surface)?.unwrap_or_else(ColorMap::identity);
        let theme_part = self.theme_part(surface)?;

        // The resolved color scheme (interner-free) — bridges the theme-part vs shape-part interners.
        let scheme = match &theme_part {
            Some(part) => {
                let doc = self.package.part_tree(part)?;
                let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                theme
                    .color_scheme()
                    .map(|cs| SchemeColors::from_scheme(cs, &doc.interner))
                    .unwrap_or_default()
            }
            None => SchemeColors::default(),
        };

        let candidates = self.placeholder_candidates(surface, &shape_idx.into())?;

        for (part, candidate) in candidates {
            // Extract the candidate's own effects while holding its part's borrow (fully owned).
            let own = {
                let doc = self.package.part_tree(&part)?;
                match candidate_shape(doc, candidate)? {
                    Some(shape) => shape_own_effects(shape, &doc.interner, &scheme, &map)?,
                    None => OwnEffects::Absent,
                }
            };

            match own {
                OwnEffects::Resolved(spec) => return Ok(Some(*spec)),
                OwnEffects::StyleRef(idx, color) => {
                    // Resolve the referenced theme effect-style (theme-part interner), substituting phClr.
                    if let Some(theme_part) = &theme_part {
                        let doc = self.package.part_tree(theme_part)?;
                        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                        if let Some(style) = theme.effect_style(idx) {
                            return Ok(Some(resolve_effects(
                                style,
                                &scheme,
                                &map,
                                color,
                                &doc.interner,
                            )));
                        }
                    }
                }
                OwnEffects::Absent => {}
            }
        }

        Ok(None)
    }

    /// The **effective** transform of shape `shape_idx` on `surface` — where the shape actually
    /// renders, not what it declares. For a placeholder that places itself nowhere, this is the
    /// same-slot placeholder's transform on the slide layout, and failing that on the master.
    ///
    /// Returns `Ok(None)` when no tier places the shape. Reading does not dirty any part.
    ///
    /// # Inheritance is all-or-nothing
    ///
    /// Unlike text formatting, whose tiers each contribute what the ones above left unset, a
    /// transform is inherited **whole**: the first tier that states anything wins entirely. A shape
    /// cannot take its position from the layout and its size from the master, because PowerPoint
    /// offers no such thing — a shape that places itself places itself completely.
    ///
    /// A **present but empty** `<a:xfrm/>` states nothing, so the walk steps past it exactly as it
    /// steps past a tier with no transform element at all.
    ///
    /// A shape that is **not a placeholder** has no tiers to inherit from, so its effective transform
    /// is its explicit one.
    ///
    /// See [the effective-properties guide](crate::effective_properties).
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, a part is malformed, or a relationship in
    /// the inheritance chain points outside the package.
    pub fn effective_shape_transform(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<Transform2D>, PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let candidates = self.placeholder_candidates(surface, &path)?;

        let mut own = None;
        for (part, candidate) in candidates {
            let doc = self.package.part_tree(&part)?;
            let Some(shape) = candidate_shape(doc, candidate)? else {
                continue; // This tier does not define the slot at all.
            };
            let Some(element) = slide::shape_transform(shape, &doc.interner) else {
                continue; // …or defines it without placing it.
            };
            let transform = Transform2D::read(element, &doc.interner);
            if !transform.is_empty() {
                own = Some(transform);
                break;
            }
        }
        let Some(own) = own else {
            return Ok(None);
        };

        // Whichever tier placed the shape did so in the shape's *own* space; composing the enclosing
        // groups is what turns that into a slide rectangle. For a top-level shape this is the
        // identity, so nothing about the inheritance walk changes.
        if path.is_top_level() {
            return Ok(Some(own));
        }
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        Ok(placement::compose(sp_tree, &doc.interner, &path, &own))
    }

    /// The **effective** position and size of shape `shape_idx` on `surface` — where the shape
    /// actually renders, with the layout and master consulted for a placeholder that declares no
    /// bounds of its own.
    ///
    /// This is the question [`shape_bounds`](Self::shape_bounds) cannot answer: a title that
    /// declares no `a:xfrm` still renders somewhere, and where is on its layout. Returns `Ok(None)`
    /// when no tier places the shape, or when the tier that does names a rotation or a flip without
    /// naming both an `a:off` and an `a:ext`.
    ///
    /// Bounds are absolute within [`slide_size`](Self::slide_size). Reading does not dirty any part.
    ///
    /// See [the effective-properties guide](crate::effective_properties).
    ///
    /// # Errors
    /// As [`effective_shape_transform`](Self::effective_shape_transform).
    pub fn effective_shape_bounds(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<ShapeBounds>, PptxError> {
        Ok(self
            .effective_shape_transform(surface, shape_idx)?
            .as_ref()
            .and_then(ShapeBounds::from_transform))
    }

    // -----------------------------------------------------------------------------------------
    // Effective text formatting — what the text actually renders as
    //
    // Every reader above answers what a paragraph or run *declares*. These two answer what it
    // *renders as*, which is a different question: a placeholder that declares nothing still has a
    // size, and that size lives in the master's `p:txStyles`. Seven tiers, each contributing only
    // what the tiers above left unset — see `text_style_tiers` for the walk they share.
    // -----------------------------------------------------------------------------------------

    /// The **effective** character properties of run `run_idx` — what the run actually renders as,
    /// with every tier of inheritance resolved and its colors baked to concrete `RRGGBB`.
    ///
    /// Seven tiers contribute, highest priority first, each supplying only what the tiers above left
    /// unset:
    ///
    /// 1. the run's own `a:rPr`;
    /// 2. the paragraph's `a:pPr > a:defRPr`;
    /// 3. the shape's `a:lstStyle`, at the paragraph's level;
    /// 4. the same-slot placeholder's `a:lstStyle` on the layout, then the master — a shape that is
    ///    not a placeholder has no slot to be matched on, so it takes nothing here;
    /// 5. the master's `p:txStyles` — `p:titleStyle` for a title placeholder, `p:otherStyle` for the
    ///    date/footer/slide-number slots, `p:bodyStyle` for the rest. A shape that is *not* a
    ///    placeholder still takes one: `p:bodyStyle` if it is a text box (`p:cNvSpPr@txBox`),
    ///    `p:otherStyle` otherwise, per ECMA-376 Part 1 §19.3.1.35;
    /// 6. `p:defaultTextStyle` in `presentation.xml`;
    /// 7. the theme's font scheme, for a typeface still naming `+mj-lt` / `+mn-lt`.
    ///
    /// The paragraph's level (`a:pPr@lvl`, [`IndentLevel::TOP`] when unstated) is read once and
    /// selects which `a:lvlNpPr` every tier from 3 down contributes — which is why demoting a line
    /// changes its size and bullet without anything being written to the run.
    ///
    /// Each of tiers 3 to 6 is a list style, and each contributes **twice**: its `a:lvlNpPr` at the
    /// paragraph's level, and beneath that its own `a:defPPr` — the properties ECMA-376 Part 1
    /// §21.1.2.2.2 applies "when no other paragraph properties have been specified". A level the
    /// style does not define falls to that default rather than to `a:lvl1pPr`.
    ///
    /// Returns an empty spec when no tier contributes anything. Reading does not dirty any part.
    ///
    /// See [the effective-properties guide](crate::effective_properties).
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the shape has no text body, a relationship
    /// points outside the package, or a part is not well-formed.
    pub fn effective_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
        run_idx: usize,
    ) -> Result<CharacterPropertiesSpec, PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let scheme = self.resolved_scheme_colors(surface)?;
        let map = self.color_map(surface)?.unwrap_or_else(ColorMap::identity);

        // Tiers 1 and 2, and the level the rest are read at — all from the shape's own body.
        let (level, own, paragraph_default) =
            self.with_text_body(surface, &path, |body, interner| {
                let paragraph = nth_paragraph(body, para_idx)?;
                let count = paragraph.runs().count();
                let run = paragraph
                    .runs()
                    .nth(run_idx)
                    .ok_or(PptxError::RunIndexOutOfRange {
                        index: run_idx,
                        count,
                    })?;
                let properties = paragraph.properties();
                Ok((
                    paragraph_level(body, para_idx, interner),
                    run.properties()
                        .map(|rpr| resolve_character_properties(rpr, &scheme, &map, None, interner))
                        .unwrap_or_default(),
                    properties
                        .and_then(|ppr| ppr.default_run_properties(interner))
                        .map(|def| {
                            resolve_character_properties(&def, &scheme, &map, None, interner)
                        })
                        .unwrap_or_default(),
                ))
            })?;

        // Tiers 3–6 contribute their level's `a:defRPr`.
        let effective = self
            .text_style_tiers(surface, &path, level, &scheme, &map)?
            .iter()
            .filter_map(ParagraphPropertiesSpec::default_run_properties)
            .fold(own.merge_under(&paragraph_default), |resolved, tier| {
                resolved.merge_under(tier)
            });

        // Tier 7: a typeface that still names a theme font.
        self.resolve_theme_fonts(surface, effective)
    }

    /// The **effective** paragraph properties of paragraph `para_idx` — the layout it actually
    /// renders with, every tier of inheritance resolved.
    ///
    /// The same ladder as [`effective_run_properties`](Self::effective_run_properties), minus the
    /// run-level tiers: the paragraph's own `a:pPr`, then the shape's `a:lstStyle`, the same-slot
    /// placeholder's on the layout and master, the master's `p:txStyles`, and `p:defaultTextStyle`.
    /// Its [`default_run_properties`](ParagraphPropertiesSpec::default_run_properties) carry the
    /// merged `a:defRPr` of every tier, with colors baked.
    ///
    /// This is where a bullet comes from: a level-2 paragraph that declares nothing still answers with
    /// the master `bodyStyle`'s `a:lvl3pPr` bullet, size and indent.
    ///
    /// Returns an empty spec when no tier contributes anything. Reading does not dirty any part.
    ///
    /// See [the effective-properties guide](crate::effective_properties).
    ///
    /// # Errors
    /// As [`effective_run_properties`](Self::effective_run_properties).
    pub fn effective_paragraph_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
    ) -> Result<ParagraphPropertiesSpec, PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let scheme = self.resolved_scheme_colors(surface)?;
        let map = self.color_map(surface)?.unwrap_or_else(ColorMap::identity);

        let (level, own) = self.with_text_body(surface, &path, |body, interner| {
            let level = paragraph_level(body, para_idx, interner);
            let own = nth_paragraph(body, para_idx)?
                .properties()
                .map(|ppr| resolved_paragraph_spec(ppr, &scheme, &map, interner))
                .unwrap_or_default();
            Ok((level, own))
        })?;

        Ok(self
            .text_style_tiers(surface, &path, level, &scheme, &map)?
            .iter()
            .fold(own, |resolved, tier| resolved.merge_under(tier)))
    }

    // ---------------------------------------------------------------------------------------------
    // Effective cell formatting — what a table cell actually renders as.
    //
    // Resolution order: the cell's own `a:tcPr` wins; then the table style's parts, selected by the
    // cell's position and the `a:tblPr` flags (`applicable_parts`), most specific first; then the
    // theme, for an `lnRef` / `fillRef`. Colours bake to concrete `RRGGBB`, exactly as the shape
    // resolvers do. Every read walks three parts (slide, `tableStyles.xml`, theme), extracting owned
    // values while each is borrowed. Reading dirties nothing.
    // ---------------------------------------------------------------------------------------------

    /// The **effective** fill of the cell at `(row, column)` of the table shape `shape_idx` frames — an
    /// interner-free [`FillSpec`] with its colour baked to concrete `RRGGBB`, or `None` if nothing
    /// fills the cell. The cell's own `a:tcPr` fill wins; else the first applicable style part with a
    /// fill (explicit or a theme `fillRef`).
    ///
    /// See [the effective-properties guide](crate::effective_properties).
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), plus [`PptxError::TableCellOutOfRange`].
    pub fn effective_cell_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
    ) -> Result<Option<FillSpec>, PptxError> {
        let surface = surface.into();
        let scheme = self.resolved_scheme_colors(surface)?;
        let map = self.color_map(surface)?.unwrap_or_else(ColorMap::identity);
        let theme_part = self.theme_part(surface)?;

        let path = shape_idx.into();
        let (own, dims, flags) = self.with_table(surface, &path, |table, interner| {
            let (rows, columns) = (table.row_count(), table.column_count());
            let cell = cell_at(table, row, column)?;
            let own = cell
                .properties()
                .and_then(|tcpr| tcpr.fill(interner))
                .map(|fill| resolve_fill(&fill, &scheme, &map, None, interner));
            Ok((own, (rows, columns), table_flags(table, interner)))
        })?;

        if let Some(spec) = own {
            return Ok(Some(spec));
        }

        let parts = applicable_parts(row, column, dims.0, dims.1, flags);
        let Some(part_fills) =
            self.cell_style_candidates(surface, &path, &parts, |cell_style, interner| {
                part_own_fill(cell_style, interner, &scheme, &map)
            })?
        else {
            return Ok(None);
        };

        for own in part_fills {
            match own {
                OwnFill::Resolved(spec) => return Ok(Some(spec)),
                OwnFill::StyleRef(idx, color) => {
                    if let Some(theme_part) = &theme_part {
                        let doc = self.package.part_tree(theme_part)?;
                        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                        if let Some(style) = theme.fill_style(idx) {
                            return Ok(Some(resolve_fill(
                                style,
                                &scheme,
                                &map,
                                color,
                                &doc.interner,
                            )));
                        }
                    }
                }
                OwnFill::Absent => {}
            }
        }
        Ok(None)
    }

    /// The **effective** border on one `edge` of the cell at `(row, column)` — an interner-free
    /// [`LineSpec`] with its stroke colour baked, or `None`. The cell's own `a:tcPr` edge wins; else
    /// the applicable style parts' `a:tcBdr`, taking the outer edge (`top`/`left`/…) for a cell on the
    /// table's rim and the interior edge (`insideH`/`insideV`) for one within it.
    ///
    /// See [the effective-properties guide](crate::effective_properties).
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), plus [`PptxError::TableCellOutOfRange`].
    pub fn effective_cell_border(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        edge: CellBorder,
    ) -> Result<Option<LineSpec>, PptxError> {
        let surface = surface.into();
        let scheme = self.resolved_scheme_colors(surface)?;
        let map = self.color_map(surface)?.unwrap_or_else(ColorMap::identity);
        let theme_part = self.theme_part(surface)?;

        let path = shape_idx.into();
        let (own, dims, flags) = self.with_table(surface, &path, |table, interner| {
            let (rows, columns) = (table.row_count(), table.column_count());
            let cell = cell_at(table, row, column)?;
            let own = cell
                .properties()
                .and_then(|tcpr| tcpr.border(interner, edge))
                .map(|line| resolve_line(&line, &scheme, &map, None, interner));
            Ok((own, (rows, columns), table_flags(table, interner)))
        })?;

        if let Some(spec) = own {
            return Ok(Some(spec));
        }

        let (rows, columns) = dims;
        let style_edge = style_border_key(edge, row, column, rows, columns);
        let parts = applicable_parts(row, column, rows, columns, flags);
        let Some(part_lines) =
            self.cell_style_candidates(surface, &path, &parts, |cell_style, interner| {
                cell_style
                    .borders(interner)
                    .and_then(|borders| borders.border(interner, style_edge))
                    .map_or(OwnLine::Absent, |themeable| {
                        part_own_line(themeable, interner, &scheme, &map)
                    })
            })?
        else {
            return Ok(None);
        };

        for own in part_lines {
            match own {
                OwnLine::Resolved(spec) => return Ok(Some(spec)),
                OwnLine::StyleRef(idx, color) => {
                    if let Some(theme_part) = &theme_part {
                        let doc = self.package.part_tree(theme_part)?;
                        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                        if let Some(style) = theme.line_style(idx) {
                            return Ok(Some(resolve_line(
                                style,
                                &scheme,
                                &map,
                                color,
                                &doc.interner,
                            )));
                        }
                    }
                }
                OwnLine::Absent => {}
            }
        }
        Ok(None)
    }

    /// The **effective** run properties of a cell's text run — the [`CharacterPropertiesSpec`] it
    /// actually renders with, colours baked. A shorter ladder than a shape's (a cell inherits from its
    /// table style, not a placeholder chain), highest first: the run's own `a:rPr`, the paragraph's
    /// `a:defRPr`, the table style's `a:tcTxStyle` for each applicable part (bold / italic / colour),
    /// then the presentation's `p:defaultTextStyle`.
    ///
    /// See [the effective-properties guide](crate::effective_properties).
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), plus [`PptxError::TableCellOutOfRange`] and the
    /// paragraph/run index errors.
    pub fn effective_cell_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        para_idx: usize,
        run_idx: usize,
    ) -> Result<CharacterPropertiesSpec, PptxError> {
        let surface = surface.into();
        let scheme = self.resolved_scheme_colors(surface)?;
        let map = self.color_map(surface)?.unwrap_or_else(ColorMap::identity);

        let path = shape_idx.into();
        let (level, own, para_default, dims, flags) =
            self.with_table(surface, &path, |table, interner| {
                let (rows, columns) = (table.row_count(), table.column_count());
                let cell = cell_at(table, row, column)?;
                let body = cell
                    .text_body()
                    .ok_or(PptxError::MalformedSlide("table cell has no text body"))?;
                let level = paragraph_level(body, para_idx, interner);
                let paragraph = nth_paragraph(body, para_idx)?;
                let run = nth_run(paragraph, run_idx)?;
                let own = run
                    .properties()
                    .map(|rpr| resolve_character_properties(rpr, &scheme, &map, None, interner))
                    .unwrap_or_default();
                let para_default = paragraph
                    .properties()
                    .and_then(|ppr| ppr.default_run_properties(interner))
                    .map(|def| resolve_character_properties(&def, &scheme, &map, None, interner))
                    .unwrap_or_default();
                Ok((
                    level,
                    own,
                    para_default,
                    (rows, columns),
                    table_flags(table, interner),
                ))
            })?;

        let parts = applicable_parts(row, column, dims.0, dims.1, flags);
        let style_text = self.cell_style_text(surface, &path, &parts, &scheme, &map)?;
        let default_text = self.default_text_run_properties(level, &scheme, &map)?;

        let effective = own
            .merge_under(&para_default)
            .merge_under(&style_text)
            .merge_under(&default_text);
        self.resolve_theme_fonts(surface, effective)
    }

    /// Runs `extract` over each applicable style part's `a:tcStyle`, most specific first, returning the
    /// results in that order — or `None` when the table resolves to no style (so the caller stops at
    /// the cell's own properties). Resolves an inline `a:tableStyle` or a shared one alike.
    fn cell_style_candidates<T>(
        &mut self,
        surface: Surface,
        shape_idx: impl Into<ShapePath>,
        parts: &[TableStylePart],
        extract: impl Fn(&TableStyleCellStyle, &Interner) -> T,
    ) -> Result<Option<Vec<T>>, PptxError> {
        self.with_resolved_style(surface, shape_idx, |style, interner| {
            Ok(parts
                .iter()
                .filter_map(|&part| {
                    let cell_style = style.part(interner, part)?.cell_style(interner)?;
                    Some(extract(&cell_style, interner))
                })
                .collect())
        })
    }

    /// The table style's text contribution for a cell — the `a:tcTxStyle` of each applicable part,
    /// merged most-specific-first. Empty when the table resolves to no style.
    fn cell_style_text(
        &mut self,
        surface: Surface,
        shape_idx: impl Into<ShapePath>,
        parts: &[TableStylePart],
        scheme: &SchemeColors,
        map: &ColorMap,
    ) -> Result<CharacterPropertiesSpec, PptxError> {
        Ok(self
            .with_resolved_style(surface, shape_idx, |style, interner| {
                let mut spec = CharacterPropertiesSpec::new();
                for &part in parts {
                    if let Some(text_style) = style
                        .part(interner, part)
                        .and_then(|part| part.text_style(interner))
                    {
                        spec =
                            spec.merge_under(&style_text_spec(&text_style, scheme, map, interner));
                    }
                }
                Ok(spec)
            })?
            .unwrap_or_default())
    }

    /// The presentation's `p:defaultTextStyle` run properties at `level`, colours baked — the bottom
    /// tier of a cell's text ladder. Empty when the presentation declares none.
    fn default_text_run_properties(
        &mut self,
        level: IndentLevel,
        scheme: &SchemeColors,
        map: &ColorMap,
    ) -> Result<CharacterPropertiesSpec, PptxError> {
        let presentation_part = self.presentation_part.clone();
        let doc = self.package.part_tree(&presentation_part)?;
        let Some(default) = nav::child(&doc.root, &doc.interner, PML, "defaultTextStyle") else {
            return Ok(CharacterPropertiesSpec::new());
        };
        let list_style = TextListStyle::from_xml(default, &doc.interner)?;
        let spec = list_style_tier(Some(&list_style), level, scheme, map, &doc.interner)
            .iter()
            .flatten()
            .filter_map(ParagraphPropertiesSpec::default_run_properties)
            .fold(CharacterPropertiesSpec::new(), |resolved, tier| {
                resolved.merge_under(tier)
            });
        Ok(spec)
    }

    /// Tiers 3–6 of the ladder, in order and already interner-free: the shape's own `a:lstStyle`, the
    /// same-slot placeholder's on each ancestor part, the master's `p:txStyles`, and the
    /// presentation's `p:defaultTextStyle` — each taken at `level`, and each contributing its
    /// `a:defPPr` beneath what it says at that level (see `list_style_tier`).
    ///
    /// One walk serves both public answers: a tier's `a:lvlNpPr` *is* the paragraph contribution, and
    /// its `a:defRPr` is the character one.
    fn text_style_tiers(
        &mut self,
        surface: Surface,
        shape_idx: impl Into<ShapePath>,
        level: IndentLevel,
        scheme: &SchemeColors,
        map: &ColorMap,
    ) -> Result<Vec<ParagraphPropertiesSpec>, PptxError> {
        let mut tiers = Vec::new();

        // Tier 3 — the shape's own list style, plus the two facts the tiers below need: the
        // placeholder slot they are matched on, and whether the shape is a text box.
        let (placeholder, is_text_box) = {
            let part = self.surface_part(surface)?;
            let doc = self.package.part_tree(&part)?;
            let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
            if let Some(txbody) = slide::shape_txbody(shape, &doc.interner) {
                let body = TextBody::from_xml(txbody, &doc.interner)?;
                tiers.extend(
                    list_style_tier(body.list_style(), level, scheme, map, &doc.interner)
                        .into_iter()
                        .flatten(),
                );
            }
            (
                slide::shape_placeholder(shape, &doc.interner),
                slide::shape_is_text_box(shape, &doc.interner),
            )
        };

        // Tier 4 — the same-slot placeholder's list style, on the layout then the master. A shape
        // that is not a placeholder has no slot to be matched on, so it inherits from no ancestor
        // shape and this tier contributes nothing to it.
        if let Some(slot) = placeholder {
            for ancestor in self.inheritance_chain(surface)?.into_iter().skip(1) {
                let doc = self.package.part_tree(&ancestor)?;
                let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
                let Some(shape) = slide::find_placeholder(sp_tree, slot, &doc.interner) else {
                    continue;
                };
                let Some(txbody) = slide::shape_txbody(shape, &doc.interner) else {
                    continue;
                };
                let body = TextBody::from_xml(txbody, &doc.interner)?;
                tiers.extend(
                    list_style_tier(body.list_style(), level, scheme, map, &doc.interner)
                        .into_iter()
                        .flatten(),
                );
            }
        }

        // Tier 5 — the master's text styles. A slide master names them by slot in `p:txStyles`
        // (`p:titleStyle` / `p:otherStyle` / `p:bodyStyle`); a notes master instead carries a single
        // `p:notesStyle` that styles its body text.
        //
        // Only the *last* part of the chain is consulted, unlike tier 4 which walks every ancestor:
        // ECMA-376 Part 1 §19.3.1.52 says `p:txStyles` "is only for use within the Slide Master",
        // and a chain holds at most one master, always last. An absent element therefore means
        // either that the chain never reached a master or that the master declares no text styles.
        let chain = self.inheritance_chain(surface)?;
        let master = chain
            .last()
            .expect("a chain always holds the surface's own part");
        let doc = self.package.part_tree(master)?;
        let master_style = if matches!(surface, Surface::Notes(_) | Surface::NotesMaster) {
            nav::child(&doc.root, &doc.interner, PML, "notesStyle")
        } else {
            let local = match placeholder {
                Some(slot) => master_style_local(slot),
                None => non_placeholder_style_local(is_text_box),
            };
            nav::child(&doc.root, &doc.interner, PML, "txStyles")
                .and_then(|styles| nav::child(styles, &doc.interner, PML, local))
        };
        if let Some(named) = master_style {
            let list_style = TextListStyle::from_xml(named, &doc.interner)?;
            tiers.extend(
                list_style_tier(Some(&list_style), level, scheme, map, &doc.interner)
                    .into_iter()
                    .flatten(),
            );
        }

        // Tier 6 — `p:defaultTextStyle`, which applies to every shape, placeholder or not.
        let presentation_part = self.presentation_part.clone();
        let doc = self.package.part_tree(&presentation_part)?;
        if let Some(default) = nav::child(&doc.root, &doc.interner, PML, "defaultTextStyle") {
            let list_style = TextListStyle::from_xml(default, &doc.interner)?;
            tiers.extend(
                list_style_tier(Some(&list_style), level, scheme, map, &doc.interner)
                    .into_iter()
                    .flatten(),
            );
        }

        Ok(tiers)
    }

    /// Tier 7 — replaces any typeface still naming a theme font (`+mj-lt`, `+mn-ea`, …) with the one
    /// the surface's theme actually names. A slot the scheme leaves undefined keeps its reference,
    /// which is the honest answer: the file points somewhere the theme does not go.
    fn resolve_theme_fonts(
        &mut self,
        surface: Surface,
        spec: CharacterPropertiesSpec,
    ) -> Result<CharacterPropertiesSpec, PptxError> {
        if !FontSlot::all_slots()
            .into_iter()
            .any(|slot| spec.font(slot).is_some_and(TextFont::is_theme_reference))
        {
            return Ok(spec);
        }
        let Some(theme_part) = self.theme_part(surface)? else {
            return Ok(spec);
        };
        let doc = self.package.part_tree(&theme_part)?;
        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
        let Some(font_scheme) = theme.font_scheme() else {
            return Ok(spec);
        };

        let mut resolved = spec.clone();
        for slot in FontSlot::all_slots() {
            let Some(font) = spec.font(slot) else {
                continue;
            };
            if let Some(named) = font_scheme.resolve(font) {
                if named != font {
                    resolved = resolved.with_font_for(slot, named.clone());
                }
            }
        }
        Ok(resolved)
    }

    /// The surface's theme color scheme, resolved to concrete RGB — the interner-free bridge every
    /// effective reader builds once before walking parts.
    fn resolved_scheme_colors(&mut self, surface: Surface) -> Result<SchemeColors, PptxError> {
        let Some(part) = self.theme_part(surface)? else {
            return Ok(SchemeColors::default());
        };
        let doc = self.package.part_tree(&part)?;
        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
        Ok(theme
            .color_scheme()
            .map(|scheme| SchemeColors::from_scheme(scheme, &doc.interner))
            .unwrap_or_default())
    }
}

pub(super) fn resolve_shape_ref<'a>(
    doc: &'a RawDocument,
    surface: Surface,
    path: &ShapePath,
) -> Result<&'a RawElement, PptxError> {
    let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
    slide::resolve_shape(sp_tree, &doc.interner, path).map_err(|count| {
        PptxError::ShapeIndexOutOfRange {
            surface,
            path: path.clone(),
            count,
        }
    })
}

/// Resolves `path` to a shape mutably, from a part's already-split `root` and `interner` — so the
/// caller keeps the `&mut Interner` it needs to name any new elements the edit inserts.
pub(super) fn resolve_shape_in<'a>(
    root: &'a mut RawElement,
    interner: &Interner,
    surface: Surface,
    path: &ShapePath,
) -> Result<&'a mut RawElement, PptxError> {
    let sp_tree = slide::sp_tree_mut(root, interner)?;
    slide::resolve_shape_mut(sp_tree, interner, path).map_err(|count| {
        PptxError::ShapeIndexOutOfRange {
            surface,
            path: path.clone(),
            count,
        }
    })
}

/// Resolves `path` to a shape's parent container and its child position, for removal.
pub(super) fn resolve_shape_position_in<'a>(
    root: &'a mut RawElement,
    interner: &Interner,
    surface: Surface,
    path: &ShapePath,
) -> Result<(&'a mut RawElement, usize), PptxError> {
    let sp_tree = slide::sp_tree_mut(root, interner)?;
    slide::resolve_shape_position(sp_tree, interner, path).map_err(|count| {
        PptxError::ShapeIndexOutOfRange {
            surface,
            path: path.clone(),
            count,
        }
    })
}

/// How to locate a candidate shape within a part's shape tree while resolving an effective property.
#[derive(Debug, Clone)]
pub(super) enum Candidate {
    /// The originally-requested shape, by address (the surface's own part).
    Address(ShapePath),
    /// The matching placeholder on an ancestor part (layout / master).
    Placeholder(slide::Placeholder),
}

/// Resolves a [`Candidate`] to the shape it names in `doc`, or `None` when that part has no such
/// shape — an ancestor that simply does not define the slot, which every effective walk treats as
/// "this tier says nothing" and steps past.
///
/// Takes the document rather than the package so the caller owns the borrow and can extract what it
/// needs before the next candidate is fetched.
fn candidate_shape(
    doc: &RawDocument,
    candidate: Candidate,
) -> Result<Option<&RawElement>, PptxError> {
    let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
    Ok(match candidate {
        Candidate::Address(path) => slide::resolve_shape(sp_tree, &doc.interner, &path).ok(),
        Candidate::Placeholder(ph) => slide::find_placeholder(sp_tree, ph, &doc.interner),
    })
}

/// A candidate shape's own fill, extracted while its part's tree is borrowed (fully owned, so no
/// borrow escapes): an already-resolved fill, a theme style reference to resolve against the theme, or
/// no fill.
enum OwnFill {
    /// An explicit `p:spPr` fill, already resolved to concrete colors.
    Resolved(FillSpec),
    /// A `p:style > a:fillRef@idx` with its (already-resolved) `phClr` substitute color.
    StyleRef(u32, Option<ResolvedColor>),
    /// The shape declares no fill of its own.
    Absent,
}

/// The fill a `shape` declares itself (explicit `p:spPr` fill, or a `p:style > a:fillRef`), resolved
/// against `scheme` / `map`. The style-reference case returns its index + resolved color for the
/// caller to resolve against the theme (which lives in a different part interner).
fn shape_own_fill(
    shape: &RawElement,
    interner: &Interner,
    scheme: &SchemeColors,
    map: &ColorMap,
) -> Result<OwnFill, PptxError> {
    if let Some(fill_element) = slide::shape_fill(shape, interner) {
        let fill = Fill::from_xml(fill_element, interner)?;
        return Ok(OwnFill::Resolved(resolve_fill(
            &fill, scheme, map, None, interner,
        )));
    }
    if let Some(reference) = slide::shape_fill_ref(shape, interner) {
        if let Some(idx) = reference.index().filter(|idx| *idx > 0) {
            let color = reference
                .color()
                .and_then(|c| resolve_color(c, scheme, map, None, interner));
            return Ok(OwnFill::StyleRef(idx, color));
        }
    }
    Ok(OwnFill::Absent)
}

/// A candidate shape's own outline, extracted while its part's tree is borrowed (fully owned, so no
/// borrow escapes): an already-resolved outline, a theme style reference to resolve against the theme,
/// or no outline.
enum OwnLine {
    /// An explicit `p:spPr > a:ln`, already resolved to a concrete stroke color.
    Resolved(LineSpec),
    /// A `p:style > a:lnRef@idx` with its (already-resolved) `phClr` substitute color.
    StyleRef(u32, Option<ResolvedColor>),
    /// The shape declares no outline of its own.
    Absent,
}

/// The outline a `shape` declares itself (explicit `p:spPr > a:ln`, or a `p:style > a:lnRef`), resolved
/// against `scheme` / `map`. The style-reference case returns its index + resolved color for the caller
/// to resolve against the theme (which lives in a different part interner).
fn shape_own_line(
    shape: &RawElement,
    interner: &Interner,
    scheme: &SchemeColors,
    map: &ColorMap,
) -> Result<OwnLine, PptxError> {
    if let Some(line_element) = slide::shape_line(shape, interner) {
        let line = LineProperties::from_xml(line_element, interner)?;
        return Ok(OwnLine::Resolved(resolve_line(
            &line, scheme, map, None, interner,
        )));
    }
    if let Some(reference) = slide::shape_line_ref(shape, interner) {
        if let Some(idx) = reference.index().filter(|idx| *idx > 0) {
            let color = reference
                .color()
                .and_then(|c| resolve_color(c, scheme, map, None, interner));
            return Ok(OwnLine::StyleRef(idx, color));
        }
    }
    Ok(OwnLine::Absent)
}

/// A style part's cell fill: an explicit fill (baked) or a theme `a:fillRef` (index + resolved
/// `phClr` substitute), mirroring [`shape_own_fill`] for a `a:tcStyle`.
fn part_own_fill(
    cell_style: &TableStyleCellStyle,
    interner: &Interner,
    scheme: &SchemeColors,
    map: &ColorMap,
) -> OwnFill {
    if let Some(fill) = cell_style.fill(interner) {
        return OwnFill::Resolved(resolve_fill(&fill, scheme, map, None, interner));
    }
    if let Some(reference) = cell_style.fill_reference(interner) {
        if let Some(idx) = reference.index().filter(|idx| *idx > 0) {
            let color = reference
                .color()
                .and_then(|color| resolve_color(color, scheme, map, None, interner));
            return OwnFill::StyleRef(idx, color);
        }
    }
    OwnFill::Absent
}

/// A themeable border line: an explicit `a:ln` (baked) or a theme `a:lnRef` (index + resolved colour).
fn part_own_line(
    border: ThemeableLineStyle,
    interner: &Interner,
    scheme: &SchemeColors,
    map: &ColorMap,
) -> OwnLine {
    match border {
        ThemeableLineStyle::Line(line) => {
            OwnLine::Resolved(resolve_line(&line, scheme, map, None, interner))
        }
        ThemeableLineStyle::Reference(reference) => {
            match reference.index().filter(|idx| *idx > 0) {
                Some(idx) => {
                    let color = reference
                        .color()
                        .and_then(|color| resolve_color(color, scheme, map, None, interner));
                    OwnLine::StyleRef(idx, color)
                }
                None => OwnLine::Absent,
            }
        }
    }
}

/// Which `a:tcBdr` edge draws a cell's `edge`: the outer edge (`top`/`left`/…) for a cell on the
/// table's rim, the interior edge (`insideH`/`insideV`) for one within it; diagonals map straight
/// across.
fn style_border_key(
    edge: CellBorder,
    row: usize,
    column: usize,
    rows: usize,
    columns: usize,
) -> TableStyleBorder {
    match edge {
        CellBorder::Left if column == 0 => TableStyleBorder::Left,
        CellBorder::Left => TableStyleBorder::InsideVertical,
        CellBorder::Right if column + 1 == columns => TableStyleBorder::Right,
        CellBorder::Right => TableStyleBorder::InsideVertical,
        CellBorder::Top if row == 0 => TableStyleBorder::Top,
        CellBorder::Top => TableStyleBorder::InsideHorizontal,
        CellBorder::Bottom if row + 1 == rows => TableStyleBorder::Bottom,
        CellBorder::Bottom => TableStyleBorder::InsideHorizontal,
        CellBorder::TopLeftToBottomRight => TableStyleBorder::TopLeftToBottomRight,
        CellBorder::BottomLeftToTopRight => TableStyleBorder::TopRightToBottomLeft,
        // `CellBorder` is `#[non_exhaustive]`; the six edges above are its entire present set, so this
        // is unreachable today — a future edge falls back to an interior vertical rather than panic.
        _ => TableStyleBorder::InsideVertical,
    }
}

/// A table style's text contribution as an interner-free spec: its take on bold/italic (the tri-state
/// [`OnOffStyle`], `Default` contributing nothing) and its text colour, baked to concrete `RRGGBB`.
fn style_text_spec(
    text_style: &TableStyleTextStyle,
    scheme: &SchemeColors,
    map: &ColorMap,
    interner: &Interner,
) -> CharacterPropertiesSpec {
    let mut spec = CharacterPropertiesSpec::new();
    match text_style.bold(interner) {
        OnOffStyle::On => spec = spec.with_bold(true),
        OnOffStyle::Off => spec = spec.with_bold(false),
        OnOffStyle::Default => {}
    }
    match text_style.italic(interner) {
        OnOffStyle::On => spec = spec.with_italic(true),
        OnOffStyle::Off => spec = spec.with_italic(false),
        OnOffStyle::Default => {}
    }
    if let Some(color) = text_style.color(interner) {
        if let Some(resolved) = resolve_color(&color, scheme, map, None, interner) {
            spec = spec.with_fill(FillSpec::Solid(ColorSpec::Srgb(resolved.to_hex())));
        }
    }
    spec
}

/// A candidate shape's own effects, extracted while its part's tree is borrowed (fully owned, so no
/// borrow escapes): an already-resolved effect list, a theme style reference to resolve against the
/// theme, or no effects.
enum OwnEffects {
    /// An explicit `p:spPr > a:effectLst`, already resolved to concrete colors. Boxed — an
    /// [`EffectListSpec`] is far larger than the other variants.
    Resolved(Box<EffectListSpec>),
    /// A `p:style > a:effectRef@idx` with its (already-resolved) `phClr` substitute color.
    StyleRef(u32, Option<ResolvedColor>),
    /// The shape declares no effects of its own.
    Absent,
}

/// The effects a `shape` declares itself (explicit `p:spPr > a:effectLst`, or a `p:style > a:effectRef`),
/// resolved against `scheme` / `map`. The style-reference case returns its index + resolved color for the
/// caller to resolve against the theme (which lives in a different part interner).
fn shape_own_effects(
    shape: &RawElement,
    interner: &Interner,
    scheme: &SchemeColors,
    map: &ColorMap,
) -> Result<OwnEffects, PptxError> {
    if let Some(effect_element) = slide::shape_effects(shape, interner) {
        let effects = EffectList::from_xml(effect_element, interner)?;
        return Ok(OwnEffects::Resolved(Box::new(resolve_effects(
            &effects, scheme, map, None, interner,
        ))));
    }
    if let Some(reference) = slide::shape_effect_ref(shape, interner) {
        if let Some(idx) = reference.index().filter(|idx| *idx > 0) {
            let color = reference
                .color()
                .and_then(|c| resolve_color(c, scheme, map, None, interner));
            return Ok(OwnEffects::StyleRef(idx, color));
        }
    }
    Ok(OwnEffects::Absent)
}
