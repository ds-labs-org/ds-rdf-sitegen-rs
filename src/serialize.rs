use oxjsonld::JsonLdSerializer;
use oxrdf::{GraphNameRef, Triple, TripleRef};
use oxttl::TurtleSerializer;

/// A `(prefix, IRI)` table shared by both [`to_turtle`] and [`to_jsonld`],
/// so a served RDF representation reads with the same prefixes as the
/// source ontology rather than falling back to bare angle-bracket IRIs.
pub struct Prefixes<'a>(pub &'a [(&'a str, &'a str)]);

/// Serializes `triples` as Turtle, with `prefixes.0` declared as `@prefix`
/// lines.
pub fn to_turtle(triples: &[Triple], prefixes: &Prefixes) -> Vec<u8> {
  let mut builder = TurtleSerializer::new();
  for (name, iri) in prefixes.0 {
    builder = builder.with_prefix(*name, *iri).expect("Prefixes entries are valid IRIs");
  }
  let mut w = builder.for_writer(Vec::new());
  for t in triples {
    w.serialize_triple(t).expect("failed to serialize a triple to Turtle");
  }
  w.finish().expect("failed to finish Turtle serialization")
}

/// Serializes `triples` as JSON-LD, with `prefixes.0` declared as `@context`
/// entries.
pub fn to_jsonld(triples: &[Triple], prefixes: &Prefixes) -> Vec<u8> {
  let mut builder = JsonLdSerializer::new();
  for (name, iri) in prefixes.0 {
    builder = builder.with_prefix(*name, *iri).expect("Prefixes entries are valid IRIs");
  }
  let mut w = builder.for_writer(Vec::new());
  for t in triples {
    w.serialize_quad(TripleRef::from(t).in_graph(GraphNameRef::DefaultGraph)).expect("failed to serialize a triple to JSON-LD");
  }
  w.finish().expect("failed to finish JSON-LD serialization")
}
