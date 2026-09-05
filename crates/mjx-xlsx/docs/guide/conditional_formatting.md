# Conditional formatting

**This library reports which conditional-formatting rules apply to a cell. It never decides whether
a rule's condition is true.** That is the same boundary the
[Formulas and cached values](formulas_and_cached_values) page draws, and for the same reason: a
condition is a formula, deciding it needs a calculation engine, and there is none here and none
planned. What follows is what that means in practice, stated plainly enough that nobody plans around
a behaviour this crate does not have.

## Rules come back as candidates, in priority order

ECMA-376 Part 1 §18.3.1.10: *"The priority of this conditional formatting rule … Lower numeric
values are higher priority than higher numeric values, where 1 is the highest priority."*

A worksheet holds a **list** of `conditionalFormatting` blocks, each with its own range list, and a
priority orders rules *across all of them at once*. So the rules that apply to one cell are gathered
from every block whose range list covers it and sorted together:

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_sml::CellReference;
use mjx_xlsx::Workbook;

let cell = |text: &str| CellReference::parse(text).expect("a reference");
let workbook = Workbook::open(&mjx_fixtures::fixture("conditional_formatting.xlsx"))?;

// B2 is covered by four different blocks. The chain is ordered across all of them, so
// consecutive rules come from *different* blocks — and one block appears at both ends.
let chain = workbook
    .conditional_rules_for(0, cell("B2"), |chain| {
        chain
            .rules()
            .iter()
            .map(|applied| (applied.block_index(), applied.priority()))
            .collect::<Vec<_>>()
    })?
    .expect("the tab reaches a worksheet part");

assert_eq!(chain, vec![(0, 1), (1, 2), (3, 2), (2, 3), (0, 4), (3, 7)]);
# Ok(())
# }
```

Three things in that list are worth naming, because each is a decision:

* **The priorities are the file's own.** `5` and `6` are missing and `2` appears twice, and both are
  left exactly as they are. Excel's own numbering has gaps (a rule was removed) and duplicates (a
  range was copied), and renumbering would change which rule wins.
* **Rules sharing a priority keep document order.** The file states no other order for them.
* **The order is across blocks, never within one.** Sorting each block and concatenating gives
  `1, 4, 2, 3, 2, 7`, which is a different answer.

## `stopIfTrue` is a reported position, not a truncation

§18.3.1.10 again: *"If this flag is 1, no rules with lower priority shall be applied over this rule,
**when this rule evaluates to true**."* The stop is conditional on the rule firing, and nothing here
knows whether it fires — so the chain lists every applicable rule and
`ConditionalRuleChain::first_stopping_rule` says where a consumer that *can* evaluate would first
consider stopping. Truncating the chain would assert that the condition held.

## The conditional layer sits *beside* the base format, never inside it

A rule's `dxfId` names a differential format in `xl/styles.xml`. §18.8.15 says a `dxf` is *"to be
applied on top of or in addition to any formatting already present"* — **when the rule's criteria are
met**. So the two are reported side by side, and there is deliberately no call that merges them:

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_sml::CellReference;
use mjx_xlsx::Workbook;

let cell = |text: &str| CellReference::parse(text).expect("a reference");
let workbook = Workbook::open(&mjx_fixtures::fixture("conditional_formatting.xlsx"))?;

let (base_style, candidates) = workbook
    .conditional_cell_format(0, cell("B2"), |resolved| {
        (
            // What styles.xml says the cell's format is, with no rule taken into account.
            resolved.base().style_index(),
            // And the candidate deltas over it, in priority order.
            resolved
                .layer()
                .iter()
                .map(|entry| (entry.priority(), entry.differential_format_index()))
                .collect::<Vec<_>>(),
        )
    })?
    .expect("the tab reaches both parts");

assert_eq!(base_style, 1);
assert_eq!(
    candidates,
    // A colour scale, a data bar and an icon set draw themselves and name no dxf at all.
    vec![(1, Some(0)), (2, None), (2, Some(0)), (3, None), (4, Some(1)), (7, None)],
);
# Ok(())
# }
```

Every member of a `dxf` is optional, and an absent one means **inherited** — a `dxf` stating only a
fill leaves the font, the border and the number format exactly as the base format has them. That is
why a folded answer would be wrong even if a rule *were* known to fire.

## Authoring a rule appends a `dxf`; it never renumbers one

A rule names its formatting by **index**. So allocating one is an append, and every `@dxfId` already
in the workbook still names what it named:

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_sml::{CellRangeList, ConditionalRuleSpec, DifferentialFormatSpec};
use mjx_ooxml_types::spreadsheetml::ConditionalFormattingOperator;
use mjx_xlsx::Workbook;

let mut workbook = Workbook::open(&mjx_fixtures::fixture("conditional_formatting.xlsx"))?;

// The fixture holds two dxfs, so the next index is 2 — and 0 and 1 are untouched.
let dxf = workbook.append_differential_format(&DifferentialFormatSpec::highlight(
    "9C0006", "FFC7CE",
))?;
assert_eq!(dxf, 2);

workbook.add_conditional_formatting(
    0,
    &CellRangeList::parse("A2:A10").expect("a sqref"),
    &[ConditionalRuleSpec {
        differential_format_index: Some(dxf),
        // The priority is yours. Nothing here derives, renumbers or deduplicates one.
        ..ConditionalRuleSpec::cell_is(
            ConditionalFormattingOperator::Equal,
            ["\"North\"".to_owned()],
            11,
        )
    }],
)?;

// Exactly two parts changed: the worksheet, and xl/styles.xml.
let _ = workbook.save()?;
# Ok(())
# }
```

The rule kinds `ConditionalRuleSpec` describes are the five whose markup is completely determined by
their arguments — `cellIs`, `expression`, `colorScale`, `dataBar` and `iconSet`. The other thirteen
members of `ST_CfType` need attributes the spec would have to guess at (`top10` needs `rank`,
`bottom` and `percent`; `timePeriod` needs `timePeriod`), so they are authored through
`mjx_sml::ConditionalFormattingRule` directly, which states every attribute the schema declares.

## The `x14` extensions are preserved, not modelled

Excel's modern conditional formats — data bars with negative fills, icon-set overrides — live in the
`x14` namespace inside an `extLst`, both on a rule and on the worksheet itself. This crate models
none of them and **loses none of them**: they come back through an unrelated edit byte for byte,
prefix, `uri` and GUIDs included.

## The limits, in one list

| What | Status |
|---|---|
| Which rules apply to a cell | Reported, merged across blocks, in priority order |
| `@priority` | Preserved exactly — gaps and duplicates included, **never renumbered** |
| `stopIfTrue` | Reported as a position in the chain; never applied as a truncation |
| The `dxf` a rule imposes | Reported beside the base format, **never folded into it** |
| `dxfs` | Appended to, never reordered — an existing `@dxfId` cannot be repointed |
| `cfRule/formula` | Text, on the same terms as a cell's formula: never parsed, never rewritten |
| `x14` extensions | Preserved byte for byte; not modelled |
| Whether a rule's condition is **true** | Not answered, and not planned |
