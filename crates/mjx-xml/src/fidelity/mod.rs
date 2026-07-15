//! Byte-preserving fidelity layer: parse a part into the [`mjx_ooxml_core::RawDocument`] tree and
//! serialize it back. Targets byte-identical round-trips for clean Office/fixture XML, with
//! canonical-XML equality as the guaranteed contract for the general case.

mod reader;
mod writer;

pub use reader::parse;
pub use writer::{serialize, serialize_to_vec};

#[cfg(test)]
mod tests {
    use super::{parse, serialize_to_vec};

    /// Asserts that parsing then serializing reproduces the input byte-for-byte.
    #[track_caller]
    fn assert_round_trips(xml: &[u8]) {
        let doc = parse(xml).expect("parse");
        let out = serialize_to_vec(&doc);
        assert_eq!(
            String::from_utf8_lossy(&out),
            String::from_utf8_lossy(xml),
            "round-trip mismatch"
        );
    }

    #[test]
    fn declaration_and_root() {
        assert_round_trips(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><root/>"#);
    }

    #[test]
    fn self_closing_vs_explicit_empty() {
        assert_round_trips(b"<a><b/><c></c></a>");
    }

    #[test]
    fn attribute_order_quotes_and_entities_preserved() {
        // attribute order kept; entity in value NOT unescaped; single-quote preserved.
        assert_round_trips(br#"<w:p xmlns:w="urn:w" w:one="a &amp; b" w:two='x&lt;y'/>"#);
    }

    #[test]
    fn text_entities_and_whitespace_preserved() {
        assert_round_trips(b"<a>  1 &lt; 2  \n  <b>x</b>\n</a>");
    }

    #[test]
    fn cdata_comment_and_pi() {
        assert_round_trips(b"<a><![CDATA[ raw <b> & ]]><!-- c --><?pi data?></a>");
    }

    #[test]
    fn namespace_prefixes_and_xmlns_order() {
        assert_round_trips(
            br#"<p:sld xmlns:p="urn:p" xmlns:a="urn:a"><p:cSld><a:off x="1" y="2"/></p:cSld></p:sld>"#,
        );
    }

    #[test]
    fn trailing_newline_epilogue() {
        assert_round_trips(b"<?xml version=\"1.0\"?>\n<root>text</root>\n");
    }

    #[test]
    fn byte_order_mark_preserved() {
        assert_round_trips(b"\xEF\xBB\xBF<root/>");
    }

    #[test]
    fn rejects_unbalanced() {
        assert!(parse(b"<a><b></a>").is_err());
        assert!(parse(b"not xml at all").is_err());
    }
}
