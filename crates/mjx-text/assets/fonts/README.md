# The bundled metric-compatible substitute set (tier 2)

`docs/UI_PLATFORM_PLAN.md` §10 resolves a font through three tiers: the platform's own faces, then
**these**, then a lazily fetched subset. This directory is tier 2 — the faces an application ships so
that a document naming a Microsoft font still paginates the way Office paginates it. **iOS never
reaches tier 1** (the sandbox does not expose system font files, and reaching them would need
CoreText FFI, which breaks the pure-Rust rule), so on that platform these files are the *first* tier
that can answer, which is why the set is held to the metric-compatibility bar rather than a
"looks similar" one.

They are also the corpus `tests/metric_compatibility.rs` measures. That test does **not** ask whether
a face agrees with itself; it asserts these faces' advance widths against *published metrics of the
Microsoft originals*, sourced outside this repository and cited line by line in
`src/reference.rs`.

## What is here, and where each file came from

| File | Family | Version / provenance | SHA-256 | Licence |
|---|---|---|---|---|
| `Carlito-Regular.ttf` | Carlito | Arch Linux `ttf-carlito` 20230509-2, upstream <https://github.com/googlefonts/carlito> | `f6418f708baede9789daef5d458c0f53d2a888af9820e8062934e504fedc6595` | SIL OFL 1.1 — `LICENSE-Carlito-OFL.txt` |
| `Caladea-Regular.ttf` | Caladea | <https://github.com/huertatipografica/Caladea> `master`, `fonts/ttf/Caladea-Regular.ttf` | `f1e899278b7b4491aba5b6a8253c4b04c050cc59b21865be5c37559a775153cd` | SIL OFL 1.1 — `LICENSE-Caladea-OFL.txt` |
| `LiberationSans-Regular.ttf` | Liberation Sans | Arch Linux `ttf-liberation` 2.1.5-2, upstream <https://github.com/liberationfonts/liberation-fonts> | `baccc64becc3eb7d104b7c84d99f5314a0a1f896e2b3ea6c2f22fc08d2003bee` | SIL OFL 1.1 — `LICENSE-Liberation-OFL.txt` |
| `LiberationSerif-Regular.ttf` | Liberation Serif | as above | `86b9ea1c2f41bed9d7c09ccad4abc2894b33df5de60e5bbbece5d48610911870` | as above |
| `LiberationMono-Regular.ttf` | Liberation Mono | as above | `47ed5b5fcfe6b3c9228937b05de9c769f5fa55b777d539af0f65172f0a24c90b` | as above |

Every file here is under the **SIL Open Font License, Version 1.1**, and each licence text sits
beside the faces it covers, which is what the OFL requires of a redistribution. Note that
`docs/UI_PLATFORM_PLAN.md` and the tracker describe Caladea as Apache-2.0; that was true of Google's
2013 *crosextrafonts* release, and the upstream project has since relicensed to the OFL. The text
committed here is the one shipped with the file committed here.

None of these fonts is modified, and none is renamed — the OFL's reserved-font-name clauses are
therefore not engaged.

## Why only the regular styles

These five files are the regular weight of each family, 1.8 MB in total. A complete tier 2 also wants
each family's bold, italic and bold-italic — fifteen more files, roughly six further megabytes of
binaries in git. **That is a packaging decision for the repository's owner rather than one this crate
should take on its own**, so it is recorded on MJXOFF-157 and not taken here.

Nothing is broken meanwhile, and nothing is stubbed: [`crate::FontResolver`] applies the CSS
font-matching rules for style and weight, so a request for bold Calibri on a machine with no bold
Carlito resolves to the regular face and the substitution manifest records the style it actually
got. That path exists whatever this directory holds — a system tier can be missing a style just as
easily as a bundle can — so serving it from a smaller bundle is a smaller bundle, not a hole.
