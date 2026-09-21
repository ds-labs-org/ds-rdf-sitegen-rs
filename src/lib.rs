//! Build-time pipeline that turns a Turtle ontology/data source into:
//!
//! - generic, queryable `Indices` (labels/comments/any literal, all
//!   language-tag-aware; links such as `rdfs:domain`/`rdfs:subClassOf`;
//!   `owl:Restriction` cardinality; `owl:oneOf` list walking),
//! - a Concise Bounded Description (CBD) plus dual Turtle/JSON-LD
//!   serialization for any resource, written under a caller-chosen
//!   [`FileLayout`] so the output tree can mirror however the caller
//!   actually serves those files (a flat `generated-rdf/{Name}.ttl`, or a
//!   namespaced `generated-rdf/ns/{Name}.ttl` / `generated-rdf/data/{id}.ttl`
//!   split matching served URL paths), and
//! - a pluggable [`CodegenEmitter`] driver for generating a `include!`-able
//!   Rust source module from that same data.
//!
//! Extracted from `dataspace/site`'s original single-purpose `build.rs`
//! (which parsed one vendored ontology into a single-language `ClassInfo`
//! constant set) so a second site — one that needs bilingual (`@en`/`@fr`)
//! content and a vocabulary/data namespace split — can reuse the same
//! parse/index/CBD/serialize machinery instead of re-implementing it.
//!
//! This crate is meant to be used only from `[build-dependencies]`.

mod cbd;
mod codegen;
mod discovery;
mod indices;
mod layout;
mod ontology;
mod serialize;
mod util;

pub use cbd::{ResourceKey, describe_cbd};
pub use codegen::{CodegenEmitter, generate_rust_module, write_out_dir_file};
pub use discovery::{subjects_in_namespace, vocabulary_terms};
pub use indices::{Indices, LocalizedValues, applicable_properties, assert_bilingual_complete, class_chain, is_required};
pub use layout::{FileLayout, FlatLayout, NamespacedLayout, write_if_changed, write_rdf_files};
pub use ontology::Ontology;
pub use serialize::{Prefixes, to_jsonld, to_turtle};

// Re-exported so consumers don't need their own direct `oxrdf` dependency
// just to hold onto a `Triple`/`Term`/`NamedOrBlankNode`.
pub use oxrdf::{NamedNode, NamedOrBlankNode, Term, Triple};

pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
pub const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
pub const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
pub const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
pub const RDF_PROPERTY: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property";
pub const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
pub const RDFS_COMMENT: &str = "http://www.w3.org/2000/01/rdf-schema#comment";
pub const RDFS_SEE_ALSO: &str = "http://www.w3.org/2000/01/rdf-schema#seeAlso";
pub const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
pub const RDFS_SUBCLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
pub const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
pub const OWL_OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
pub const OWL_DATATYPE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";
pub const OWL_ONE_OF: &str = "http://www.w3.org/2002/07/owl#oneOf";
pub const OWL_ON_PROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
pub const OWL_MIN_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#minCardinality";

/// Language tag used to key an untagged (plain) literal in
/// [`Indices::literal`]/[`Indices::literal_all_langs`] — an ordinary Rust
/// empty string, not a magic sentinel, since `oxrdf::Literal::language()`
/// already returns `None` for a plain literal and `""` is simply what
/// `.unwrap_or("")` produces from that.
pub const UNTAGGED: &str = "";

#[cfg(test)]
mod tests;
