use std::fs;
use std::path::{Path, PathBuf};

use oxrdf::Triple;

use crate::serialize::{Prefixes, to_jsonld, to_turtle};

/// Where a resource's serialized RDF files land, relative to an output
/// root — decouples CBD/serialization from file placement, so the same
/// pipeline can produce dataspace/site's flat `generated-rdf/{Name}.ttl`
/// layout or www's namespaced `generated-rdf/ns/{Name}.ttl` /
/// `generated-rdf/data/{id}.ttl` split (which mirrors how those files are
/// actually served).
pub trait FileLayout {
  /// Path (relative to an output root), WITHOUT extension, for
  /// `resource_iri`'s serialized files. Callers append `.ttl`/`.jsonld`.
  fn path_for(&self, resource_iri: &str) -> PathBuf;
}

/// Flat layout: strips `strip_prefix` off `resource_iri` and uses what's
/// left as the file's local name — dataspace/site's current
/// `generated-rdf/{ClassLocal}.ttl` shape.
pub struct FlatLayout {
  pub strip_prefix: String,
}

impl FileLayout for FlatLayout {
  fn path_for(&self, resource_iri: &str) -> PathBuf {
    PathBuf::from(resource_iri.strip_prefix(self.strip_prefix.as_str()).unwrap_or(resource_iri))
  }
}

/// Namespaced layout: `namespaces` is a list of `(IRI prefix, output
/// subdirectory)` pairs, checked in order — the first prefix `resource_iri`
/// starts with wins, and the file lands at `{subdir}/{local part}`. Panics
/// if `resource_iri` matches none of the configured namespaces, since that
/// means a resource is being emitted the layout wasn't told about.
pub struct NamespacedLayout {
  pub namespaces: Vec<(String, String)>,
}

impl FileLayout for NamespacedLayout {
  fn path_for(&self, resource_iri: &str) -> PathBuf {
    for (prefix, subdir) in &self.namespaces {
      if let Some(local) = resource_iri.strip_prefix(prefix.as_str()) {
        return Path::new(subdir).join(local);
      }
    }
    panic!("resource IRI {resource_iri:?} matched no configured namespace in NamespacedLayout (configured prefixes: {:?})", self.namespaces.iter().map(|(p, _)| p).collect::<Vec<_>>());
  }
}

/// Writes `content` to `path` only if it would actually change (a byte
/// comparison against the file's current content). Required whenever the
/// output directory is also `cargo:rerun-if-changed`-watched: an
/// unconditional write would perpetually bump the directory's mtime and
/// make every `cargo check`/`build` rerun the build script forever, even
/// with nothing to regenerate.
pub fn write_if_changed(path: &Path, content: &[u8]) -> std::io::Result<()> {
  if fs::read(path).ok().as_deref() == Some(content) {
    return Ok(());
  }
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)?;
  }
  fs::write(path, content)
}

/// Serializes `cbd` as both Turtle and JSON-LD and writes both under
/// `out_dir.join(layout.path_for(resource_iri))`, via [`write_if_changed`].
pub fn write_rdf_files(out_dir: &Path, layout: &dyn FileLayout, resource_iri: &str, cbd: &[Triple], prefixes: &Prefixes) -> std::io::Result<()> {
  let base = out_dir.join(layout.path_for(resource_iri));
  write_if_changed(&with_appended_extension(&base, "ttl"), &to_turtle(cbd, prefixes))?;
  write_if_changed(&with_appended_extension(&base, "jsonld"), &to_jsonld(cbd, prefixes))?;
  Ok(())
}

/// `base` + `.ext`. Not `Path::with_extension`, which replaces whatever
/// follows the last dot: `path_for` returns a path *without* extension, so a
/// dotted local name such as `v0.0.1` would become `v0.0.ttl`.
fn with_appended_extension(base: &Path, ext: &str) -> PathBuf {
  let mut name = base.as_os_str().to_owned();
  name.push(".");
  name.push(ext);
  PathBuf::from(name)
}
