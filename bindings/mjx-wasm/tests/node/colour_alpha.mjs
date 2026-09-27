// A colour's opacity, projected (MJXOFF-243, RC04).
//
// The mirror of `bindings/mjx-python/tests/test_colour_alpha.py`. `ColorSpec` already round-trips an
// `a:alpha` as one of the twenty-eight members of `EG_ColorTransform`, which is how a caller
// *writes* one; what it cannot answer is the question a caller *reads* — how opaque is this colour.
//
// The intended surface is one getter, `colorSpec.alpha`: the opacity as a proportion of one, or
// `undefined` for a colour that states none. It mirrors `ResolvedColor.alpha`, which is already a
// number in `0..=1`, so a reader who meets both sees one vocabulary.
//
//     node --test "bindings/mjx-wasm/tests/node/*.mjs"

import assert from "node:assert/strict";
import test from "node:test";

import {
  ColorSpec,
  ColorTransformKind,
  Deck,
  FillSpec,
  Fraction,
  PresetShapeType,
  ShapeBounds,
  SlideSize,
} from "../../npm/dist/bundler/mjx_ooxml.js";

/** The corporate deck's overlay colour and its opacity, which RC03 pinned and RC04 carries. */
const OVERLAY = "1F3864";
const OVERLAY_ALPHA = 0.35;

/** How close two opacities have to be: a hundredth of a per cent is far below a visible step. */
const TOLERANCE = 1e-4;

/** A blank deck with one slide, plus the freeing every caller owes. */
function withDeck(body) {
  const size = SlideSize.widescreen();
  const deck = Deck.blank(size);
  size.free();
  try {
    deck.addSlide();
    return body(deck);
  } finally {
    deck.free();
  }
}

/** A rectangle filled with `OVERLAY` at `ratio`, and the shape index it was added at. */
function translucentShape(deck, ratio, left) {
  const bounds = ShapeBounds.fromInches(left, 1, 4, 3);
  const shape = deck.addShape(0, PresetShapeType.Rectangle, bounds);
  bounds.free();
  const fraction = Fraction.of(ratio);
  const colour = ColorSpec.srgb(OVERLAY).withAlpha(fraction);
  const fill = FillSpec.solid(colour);
  deck.setShapeFill(0, shape, fill);
  fill.free();
  colour.free();
  fraction.free();
  return shape;
}

/** The opacity a shape's effective fill reports. */
function reportedAlpha(deck, shape) {
  const fill = deck.effectiveShapeFill(0, shape);
  assert.ok(fill !== undefined, "the shape states a fill");
  const colour = fill.color;
  assert.ok(colour !== undefined, "the fill states a colour");
  const alpha = colour.alpha;
  const hex = colour.srgbValue;
  colour.free();
  fill.free();
  assert.equal(hex, OVERLAY, "the resolved colour is no longer the triplet the slide states");
  return alpha;
}

test("a colour reports the opacity it carries", () => {
  for (const ratio of [0, 0.2, OVERLAY_ALPHA, 1]) {
    const fraction = Fraction.of(ratio);
    const colour = ColorSpec.srgb(OVERLAY).withAlpha(fraction);
    const reported = colour.alpha;
    colour.free();
    fraction.free();
    assert.ok(
      Math.abs(reported - ratio) < TOLERANCE,
      `a colour built with an alpha of ${ratio} reports ${reported}`,
    );
  }
});

test("a colour that states no opacity reports undefined", () => {
  const colour = ColorSpec.srgb(OVERLAY);
  // The getter has to exist for `undefined` to mean anything: a property that was never declared
  // reads as `undefined` too, which would make this case pass on a binding that projects nothing.
  const declared = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(colour), "alpha");
  const reported = colour.alpha;
  colour.free();
  assert.ok(declared !== undefined, "`ColorSpec` declares no `alpha` accessor at all");
  assert.equal(
    reported,
    undefined,
    "`undefined` rather than 1, so a caller can tell a stated opacity from an absent one",
  );
});

test("the opacity is still one of the transforms", () => {
  const fraction = Fraction.of(OVERLAY_ALPHA);
  const colour = ColorSpec.srgb(OVERLAY).withAlpha(fraction);
  const transforms = colour.transforms;
  assert.equal(transforms.length, 1);
  assert.equal(transforms[0].kind, ColorTransformKind.Alpha);
  const carried = transforms[0].percentageValue;
  assert.ok(carried !== undefined, "an `a:alpha` carries a percentage");
  assert.ok(Math.abs(carried.ratio - OVERLAY_ALPHA) < TOLERANCE);
  carried.free();
  for (const transform of transforms) {
    transform.free();
  }
  colour.free();
  fraction.free();
});

test("a resolved fill reports the opacity the slide states", () => {
  withDeck((deck) => {
    const shape = translucentShape(deck, OVERLAY_ALPHA, 1);
    const reported = reportedAlpha(deck, shape);
    assert.ok(
      Math.abs(reported - OVERLAY_ALPHA) < TOLERANCE,
      `the shape is filled at 35 % and the resolved fill reports ${reported}; an opaque answer ` +
        "here is a band painted as a slab over whatever is beneath it",
    );
  });
});

test("two shapes at two opacities do not collapse", () => {
  withDeck((deck) => {
    const first = translucentShape(deck, 0.25, 1);
    const second = translucentShape(deck, 0.75, 6);
    const reported = [reportedAlpha(deck, first), reportedAlpha(deck, second)];
    assert.ok(
      Math.abs(reported[0] - 0.25) < TOLERANCE && Math.abs(reported[1] - 0.75) < TOLERANCE,
      `the two shapes state 0.25 and 0.75 and the deck reports ${reported}`,
    );
  });
});
