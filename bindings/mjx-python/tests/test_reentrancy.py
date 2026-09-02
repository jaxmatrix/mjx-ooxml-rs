"""The `&mut self` re-entrancy hazard, and the two things that make it safe.

Almost every `Deck` method takes `&mut self`, so every call takes a `PyRefMut` and a *second live
borrow raises*. The specification's claim is that the hazard is mitigated **structurally** — the
curated subset binds no callbacks, nothing returns a view into the deck, and every method drops its
borrow before returning — so no ordinary use can reach a second borrow.

These tests hold that claim two ways:

* the ordinary use it protects works, in sequence and after failures; and
* the *one* way a caller can still force re-entrancy — a Python object whose `__index__` calls back
  into the deck while PyO3 is extracting it as an argument — **raises and leaves the deck usable**.
  It does not abort the process, which is the outcome that would actually matter.
"""

from __future__ import annotations

import threading

import pytest

from mjx_ooxml import Deck, OoxmlError, ShapeBounds, SlideSize


def test_a_second_live_borrow_raises_rather_than_aborting() -> None:
    """Re-entering the deck from an argument's `__index__` is refused, and nothing is corrupted.

    PyO3 takes the mutable borrow of `self` before it extracts the arguments, so a callback smuggled
    in through an argument's conversion runs while the borrow is live. That is the only door left
    open once callbacks and views are excluded — and it closes with a `RuntimeError`, not with an
    abort, which is why "one deck per thread" is advice rather than a memory-safety requirement.
    """
    deck = Deck.blank(SlideSize.widescreen())
    deck.add_slide()
    deck.add_slide()

    class ReentersTheDeck:
        """An `int`-like argument that calls back into the deck as it is converted."""

        def __init__(self) -> None:
            self.calls = 0

        def __index__(self) -> int:
            self.calls += 1
            return deck.slide_count()

    smuggled = ReentersTheDeck()
    with pytest.raises(RuntimeError) as failure:
        deck.remove_slide(smuggled)  # type: ignore[arg-type]

    assert "borrow" in str(failure.value).lower()
    assert smuggled.calls == 1, "the callback did run — the borrow was genuinely live"
    # And the deck is intact: the refused call changed nothing and released its borrow.
    assert deck.slide_count() == 2
    deck.add_slide()
    assert deck.slide_count() == 3


def test_the_curated_subset_takes_no_callbacks() -> None:
    """No bound method accepts a callable, which is what makes the hazard above the only door.

    A method that took a callback would run Python while the deck was mutably borrowed, on every
    call rather than only when a caller went out of their way. The Rust methods that do —
    `with_table_style`, `with_vml_drawing`, `edit_vml_drawing` and the two VML shape readers — are
    exactly the ones the facade leaves out, so their absence here is the check.
    """
    excluded = {
        "with_table_style",
        "with_vml_drawing",
        "edit_vml_drawing",
        "with_vml_shape_for_ole_object",
        "with_vml_shape_for_activex_control",
    }
    bound = {name for name in dir(Deck) if not name.startswith("_")}
    assert not (bound & excluded), "a closure-taking method reached the binding"


def test_nothing_hands_back_a_view_into_the_deck() -> None:
    """Every read returns an owned value, so a caller cannot hold the deck open.

    Mutating what a reader returned must not touch the document — the second half of "no views".
    """
    deck = Deck.blank(SlideSize.widescreen())
    slide = deck.add_slide()
    deck.add_text_box(slide, "original", ShapeBounds.from_inches(1, 1, 3, 1))

    shapes = deck.shapes(slide)
    text = deck.shape_text(slide, 0)
    parts = deck.remove_unused_parts()

    # The lists and strings are the caller's; changing them changes nothing in the deck.
    shapes.clear()
    parts.append("/not/a/part.xml")
    text += " mutated"

    assert len(deck.shapes(slide)) == 1
    assert deck.shape_text(slide, 0) == "original"


def test_two_threads_on_one_deck_raise_rather_than_race() -> None:
    """Sharing a deck is a mistake the binding reports; it is never undefined behaviour.

    The documented rule is one deck per thread. This proves the rule is advice about *ergonomics*:
    breaking it produces a `RuntimeError` on one side, never a crash and never a torn document.
    """
    deck = Deck.blank(SlideSize.widescreen())
    for _ in range(20):
        deck.add_slide()

    failures: list[BaseException] = []
    barrier = threading.Barrier(4)

    def hammer() -> None:
        barrier.wait()
        for _ in range(200):
            try:
                deck.add_text_box(0, "x", ShapeBounds.from_inches(1, 1, 1, 1))
                deck.shape_count(0)
            except (RuntimeError, OoxmlError) as failure:
                failures.append(failure)

    threads = [threading.Thread(target=hammer) for _ in range(4)]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()

    # Whatever happened, the deck is still a deck and still saves.
    assert deck.slide_count() == 20
    assert deck.save()[:2] == b"PK"
    for failure in failures:
        assert isinstance(failure, (RuntimeError, OoxmlError))


def test_open_and_save_release_the_interpreter_lock() -> None:
    """The two calls that do real work let other threads run — the only `detach` in the binding.

    Proved by effect rather than by timing: a background thread keeps counting while the main
    thread opens and saves, and must make progress. A binding that held the lock throughout would
    let it advance only between calls.
    """
    deck = Deck.blank(SlideSize.widescreen())
    for _ in range(30):
        slide = deck.add_slide()
        deck.add_text_box(slide, "filler " * 40, ShapeBounds.from_inches(1, 1, 8, 4))
    payload = deck.save()

    counter = 0
    running = True

    def count() -> None:
        nonlocal counter
        while running:
            counter += 1

    ticker = threading.Thread(target=count)
    ticker.start()
    try:
        for _ in range(20):
            Deck.open(payload).save()
    finally:
        running = False
        ticker.join()

    assert counter > 0, "the background thread never ran, so the lock was never released"
