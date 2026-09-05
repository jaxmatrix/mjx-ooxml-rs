//! The shared string table: `sharedStrings.xml`, rich-text runs and inline strings.
//!
//! **Filled by MJXOFF-97 (D05).** Nothing here yet — this child (MJXOFF-132) creates the crate and
//! the tree, and models nothing.
//!
//! What belongs here: `CT_Sst` and its `CT_Rst` entries, the `CT_RElt` rich-text runs and their
//! `CT_RPrElt` properties, the phonetic members a CJK workbook carries, and the interning that makes
//! a shared string an index rather than a copy — `CLAUDE.md`'s "interning + `Cow` for strings",
//! applied to the one table in OOXML that exists solely to deduplicate text. The inline-string form
//! (`x:is`, a cell holding its own `CT_Rst`) is modelled here too, because it is the same type
//! reached from [`crate::cells`].
