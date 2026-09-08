// The one page of the bindings' guide set that is about this binding alone, hosted by the crate it
// is about because `mjx-python` and `mjx-wasm` are siblings and neither may see the other — the same
// arrangement `mjx-mce` and `mjx-vml` have with the sets they belong to.
//
// The other four pages are `mjx_python::guide`'s.
#![doc = include_str!("../docs/guide/the_typescript_surface.md")]
