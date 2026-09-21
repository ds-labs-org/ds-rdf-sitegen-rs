use std::collections::HashSet;

use oxrdf::{NamedOrBlankNode, Term, Triple};

use crate::{OWL_CLASS, OWL_DATATYPE_PROPERTY, OWL_OBJECT_PROPERTY, RDF_PROPERTY, RDF_TYPE};

/// Every distinct `NamedNode` subject IRI under `namespace`, in
/// first-appearance order in `triples`. This is the resource-discovery
/// primitive both "which vocabulary terms exist" ([`vocabulary_terms`]) and
/// "which data individuals actually exist yet" (a data IRI that's only
/// ever an *object* — a forward reference to something not authored yet —
/// is simply absent from this list, deferred until it does appear as a
/// subject) are built from.
pub fn subjects_in_namespace(triples: &[Triple], namespace: &str) -> Vec<String> {
  let mut seen = HashSet::new();
  let mut ordered = Vec::new();
  for t in triples {
    let NamedOrBlankNode::NamedNode(s) = &t.subject else { continue };
    let iri = s.as_str();
    if iri.starts_with(namespace) && seen.insert(iri.to_string()) {
      ordered.push(iri.to_string());
    }
  }
  ordered
}

/// Subset of [`subjects_in_namespace`] whose `rdf:type` is `owl:Class`,
/// `owl:ObjectProperty`, `owl:DatatypeProperty`, or `rdf:Property` — i.e.
/// vocabulary terms (a namespace's "schema"), as opposed to data
/// individuals.
pub fn vocabulary_terms(triples: &[Triple], namespace: &str) -> Vec<String> {
  const TERM_TYPES: [&str; 4] = [OWL_CLASS, OWL_OBJECT_PROPERTY, OWL_DATATYPE_PROPERTY, RDF_PROPERTY];
  subjects_in_namespace(triples, namespace)
    .into_iter()
    .filter(|iri| {
      triples.iter().any(|t| {
        let NamedOrBlankNode::NamedNode(s) = &t.subject else { return false };
        let Term::NamedNode(o) = &t.object else { return false };
        s.as_str() == iri && t.predicate.as_str() == RDF_TYPE && TERM_TYPES.contains(&o.as_str())
      })
    })
    .collect()
}
