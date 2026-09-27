"""A colour's opacity, projected (MJXOFF-243, RC04).

`ColorSpec` already round-trips an `a:alpha` as one of the twenty-eight members of
`EG_ColorTransform`, which is how a caller *writes* one. What it cannot answer is the question a
caller *reads*: how opaque is this colour. Before RC04 the answer did not exist anywhere below the
binding either — resolution baked every colour to a six-digit triplet — so there was nothing to
project.

The intended surface is one property, `ColorSpec.alpha`: the opacity as a proportion of one, or
`None` for a colour that states none. It mirrors `ResolvedColor.alpha`, which has always been a
`float` in `0.0..=1.0`, so a reader who meets both sees one vocabulary.

Every assertion names a concrete number, and the suite states four different opacities — fully
opaque, fully transparent, and two different mid values — because a colour whose alpha is always the
same is reached at one value and proves nothing.
"""

from __future__ import annotations

import pytest

from mjx_ooxml import (
    ColorSpec,
    ColorTransformKind,
    Deck,
    FillSpec,
    Fraction,
    PresetShapeType,
    ShapeBounds,
    SlideSize,
)

# The corporate deck's overlay colour and its opacity, which RC03 pinned and RC04 carries.
OVERLAY = "1F3864"
OVERLAY_ALPHA = 0.35


def test_a_colour_reports_the_opacity_it_carries() -> None:
    """The one property this ticket adds, at four different values."""
    for ratio in (0.0, 0.2, OVERLAY_ALPHA, 1.0):
        colour = ColorSpec.srgb(OVERLAY).with_alpha(Fraction.of(ratio))
        assert colour.alpha == pytest.approx(ratio), (
            f"a colour built with an alpha of {ratio} reports {colour.alpha}"
        )


def test_a_colour_that_states_no_opacity_reports_none() -> None:
    """`None` rather than `1.0`, so a caller can tell a stated opacity from an absent one."""
    assert ColorSpec.srgb(OVERLAY).alpha is None


def test_the_opacity_is_still_one_of_the_transforms() -> None:
    """The existing surface is unchanged: the alpha is a transform as well as a property."""
    colour = ColorSpec.srgb(OVERLAY).with_alpha(Fraction.of(OVERLAY_ALPHA))
    transforms = colour.transforms
    assert [transform.kind for transform in transforms] == [ColorTransformKind.Alpha]
    carried = transforms[0].percentage_value
    assert carried is not None
    assert carried.ratio == pytest.approx(OVERLAY_ALPHA)
    assert colour.srgb_value == OVERLAY, "the colour underneath is still a plain triplet"


def test_a_resolved_fill_reports_the_opacity_the_slide_states() -> None:
    """The whole way through: a deck's own shape, read back through the effective-fill ladder."""
    deck = Deck.blank(SlideSize.widescreen())
    deck.add_slide()
    shape = deck.add_shape(0, PresetShapeType.Rectangle, ShapeBounds.from_inches(1, 1, 4, 3))
    deck.set_shape_fill(
        0,
        shape,
        FillSpec.solid(ColorSpec.srgb(OVERLAY).with_alpha(Fraction.of(OVERLAY_ALPHA))),
    )

    fill = deck.effective_shape_fill(0, shape)
    assert fill is not None
    colour = fill.color
    assert colour is not None
    assert colour.srgb_value == OVERLAY, (
        "the resolved colour is no longer the triplet the slide states"
    )
    assert colour.alpha == pytest.approx(OVERLAY_ALPHA), (
        "the shape is filled at 35 % and the resolved fill reports "
        f"{colour.alpha}; an opaque answer here is a band painted as a slab over whatever is "
        "beneath it"
    )


def test_two_shapes_at_two_opacities_do_not_collapse() -> None:
    """Two different mid values on one slide, which a stuck-at-one-value build cannot produce."""
    deck = Deck.blank(SlideSize.widescreen())
    deck.add_slide()
    opacities = (0.25, 0.75)
    shapes = []
    for index, ratio in enumerate(opacities):
        shape = deck.add_shape(
            0, PresetShapeType.Rectangle, ShapeBounds.from_inches(1 + 5 * index, 1, 4, 3)
        )
        deck.set_shape_fill(
            0, shape, FillSpec.solid(ColorSpec.srgb(OVERLAY).with_alpha(Fraction.of(ratio)))
        )
        shapes.append(shape)

    reported = []
    for shape in shapes:
        fill = deck.effective_shape_fill(0, shape)
        assert fill is not None
        colour = fill.color
        assert colour is not None
        reported.append(colour.alpha)

    assert reported == pytest.approx(list(opacities)), (
        f"the two shapes state {opacities} and the deck reports {reported}"
    )
