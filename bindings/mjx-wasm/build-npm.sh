#!/usr/bin/env bash
# Builds the one npm package, with both wasm targets inside it.
#
# `wasm-pack` emits a whole package per target, and a project that needs both — a bundler build for
# Vite/webpack/Node, an ESM build for a browser importing directly — would otherwise install two
# packages that disagree about their own name. So both are built into `npm/dist/`, and the committed
# `npm/package.json` points its conditional exports at them:
#
#     import { Deck } from "@mjx/ooxml";          # bundler build, via "default"
#     import { Deck } from "@mjx/ooxml/web";      # ESM build, for a browser with no bundler
#
# Run from anywhere:  bindings/mjx-wasm/build-npm.sh
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

# The npm package states its own version, and a package that disagrees with the crate it was built
# from is a package nobody can trace back to a commit. Checked rather than rewritten, so the release
# bump stays one deliberate edit per file.
crate_version=$(grep -m1 '^version = ' ../../Cargo.toml | cut -d'"' -f2)
npm_version=$(grep -m1 '"version"' npm/package.json | cut -d'"' -f4)
if [ "$crate_version" != "$npm_version" ]; then
    echo "npm/package.json says $npm_version but the workspace is at $crate_version." >&2
    echo "Update npm/package.json to match, then run this again." >&2
    exit 1
fi

rm -rf npm/dist
for target in bundler web; do
    echo "==> wasm-pack build --target $target"
    wasm-pack build --release --target "$target" --out-dir "npm/dist/$target" --out-name mjx_ooxml
    # `wasm-pack` writes a package manifest per target; the committed one at `npm/package.json` is
    # the package, and these would shadow it for tools that walk upwards.
    rm -f "npm/dist/$target/package.json" "npm/dist/$target/.gitignore" "npm/dist/$target/README.md"
done

cp README.md npm/README.md
echo
echo "==> built:"
find npm/dist -type f | sort
echo
raw=$(stat -c %s npm/dist/bundler/mjx_ooxml_bg.wasm)
gzipped=$(gzip -9 -c npm/dist/bundler/mjx_ooxml_bg.wasm | wc -c)
printf '==> wasm payload: %s bytes raw, %s bytes gzipped (%s KiB)\n' \
    "$raw" "$gzipped" "$((gzipped / 1024))"
