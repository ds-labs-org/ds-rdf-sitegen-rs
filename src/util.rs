use oxrdf::{NamedNode, NamedOrBlankNode};

/// Wraps `iri` as a `NamedOrBlankNode` for map lookups. Panics on an
/// invalid IRI — every caller passes an IRI string that was itself derived
/// from a parsed triple or a constant, never untrusted input.
pub(crate) fn named(iri: &str) -> NamedOrBlankNode {
  NamedOrBlankNode::NamedNode(NamedNode::new(iri).unwrap_or_else(|e| panic!("invalid IRI {iri:?}: {e}")))
}

/// The IRI string of `node`, or `None` for a blank node (no stable IRI).
pub(crate) fn iri_str(node: &NamedOrBlankNode) -> Option<String> {
  match node {
    NamedOrBlankNode::NamedNode(n) => Some(n.as_str().to_string()),
    NamedOrBlankNode::BlankNode(_) => None,
  }
}
