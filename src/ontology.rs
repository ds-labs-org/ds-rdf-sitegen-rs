use std::fs;
use std::path::Path;

use oxrdf::Triple;
use oxttl::TurtleParser;

/// A parsed Turtle source: nothing but the flat triple list. Everything
/// queryable (labels, links, restrictions, ...) is derived from this by
/// [`crate::Indices::build`].
pub struct Ontology {
  pub triples: Vec<Triple>,
}

impl Ontology {
  /// Parses `path` as Turtle. Panics with a build.rs-friendly message
  /// (naming the path) on a read or parse failure — a build script is
  /// expected to fail loudly, not propagate a `Result` past `main`.
  pub fn load_turtle(path: &Path) -> Ontology {
    let data = fs::read(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    Ontology::from_turtle_bytes(&data, &path.display().to_string())
  }

  /// Parses `data` as Turtle. `source_label` is only used in the panic
  /// message on a parse failure — useful for synthetic/test sources that
  /// have no real path.
  pub fn from_turtle_bytes(data: &[u8], source_label: &str) -> Ontology {
    let triples: Vec<_> = TurtleParser::new()
      .for_reader(data)
      .collect::<Result<_, _>>()
      .unwrap_or_else(|e| panic!("failed to parse {source_label} as Turtle: {e}"));
    Ontology { triples }
  }
}
