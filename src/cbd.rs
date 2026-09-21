use std::collections::HashSet;

use oxrdf::{NamedOrBlankNode, Term, Triple};

/// A resource whose Concise Bounded Description [`describe_cbd`] should
/// extract: `iri` itself, plus any `satellites` (e.g. a class's applicable
/// properties, so the class's served RDF representation also carries each
/// property's own label/comment/domain triples) folded into the same
/// description. `satellites` is empty for a resource that owns all its own
/// triples directly (e.g. a plain data individual).
pub struct ResourceKey<'a> {
  pub iri: &'a str,
  pub satellites: &'a [String],
}

/// Every triple in `triples` whose subject is `key.iri` or one of
/// `key.satellites`, and whose object is independently nameable (a
/// `NamedNode` or `Literal`). Blank-node objects — e.g. an
/// `owl:Restriction` cardinality block — are excluded: a blank node has no
/// stable IRI a served RDF representation could usefully point at.
pub fn describe_cbd(triples: &[Triple], key: &ResourceKey) -> Vec<Triple> {
  let subjects: HashSet<&str> = std::iter::once(key.iri).chain(key.satellites.iter().map(String::as_str)).collect();
  triples
    .iter()
    .filter(|t| {
      let NamedOrBlankNode::NamedNode(s) = &t.subject else { return false };
      subjects.contains(s.as_str()) && !matches!(&t.object, Term::BlankNode(_))
    })
    .cloned()
    .collect()
}
