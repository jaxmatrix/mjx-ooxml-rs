//! One-off measurement for MJX-248 (subtree copy-on-write). Not a benchmark harness — B4 owns that.
use std::sync::Arc;
use std::time::Instant;

use mjx_ooxml_core::{RawAttribute, RawDocument, RawElement, RawName, RawNode};
use mjx_xml::fidelity;

/// The old, span-free layout, for a like-for-like `size_of` comparison.
#[allow(dead_code)]
struct OldRawElement {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fn build_large_part(shapes: usize) -> Vec<u8> {
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n\
         <p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"\r\n  \
         xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"\r\n  \
         xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">\r\n<p:cSld><p:spTree>\r\n",
    );
    for i in 0..shapes {
        s.push_str(&format!(
"<p:sp>\r\n  <p:nvSpPr>\r\n    <p:cNvPr id=\"{id}\" name=\"Shape {i}\"\r\n      descr=\"generated\"/>\r\n    <p:cNvSpPr/>\r\n    <p:nvPr/>\r\n  </p:nvSpPr>\r\n  \
<p:spPr>\r\n    <a:xfrm rot=\"0\"\r\n      flipH=\"0\" flipV=\"0\">\r\n      <a:off x=\"{x}\" y=\"{y}\"/>\r\n      <a:ext cx=\"1828800\" cy=\"1143000\"/>\r\n    </a:xfrm>\r\n    \
<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>\r\n    <a:solidFill><a:srgbClr val=\"4472C4\"/></a:solidFill>\r\n  </p:spPr>\r\n  \
<p:txBody>\r\n    <a:bodyPr/>\r\n    <a:lstStyle/>\r\n    <a:p><a:r><a:rPr lang=\"en-US\" sz=\"1800\"\r\n      b=\"1\" dirty=\"0\"/><a:t>Item {i} &#38; more</a:t></a:r></a:p>\r\n  </p:txBody>\r\n</p:sp>\r\n",
            id = i + 2, x = i * 1000, y = i * 700));
    }
    s.push_str("</p:spTree></p:cSld><p:clrMapOvr/></p:sld>\r\n");
    s.into_bytes()
}

/// `(nodes, elements)` — only elements carry a span.
fn count(element: &RawElement) -> (usize, usize) {
    let (mut nodes, mut elements) = (1, 1);
    for node in element.children.iter() {
        match node {
            RawNode::Element(child) => {
                let (n, e) = count(child);
                nodes += n;
                elements += e;
            }
            _ => nodes += 1,
        }
    }
    (nodes, elements)
}

/// The index in `element.children` of the `nth` child element with local name `local`.
fn child_index(
    element: &RawElement,
    interner: &mjx_ooxml_core::Interner,
    local: &str,
    nth: usize,
) -> usize {
    let mut seen = 0;
    for (index, node) in element.children.iter().enumerate() {
        if let RawNode::Element(child) = node {
            if interner.resolve(child.name.local) == local {
                if seen == nth {
                    return index;
                }
                seen += 1;
            }
        }
    }
    panic!("no <{local}> #{nth}")
}

fn time<T>(label: &str, runs: u32, mut f: impl FnMut() -> T) -> T {
    // Warm up, then take the best of `runs`.
    let mut out = f();
    let mut best = f64::INFINITY;
    for _ in 0..runs {
        let t = Instant::now();
        out = f();
        best = best.min(t.elapsed().as_secs_f64() * 1e3);
    }
    println!("  {label:<46} {best:8.3} ms");
    out
}

fn main() {
    println!(
        "size_of::<RawElement>()  old {:>3} B   new {:>3} B   (+{} B/element)",
        std::mem::size_of::<OldRawElement>(),
        std::mem::size_of::<RawElement>(),
        std::mem::size_of::<RawElement>() - std::mem::size_of::<OldRawElement>()
    );

    for shapes in [200usize, 4000] {
        let source = build_large_part(shapes);
        let shared: Arc<[u8]> = Arc::from(source.as_slice());
        let doc = fidelity::parse_shared(Arc::clone(&shared)).expect("parse");
        let (nodes, elements) = count(&doc.root);
        println!(
            "\n{shapes} shapes - {src} KiB source, {nodes} nodes / {elements} elements",
            src = source.len() / 1024
        );
        println!(
            "  spans {} KiB (8 B x elements); source buffer {} KiB, shared with the part's own \
             bytes unless the part is edited",
            elements * 8 / 1024,
            source.len() / 1024
        );

        time("parse", 20, || {
            fidelity::parse_shared(Arc::clone(&shared)).expect("parse")
        });

        let out = time("serialize, untouched (all verbatim)", 20, || {
            fidelity::serialize_to_vec(&doc)
        });
        assert_eq!(out, source, "untouched must be byte-identical");

        // One attribute of one element, in the middle of the part.
        let mut edited = fidelity::parse_shared(Arc::clone(&shared)).expect("parse");
        {
            // One attribute of one `a:xfrm`, on the shape in the middle of the part.
            let RawDocument { interner, root, .. } = &mut edited;
            let mut node = &mut *root;
            for step in ["cSld", "spTree"] {
                let index = child_index(node, interner, step, 0);
                let RawNode::Element(next) = &mut node.children[index] else {
                    unreachable!()
                };
                node = next;
            }
            let shapes = node
                .children
                .iter()
                .filter(|n| matches!(n, RawNode::Element(_)))
                .count();
            let index = child_index(node, interner, "sp", shapes / 2);
            let RawNode::Element(sp) = &mut node.children[index] else {
                unreachable!()
            };
            let index = child_index(sp, interner, "spPr", 0);
            let RawNode::Element(sp_pr) = &mut sp.children[index] else {
                unreachable!()
            };
            let index = child_index(sp_pr, interner, "xfrm", 0);
            let RawNode::Element(xfrm) = &mut sp_pr.children[index] else {
                unreachable!()
            };
            xfrm.attributes[0].value = Box::from(&b"5400000"[..]);
        }
        time("serialize, one attribute edited", 20, || {
            fidelity::serialize_to_vec(&edited)
        });

        // The pre-MJX-248 behaviour: no source buffer, so every element is rebuilt.
        let mut rebuilt = fidelity::parse_shared(Arc::clone(&shared)).expect("parse");
        rebuilt.release_source();
        time("serialize, fully reconstructed (old path)", 20, || {
            fidelity::serialize_to_vec(&rebuilt)
        });
    }
}
