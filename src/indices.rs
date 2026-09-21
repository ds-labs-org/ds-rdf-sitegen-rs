use std::collections::HashMap;

use oxrdf::{NamedOrBlankNode, Term};

use crate::ontology::Ontology;
use crate::util::{iri_str, named};
use crate::{OWL_MIN_CARDINALITY, OWL_ON_PROPERTY, RDF_FIRST, RDF_NIL, RDF_REST, RDFS_DOMAIN, RDFS_SUBCLASS_OF, UNTAGGED};

/// Language tag ("" for an untagged/plain literal) -> literal value.
pub type LocalizedValues = HashMap<String, String>;

/// Generic, queryable indices built once from an [`Ontology`]'s flat triple
/// list, reused by every resource a caller emits RDF/codegen for.
///
/// Every literal-valued triple (regardless of predicate) is indexed by
/// `subject -> predicate -> lang -> value`, and every IRI-valued triple by
/// `subject -> predicate -> [object IRI]` — so `rdfs:label`, `rdfs:comment`,
/// a site-specific `ds:body`, `rdfs:domain`, `rdfs:subClassOf`,
/// `rdfs:seeAlso`, etc. all fall out of the same two maps instead of each
/// needing its own hand-rolled pass.
pub struct Indices {
  /// The ontology's full triple list (cloned from the source [`Ontology`]),
  /// kept around for [`Indices::rdf_list`] (which needs to find a
  /// specific subject/predicate's object among the raw triples, including
  /// blank-node objects the `links` map deliberately excludes) and for
  /// passing to [`crate::describe_cbd`]/[`crate::subjects_in_namespace`].
  pub triples: Vec<oxrdf::Triple>,
  literals: HashMap<NamedOrBlankNode, HashMap<String, LocalizedValues>>,
  links: HashMap<NamedOrBlankNode, HashMap<String, Vec<String>>>,
  first: HashMap<NamedOrBlankNode, Term>,
  rest: HashMap<NamedOrBlankNode, NamedOrBlankNode>,
  /// `(class IRI, required property IRI)` pairs, derived from
  /// `<class> rdfs:subClassOf [ a owl:Restriction ; owl:onProperty <prop> ;
  /// owl:minCardinality N>=1 ]`.
  pub restrictions: Vec<(String, String)>,
}

impl Indices {
  /// Builds every index in one pass over `ontology.triples`.
  pub fn build(ontology: &Ontology) -> Indices {
    let mut literals: HashMap<NamedOrBlankNode, HashMap<String, LocalizedValues>> = HashMap::new();
    let mut links: HashMap<NamedOrBlankNode, HashMap<String, Vec<String>>> = HashMap::new();
    let mut first: HashMap<NamedOrBlankNode, Term> = HashMap::new();
    let mut rest: HashMap<NamedOrBlankNode, NamedOrBlankNode> = HashMap::new();
    let mut restriction_property: HashMap<NamedOrBlankNode, String> = HashMap::new();
    let mut restriction_min_card: HashMap<NamedOrBlankNode, i64> = HashMap::new();

    for t in &ontology.triples {
      match &t.object {
        Term::Literal(lit) => {
          let lang = lit.language().unwrap_or(UNTAGGED).to_string();
          literals
            .entry(t.subject.clone())
            .or_default()
            .entry(t.predicate.as_str().to_string())
            .or_default()
            .insert(lang, lit.value().to_string());
        }
        Term::NamedNode(n) => {
          links
            .entry(t.subject.clone())
            .or_default()
            .entry(t.predicate.as_str().to_string())
            .or_default()
            .push(n.as_str().to_string());
        }
        Term::BlankNode(_) => {}
      }

      match t.predicate.as_str() {
        RDF_FIRST => {
          first.insert(t.subject.clone(), t.object.clone());
        }
        RDF_REST => {
          let next = match &t.object {
            Term::BlankNode(b) => Some(NamedOrBlankNode::BlankNode(b.clone())),
            Term::NamedNode(n) => Some(NamedOrBlankNode::NamedNode(n.clone())),
            _ => None,
          };
          if let Some(next) = next {
            rest.insert(t.subject.clone(), next);
          }
        }
        OWL_ON_PROPERTY => {
          if let Term::NamedNode(p) = &t.object {
            restriction_property.insert(t.subject.clone(), p.as_str().to_string());
          }
        }
        OWL_MIN_CARDINALITY => {
          if let Term::Literal(lit) = &t.object
            && let Ok(n) = lit.value().parse::<i64>()
          {
            restriction_min_card.insert(t.subject.clone(), n);
          }
        }
        _ => {}
      }
    }

    // Restriction blank nodes attached via rdfs:subClassOf need the raw
    // triples walked a second time: `subclass_of`'s NamedNode-only match
    // (above, feeding `links`) skips exactly these blank-node objects.
    let mut restrictions: Vec<(String, String)> = Vec::new();
    for t in &ontology.triples {
      if t.predicate.as_str() != RDFS_SUBCLASS_OF {
        continue;
      }
      let NamedOrBlankNode::NamedNode(class) = &t.subject else { continue };
      let Term::BlankNode(restriction) = &t.object else { continue };
      let restriction = NamedOrBlankNode::BlankNode(restriction.clone());
      if let (Some(prop), Some(&min)) = (restriction_property.get(&restriction), restriction_min_card.get(&restriction))
        && min >= 1
      {
        restrictions.push((class.as_str().to_string(), prop.clone()));
      }
    }

    Indices { triples: ontology.triples.clone(), literals, links, first, rest, restrictions }
  }

  /// `subject`'s value for `predicate` in language `lang` (`""` for an
  /// untagged literal — see [`crate::UNTAGGED`]).
  pub fn literal(&self, subject: &str, predicate: &str, lang: &str) -> Option<&str> {
    self.literal_all_langs(subject, predicate)?.get(lang).map(String::as_str)
  }

  /// Every language `subject` has a value for `predicate` in.
  pub fn literal_all_langs(&self, subject: &str, predicate: &str) -> Option<&LocalizedValues> {
    self.literals.get(&named(subject))?.get(predicate)
  }

  /// Every object IRI `subject` has for `predicate`, in triple-order.
  /// Empty (not `None`) when `subject` has no such link, so call sites
  /// don't need an `unwrap_or_default`.
  pub fn links(&self, subject: &str, predicate: &str) -> &[String] {
    self.links.get(&named(subject)).and_then(|m| m.get(predicate)).map(Vec::as_slice).unwrap_or(&[])
  }

  /// `subject`'s untagged `rdfs:label`, if any.
  pub fn label(&self, subject: &str) -> Option<&str> {
    self.literal(subject, crate::RDFS_LABEL, UNTAGGED)
  }

  /// `subject`'s untagged `rdfs:comment`, if any.
  pub fn comment(&self, subject: &str) -> Option<&str> {
    self.literal(subject, crate::RDFS_COMMENT, UNTAGGED)
  }

  /// `class_iri`'s single direct `rdfs:subClassOf` parent's IRI, if any
  /// (an ontology may assert more than one; this takes the first, matching
  /// the single-superclass model dataspace/site's `ClassInfo` uses).
  pub fn subclass_of(&self, class_iri: &str) -> Option<&str> {
    self.links(class_iri, RDFS_SUBCLASS_OF).first().map(String::as_str)
  }

  /// `property_iri`'s single `rdfs:domain` class IRI, if any.
  pub fn domain_of(&self, property_iri: &str) -> Option<&str> {
    self.links(property_iri, RDFS_DOMAIN).first().map(String::as_str)
  }

  /// Every property IRI whose `rdfs:domain` is one of `classes` — the
  /// reverse of [`Indices::domain_of`], used by [`applicable_properties`].
  /// Blank-node subjects (a property can't sensibly be a blank node) are
  /// skipped rather than panicking, since this walks every subject in the
  /// ontology, not just ones a caller specifically asked about.
  pub fn properties_with_domain_in(&self, classes: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for (subject, predicates) in &self.links {
      let Some(subject_iri) = iri_str(subject) else { continue };
      if let Some(domains) = predicates.get(RDFS_DOMAIN)
        && domains.iter().any(|d| classes.contains(d))
      {
        result.push(subject_iri);
      }
    }
    result
  }

  /// Walks the `rdf:List` that `subject`'s value for `predicate` points
  /// at (e.g. `owl:oneOf`), in authored order. Panics if `subject` has no
  /// value for `predicate`, or if the list structure is malformed (a list
  /// node with no `rdf:first`) — both indicate the ontology doesn't match
  /// what the caller expected, which should fail the build loudly rather
  /// than silently produce an empty list.
  pub fn rdf_list(&self, subject: &str, predicate: &str) -> Vec<Term> {
    let head = self.triples.iter().find_map(|t| {
      let NamedOrBlankNode::NamedNode(s) = &t.subject else { return None };
      (s.as_str() == subject && t.predicate.as_str() == predicate).then(|| t.object.clone())
    });
    let mut cursor: Option<NamedOrBlankNode> = match head {
      Some(Term::BlankNode(b)) => Some(NamedOrBlankNode::BlankNode(b)),
      Some(Term::NamedNode(n)) if n.as_str() != RDF_NIL => Some(NamedOrBlankNode::NamedNode(n)),
      Some(_) => None, // rdf:nil (or, if malformed, a literal head): an empty list.
      None => panic!("no {predicate:?} found on {subject:?} in the ontology"),
    };
    let mut items = Vec::new();
    while let Some(node) = cursor {
      let item = self.first.get(&node).unwrap_or_else(|| panic!("malformed rdf:List rooted at {subject:?} {predicate:?}: a list node has no rdf:first"));
      items.push(item.clone());
      cursor = self.rest.get(&node).cloned().filter(|next| !matches!(next, NamedOrBlankNode::NamedNode(n) if n.as_str() == RDF_NIL));
    }
    items
  }
}

/// Every class this ontology says `class_iri` (transitively) is — itself,
/// then each `rdfs:subClassOf` ancestor in order — since a property can be
/// declared with `rdfs:domain` on any class in this chain.
pub fn class_chain(indices: &Indices, class_iri: &str) -> Vec<String> {
  let mut chain = vec![class_iri.to_string()];
  let mut cursor = class_iri.to_string();
  while let Some(super_iri) = indices.subclass_of(&cursor) {
    let super_iri = super_iri.to_string();
    chain.push(super_iri.clone());
    cursor = super_iri;
  }
  chain
}

/// Every property IRI in `rdfs:domain` scope across `class_iri`'s
/// `rdfs:subClassOf` chain (its own properties plus every ancestor's),
/// sorted for deterministic codegen output independent of iteration order.
pub fn applicable_properties(indices: &Indices, class_iri: &str) -> Vec<String> {
  let chain = class_chain(indices, class_iri);
  let mut properties = indices.properties_with_domain_in(&chain);
  properties.sort();
  properties
}

/// Whether `property_iri` is required (`owl:minCardinality` >= 1) on
/// `class_iri` or any class in its `rdfs:subClassOf` chain.
pub fn is_required(indices: &Indices, class_iri: &str, property_iri: &str) -> bool {
  class_chain(indices, class_iri).iter().any(|c| indices.restrictions.iter().any(|(rc, rp)| rc == c && rp == property_iri))
}

/// Panics, listing every `(subject, predicate, missing lang)` triple it
/// finds, if any subject in `namespace` that uses one of `predicates` is
/// missing a value in one of `required_langs` for that predicate.
/// Completeness is only checked for `(subject, predicate)` pairs the
/// subject actually uses at least one language for — a subject that
/// doesn't use a given predicate at all isn't a translation gap, it's
/// just not applicable to that subject.
///
/// Collects every violation before panicking (rather than failing on the
/// first one found) so a single build failure reports the whole gap, not
/// just the first triple hit.
pub fn assert_bilingual_complete(indices: &Indices, namespace: &str, predicates: &[&str], required_langs: &[&str]) {
  let mut violations = Vec::new();
  for subject in crate::subjects_in_namespace(&indices.triples, namespace) {
    for &predicate in predicates {
      let Some(values) = indices.literal_all_langs(&subject, predicate) else { continue };
      for &lang in required_langs {
        let missing = values.get(lang).map(|v| v.trim().is_empty()).unwrap_or(true);
        if missing {
          violations.push(format!("{subject} {predicate} @{lang}"));
        }
      }
    }
  }
  if !violations.is_empty() {
    panic!(
      "missing bilingual content under namespace {namespace:?} (predicates {predicates:?}, required langs {required_langs:?}) -- {} violation(s):\n  {}",
      violations.len(),
      violations.join("\n  ")
    );
  }
}
