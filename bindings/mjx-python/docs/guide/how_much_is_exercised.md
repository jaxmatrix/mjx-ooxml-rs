# How much is exercised

Each of the three walkthroughs exists three times — `crates/mjx-ooxml/examples/build_a_deck.rs`,
`bindings/mjx-python/tests/test_build_a_deck.py`, `bindings/mjx-wasm/tests/node/build_a_deck.mjs`,
and the same for the document and the workbook — and every one compares its output against the Rust
one **part by part, byte for byte**. So does the whole validation catalogue.

That is a strong gate, and it proves the projection is **wired**. It does not prove it is right
**across the surface**, and it is worth saying why in the general form, because the shape recurs:

> Our gates reliably ask whether a value *reaches* somebody. They do not ask *at how many distinct
> points* the surface was ever exercised.

A walkthrough over one deck answers the first question for every method it happens to call and says
nothing at all about the rest. MJXOFF-225 met the same shape one crate over, where a child-order
sweep read as complete while covering three of nine tables. So the question this page answers is the
second one, with a number.

## The number

`xtask/tests/binding_projection.rs` counts it, and prints it on success:

```text
binding surface exercised by its own suite:
  Python        966 of 1619 declared member(s) (59.7%), 653 named by no test
  WebAssembly   900 of 1743 declared member(s) (51.6%), 843 named by no test
```

**Two fifths of the Python surface and half the WebAssembly surface is exercised by nothing.** That
is the finding, and it is larger than the walkthrough framing suggests it would be.

The *other* half of the framing turns out to be wrong, and pleasantly so. Only **18** of the Python
members and **33** of the WebAssembly ones are reached by a walkthrough *and by nothing else* —
about 1% and 2%. The walkthroughs are not what covers this surface. The three parity pairs are:
`test_surface_coverage.py`/`surface.mjs` for PowerPoint,
`test_document_surface_coverage.py`/`document_surface.mjs` for Word, and
`test_workbook_surface_coverage.py`/`workbook_surface.mjs` for Excel. They exist precisely to reach
the corners a walkthrough does not, and they do almost all of the work.

## What "exercised" means, exactly

A declared member is exercised when its name appears in that binding's own test sources in a
position that can only be a use of it: after a `.`, or immediately before a `(` without a `.` in
front, or — in Python — as a keyword argument.

**The two directions of that measure are not equally strong, and the asymmetry is the point.**

* *Exercised* is an **upper bound**. The matcher works on names, so a `.rows` anywhere credits
  `rows` on every class that declares one. The true figure is lower than 966 and lower than 900.
* *Not exercised* is **exact**. No test can call a member whose name appears nowhere in any test
  source. 653 and 843 are floors on the untested surface, not estimates of it.

The number worth quoting is therefore the second one, which is why the gate's failure message prints
the un-exercised set in full rather than the total.

There is a trap inside the instrument, and this unit fell into it while building it. A test that
reaches members through a string — `getattr(spec, flag)`, `spec[builder](value)` — exercises them
perfectly well and names none of them, so the measure reads them as untouched. Both halves of the
Excel pair originally did that for the twelve `CellFormatSpec` attributes and were rewritten to call
each one by name. The rewrite is not cosmetic: a table of names is a table a reader cannot grep, and
a measure that cannot see a call is a measure a future author will quietly defeat.

## Why the count is a test and not a sentence

`MJXOFF-198` states the rule this page is written under: *every count is a fact that expires.* Both
totals in `binding_projection.rs` are asserted **exactly**, not as floors. A floor would let the
exercised share fall one method at a time; an exact pair fails the moment a member is added without
a test *or* a test stops calling one, and the failure names the members. Updating the constants is
then a decision somebody takes on purpose, which is the only property a number in prose can have.

The same file carries the other checkable claim about the projection:
`every_javascript_name_is_the_camel_case_of_its_rust_name` holds all 1,743 exported functions to the
camelCase rule, with a written ledger of the seven that JavaScript itself forces — `toString`, which
is a protocol rather than a name.

## What it does not measure, and what covers that instead

* **Enumeration members.** `bindings/mjx-python/tests/test_enums.py` already holds all 100
  enumerations to their Rust member names in both directions, and
  `bindings/mjx-wasm/tests/node/surface.mjs` checks the two `Format` enumerations member by member.
* **Whether a call was made on the right class.** See the asymmetry above.
* **Whether the answer was right.** That is what the three walkthroughs and the three parity pairs
  are for: every assertion in a parity file is asymmetric on purpose — a test that writes `"Region"`
  into `A1` and reads `"Region"` back out of `A1` passes against a surface that ignores both
  arguments and holds one value.

Closing the gap is not this page's job, but its shape is worth recording for whoever takes it:
a member with no test is not necessarily untested behaviour, because the facade beneath it is tested
in Rust. What is untested is the **wire** — that this name reaches that method with those arguments
in that order. Every mis-wiring the parity pairs have caught has been exactly that.
