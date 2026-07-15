<!--
mjx-ooxml-rs pull-request template. Fill in each section and delete the comments.
Keep the change atomic and green: build · test --workspace · clippy -D warnings · fmt --check · strict
rustdoc. Commits must have no `Co-Authored-By` / AI-attribution trailers.
-->

## What has been done

<!-- Summarize the change in one or two sentences, then the concrete points as bullets.
     Which crate(s)/phase does this belong to? -->

-

## How it was done

<!-- The approach and the notable design decisions/tradeoffs. Explain WHY the design is correct,
     not just what it does. Call out any fidelity implications (round-trip / byte-identity), layering,
     and performance/memory considerations. -->

-

## How to test it

<!-- The exact commands, and what to look for. Name the specific tests that cover this change. -->

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo doc --workspace --no-deps        # RUSTDOCFLAGS="-D warnings" for the strict check
```

-

## Notes

<!-- Follow-ups, known limitations, deferred work, or anything a reviewer should know.
     Leave a single "-" (or delete this section) if there is nothing to add. -->

-
