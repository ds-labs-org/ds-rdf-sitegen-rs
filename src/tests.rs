use crate::*;

const PREFIXES: &[(&str, &str)] = &[
  ("ex", "https://example.org/"),
  ("rdfs", "http://www.w3.org/2000/01/rdf-schema#"),
  ("owl", "http://www.w3.org/2002/07/owl#"),
];

fn ontology(ttl: &str) -> Ontology {
  let full = format!(
    "@prefix ex: <https://example.org/> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\n{ttl}\n"
  );
  Ontology::from_turtle_bytes(full.as_bytes(), "<test>")
}

#[test]
fn bilingual_literal_indexing() {
  let o = ontology(r#"ex:Thing rdfs:label "A"@en, "B"@fr ."#);
  let idx = Indices::build(&o);
  assert_eq!(idx.literal("https://example.org/Thing", RDFS_LABEL, "en"), Some("A"));
  assert_eq!(idx.literal("https://example.org/Thing", RDFS_LABEL, "fr"), Some("B"));
  assert_eq!(idx.literal("https://example.org/Thing", RDFS_LABEL, "de"), None);
  assert_eq!(idx.label("https://example.org/Thing"), None, "untagged label() should be None when only @en/@fr literals exist");
}

#[test]
fn untagged_literal_uses_empty_lang_key() {
  let o = ontology(r#"ex:Thing rdfs:label "Plain" ."#);
  let idx = Indices::build(&o);
  assert_eq!(idx.label("https://example.org/Thing"), Some("Plain"));
  assert_eq!(idx.literal("https://example.org/Thing", RDFS_LABEL, UNTAGGED), Some("Plain"));
}

#[test]
fn links_subclass_and_domain_of() {
  let o = ontology(
    r#"
    ex:Prop rdfs:domain ex:Class .
    ex:Sub rdfs:subClassOf ex:Super .
    "#,
  );
  let idx = Indices::build(&o);
  assert_eq!(idx.links("https://example.org/Prop", RDFS_DOMAIN), ["https://example.org/Class"]);
  assert_eq!(idx.domain_of("https://example.org/Prop"), Some("https://example.org/Class"));
  assert_eq!(idx.subclass_of("https://example.org/Sub"), Some("https://example.org/Super"));
  assert_eq!(idx.links("https://example.org/Nothing", RDFS_DOMAIN), Vec::<String>::new().as_slice());
}

#[test]
fn class_chain_and_applicable_properties_and_required() {
  let o = ontology(
    r#"
    ex:Base rdfs:subClassOf ex:Root .
    ex:BaseProp rdfs:domain ex:Base .
    ex:RootProp rdfs:domain ex:Root .
    ex:Base rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:BaseProp ; owl:minCardinality 1 ] .
    "#,
  );
  let idx = Indices::build(&o);
  assert_eq!(class_chain(&idx, "https://example.org/Base"), vec!["https://example.org/Base".to_string(), "https://example.org/Root".to_string()]);
  let props = applicable_properties(&idx, "https://example.org/Base");
  assert_eq!(props, vec!["https://example.org/BaseProp".to_string(), "https://example.org/RootProp".to_string()]);
  assert!(is_required(&idx, "https://example.org/Base", "https://example.org/BaseProp"));
  assert!(!is_required(&idx, "https://example.org/Base", "https://example.org/RootProp"));
}

#[test]
fn rdf_list_walks_one_of_in_order() {
  let o = ontology(r#"ex:Enum owl:oneOf ( ex:First ex:Second ex:Third ) ."#);
  let idx = Indices::build(&o);
  let items = idx.rdf_list("https://example.org/Enum", OWL_ONE_OF);
  let iris: Vec<String> = items
    .into_iter()
    .map(|t| match t {
      Term::NamedNode(n) => n.as_str().to_string(),
      other => panic!("expected NamedNode list items, got {other:?}"),
    })
    .collect();
  assert_eq!(iris, vec!["https://example.org/First".to_string(), "https://example.org/Second".to_string(), "https://example.org/Third".to_string()]);
}

#[test]
#[should_panic(expected = "no")]
fn rdf_list_panics_when_absent() {
  let o = ontology(r#"ex:Enum rdfs:label "no list here" ."#);
  let idx = Indices::build(&o);
  idx.rdf_list("https://example.org/Enum", OWL_ONE_OF);
}

#[test]
fn describe_cbd_excludes_blank_node_objects() {
  let o = ontology(
    r#"
    ex:Thing rdfs:label "kept literal" .
    ex:Thing rdfs:seeAlso ex:Other .
    ex:Thing rdfs:subClassOf [ a owl:Restriction ] .
    "#,
  );
  let key = ResourceKey { iri: "https://example.org/Thing", satellites: &[] };
  let cbd = describe_cbd(&o.triples, &key);
  assert_eq!(cbd.len(), 2, "expected the literal and the seeAlso triples, not the blank-node restriction triple: {cbd:?}");
  assert!(cbd.iter().all(|t| !matches!(&t.object, Term::BlankNode(_))));
}

#[test]
fn describe_cbd_includes_satellites() {
  let o = ontology(
    r#"
    ex:Prop rdfs:domain ex:Class .
    ex:Class rdfs:label "Class" .
    "#,
  );
  let key = ResourceKey { iri: "https://example.org/Class", satellites: &["https://example.org/Prop".to_string()] };
  let cbd = describe_cbd(&o.triples, &key);
  assert_eq!(cbd.len(), 2);
}

#[test]
fn flat_layout_strips_prefix() {
  let layout = FlatLayout { strip_prefix: "https://example.org/".to_string() };
  assert_eq!(layout.path_for("https://example.org/Thing"), std::path::PathBuf::from("Thing"));
}

#[test]
fn namespaced_layout_dispatches_by_prefix() {
  let layout = NamespacedLayout { namespaces: vec![("https://ds-labs.org/ns/".to_string(), "ns".to_string()), ("https://ds-labs.org/data/".to_string(), "data".to_string())] };
  assert_eq!(layout.path_for("https://ds-labs.org/ns/Organization"), std::path::PathBuf::from("ns/Organization"));
  assert_eq!(layout.path_for("https://ds-labs.org/data/ds-labs"), std::path::PathBuf::from("data/ds-labs"));
}

#[test]
#[should_panic(expected = "matched no configured namespace")]
fn namespaced_layout_panics_on_unmatched_prefix() {
  let layout = NamespacedLayout { namespaces: vec![("https://ds-labs.org/ns/".to_string(), "ns".to_string())] };
  layout.path_for("https://example.org/Unrelated");
}

#[test]
fn subjects_in_namespace_preserves_order_and_excludes_object_only_iris() {
  let o = ontology(
    r#"
    ex:First rdfs:label "1" ; ex:refersTo ex:NeverAsserted .
    ex:Second rdfs:label "2" .
    "#,
  );
  let subjects = subjects_in_namespace(&o.triples, "https://example.org/");
  assert_eq!(subjects, vec!["https://example.org/First".to_string(), "https://example.org/Second".to_string()], "ex:NeverAsserted is only ever an object, so it must be absent -- this is the deferred-URI rule");
}

#[test]
fn vocabulary_terms_filters_by_rdf_type() {
  let o = ontology(
    r#"
    ex:AClass a owl:Class .
    ex:AnIndividual a ex:AClass .
    "#,
  );
  let terms = vocabulary_terms(&o.triples, "https://example.org/");
  assert_eq!(terms, vec!["https://example.org/AClass".to_string()]);
}

#[test]
fn assert_bilingual_complete_passes_when_all_present() {
  let o = ontology(r#"ex:Doc rdfs:label "A"@en, "B"@fr ."#);
  let idx = Indices::build(&o);
  assert_bilingual_complete(&idx, "https://example.org/", &[RDFS_LABEL], &["en", "fr"]);
}

#[test]
#[should_panic(expected = "missing bilingual content")]
fn assert_bilingual_complete_panics_on_missing_translation() {
  let o = ontology(r#"ex:Doc rdfs:label "Only English"@en ."#);
  let idx = Indices::build(&o);
  assert_bilingual_complete(&idx, "https://example.org/", &[RDFS_LABEL], &["en", "fr"]);
}

#[test]
fn assert_bilingual_complete_skips_predicates_the_subject_does_not_use() {
  let o = ontology(r#"ex:Doc rdfs:label "A"@en, "B"@fr ."#);
  let idx = Indices::build(&o);
  // rdfs:comment is never used by ex:Doc at all -- not a translation gap.
  assert_bilingual_complete(&idx, "https://example.org/", &[RDFS_LABEL, RDFS_COMMENT], &["en", "fr"]);
}

struct DummyEmitter;
impl CodegenEmitter for DummyEmitter {
  fn emit(&self, indices: &Indices, resource_iri: &str) -> String {
    format!("pub const X: &str = {:?};\n", indices.label(resource_iri).unwrap_or_default())
  }
}

#[test]
fn generate_rust_module_concatenates_in_order() {
  let o = ontology(
    r#"
    ex:A rdfs:label "a" .
    ex:B rdfs:label "b" .
    "#,
  );
  let idx = Indices::build(&o);
  let module = generate_rust_module(&DummyEmitter, &idx, &["https://example.org/A", "https://example.org/B"]);
  assert!(module.starts_with("// Generated"));
  let a_pos = module.find("\"a\"").unwrap();
  let b_pos = module.find("\"b\"").unwrap();
  assert!(a_pos < b_pos, "resources must be emitted in the given order");
}

#[test]
fn turtle_and_jsonld_round_trip_a_triple() {
  let o = ontology(r#"ex:Thing rdfs:label "Hello" ."#);
  let prefixes = Prefixes(PREFIXES);
  let ttl = to_turtle(&o.triples, &prefixes);
  let jsonld = to_jsonld(&o.triples, &prefixes);
  assert!(String::from_utf8(ttl).unwrap().contains("Hello"));
  assert!(String::from_utf8(jsonld).unwrap().contains("Hello"));
}

#[test]
fn write_if_changed_skips_identical_content() {
  let dir = std::env::temp_dir().join(format!("rdf-sitegen-test-{}", std::process::id()));
  std::fs::create_dir_all(&dir).unwrap();
  let path = dir.join("out.txt");
  write_if_changed(&path, b"hello").unwrap();
  let mtime1 = std::fs::metadata(&path).unwrap().modified().unwrap();
  std::thread::sleep(std::time::Duration::from_millis(20));
  write_if_changed(&path, b"hello").unwrap();
  let mtime2 = std::fs::metadata(&path).unwrap().modified().unwrap();
  assert_eq!(mtime1, mtime2, "identical content must not rewrite the file");
  write_if_changed(&path, b"changed").unwrap();
  assert_eq!(std::fs::read(&path).unwrap(), b"changed");
  std::fs::remove_dir_all(&dir).ok();
}
