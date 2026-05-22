use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use serde::Deserialize;

const MANIFEST_FILE_NAME: &str = ".starpls.json";

#[derive(Debug, Deserialize)]
struct Manifest {
    version: u32,
    #[serde(default)]
    schemas: Vec<Schema>,
}

#[derive(Debug, Deserialize)]
struct Schema {
    name: String,
    #[serde(default)]
    include: Vec<String>,
    builtins: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CustomSchemaMatch {
    pub(crate) schema_id: String,
    pub(crate) schema_name: String,
    pub(crate) manifest_path: PathBuf,
    pub(crate) builtins_path: PathBuf,
}

pub(crate) fn discover_custom_schema(
    source_path: impl AsRef<Path>,
) -> anyhow::Result<Option<CustomSchemaMatch>> {
    let source_path = source_path.as_ref();
    let manifest_path = match find_nearest_manifest(source_path)? {
        Some(manifest_path) => manifest_path,
        None => return Ok(None),
    };
    let manifest = load_manifest(&manifest_path)?;
    let manifest_dir = manifest_path
        .parent()
        .context("custom builtins manifest path has no parent")?
        .to_path_buf();
    let relative_source_path = match source_path.strip_prefix(&manifest_dir) {
        Ok(relative_source_path) => relative_source_path,
        Err(_) => return Ok(None),
    };

    let schema = match manifest.matching_schema(relative_source_path) {
        Some(schema) => schema,
        None => return Ok(None),
    };

    let canonical_manifest_path = fs::canonicalize(&manifest_path).with_context(|| {
        format!(
            "failed to canonicalize custom builtins manifest {}",
            manifest_path.display()
        )
    })?;
    let schema_id = format!("{}#{}", canonical_manifest_path.display(), schema.name);

    Ok(Some(CustomSchemaMatch {
        schema_id,
        schema_name: schema.name.clone(),
        manifest_path,
        builtins_path: manifest_dir.join(&schema.builtins),
    }))
}

fn find_nearest_manifest(source_path: &Path) -> anyhow::Result<Option<PathBuf>> {
    let start_dir = if source_path.is_dir() {
        source_path
    } else {
        match source_path.parent() {
            Some(parent) => parent,
            None => return Ok(None),
        }
    };

    for ancestor in start_dir.ancestors() {
        let manifest_path = ancestor.join(MANIFEST_FILE_NAME);
        if manifest_path
            .try_exists()
            .with_context(|| format!("failed to inspect {}", manifest_path.display()))?
        {
            return Ok(Some(manifest_path));
        }
    }

    Ok(None)
}

fn load_manifest(path: &Path) -> anyhow::Result<Manifest> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("failed to read custom builtins manifest {}", path.display()))?;
    let manifest = serde_json::from_str::<Manifest>(&data).with_context(|| {
        format!(
            "failed to parse custom builtins manifest {}",
            path.display()
        )
    })?;
    anyhow::ensure!(
        manifest.version == 1,
        "unsupported custom builtins manifest version {} in {}",
        manifest.version,
        path.display()
    );
    Ok(manifest)
}

impl Manifest {
    fn matching_schema(&self, relative_source_path: &Path) -> Option<&Schema> {
        let relative_source_path = path_to_slash_string(relative_source_path)?;
        self.schemas.iter().find(|schema| {
            schema
                .include
                .iter()
                .any(|pattern| glob_matches(pattern, &relative_source_path))
        })
    }
}

fn path_to_slash_string(path: &Path) -> Option<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        parts.push(component.as_os_str().to_str()?);
    }
    Some(parts.join("/"))
}

fn glob_matches(pattern: &str, text: &str) -> bool {
    let pattern_segments = pattern.split('/').collect::<Vec<_>>();
    let text_segments = text.split('/').collect::<Vec<_>>();
    glob_segments_match(&pattern_segments, &text_segments)
}

fn glob_segments_match(pattern: &[&str], text: &[&str]) -> bool {
    match (pattern.split_first(), text.split_first()) {
        (None, None) => true,
        (None, Some(_)) => false,
        (Some((&"**", rest)), _) => {
            glob_segments_match(rest, text)
                || text
                    .split_first()
                    .map(|(_, text_rest)| glob_segments_match(pattern, text_rest))
                    .unwrap_or(false)
        }
        (Some((pattern_head, pattern_rest)), Some((text_head, text_rest))) => {
            glob_component_matches(pattern_head, text_head)
                && glob_segments_match(pattern_rest, text_rest)
        }
        (Some(_), None) => false,
    }
}

fn glob_component_matches(pattern: &str, text: &str) -> bool {
    let pattern = pattern.as_bytes();
    let text = text.as_bytes();
    let (mut pattern_idx, mut text_idx) = (0, 0);
    let mut star_idx = None;
    let mut star_text_idx = 0;

    while text_idx < text.len() {
        if pattern_idx < pattern.len()
            && (pattern[pattern_idx] == b'?' || pattern[pattern_idx] == text[text_idx])
        {
            pattern_idx += 1;
            text_idx += 1;
        } else if pattern_idx < pattern.len() && pattern[pattern_idx] == b'*' {
            star_idx = Some(pattern_idx);
            pattern_idx += 1;
            star_text_idx = text_idx;
        } else if let Some(star_idx) = star_idx {
            pattern_idx = star_idx + 1;
            star_text_idx += 1;
            text_idx = star_text_idx;
        } else {
            return false;
        }
    }

    pattern[pattern_idx..].iter().all(|byte| *byte == b'*')
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;

    use super::discover_custom_schema;
    use super::find_nearest_manifest;
    use super::glob_matches;

    #[test]
    fn finds_nearest_manifest_by_walking_upward() {
        let root = test_dir("nearest");
        let nested = root.join("a/b/c");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.join(".starpls.json"), r#"{"version":1}"#).unwrap();

        let manifest = find_nearest_manifest(&nested.join("main.star"))
            .unwrap()
            .unwrap();

        assert_eq!(manifest, root.join(".starpls.json"));
        cleanup(root);
    }

    #[test]
    fn first_matching_schema_wins() {
        let root = test_dir("first-match");
        let nested = root.join("pkg");
        fs::create_dir_all(&nested).unwrap();
        fs::write(
            root.join(".starpls.json"),
            r#"
            {
              "version": 1,
              "schemas": [
                {
                  "name": "first",
                  "include": ["pkg/*.star"],
                  "builtins": "first.starpls.json"
                },
                {
                  "name": "second",
                  "include": ["**/*.star"],
                  "builtins": "second.starpls.json"
                }
              ]
            }
            "#,
        )
        .unwrap();

        let matched = discover_custom_schema(nested.join("main.star"))
            .unwrap()
            .unwrap();

        assert_eq!(matched.schema_name, "first");
        assert_eq!(matched.builtins_path, root.join("first.starpls.json"));
        assert!(matched.schema_id.ends_with(".starpls.json#first"));
        cleanup(root);
    }

    #[test]
    fn ignores_files_outside_includes() {
        let root = test_dir("outside-include");
        fs::create_dir_all(root.join("pkg")).unwrap();
        fs::write(
            root.join(".starpls.json"),
            r#"
            {
              "version": 1,
              "schemas": [
                {
                  "name": "example",
                  "include": ["pkg/*.star"],
                  "builtins": "example.starpls.json"
                }
              ]
            }
            "#,
        )
        .unwrap();

        let matched = discover_custom_schema(root.join("other/main.star")).unwrap();

        assert_eq!(matched, None);
        cleanup(root);
    }

    #[test]
    fn matches_supported_glob_patterns() {
        assert!(glob_matches("**/*.star", "main.star"));
        assert!(glob_matches("**/*.star", "pkg/nested/main.star"));
        assert!(glob_matches("pkg/*.star", "pkg/main.star"));
        assert!(!glob_matches("pkg/*.star", "pkg/nested/main.star"));
        assert!(glob_matches("pkg/??.star", "pkg/ab.star"));
        assert!(!glob_matches("pkg/??.star", "pkg/abc.star"));
    }

    fn test_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "starpls-custom-builtin-manifest-{}-{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn cleanup(path: impl AsRef<Path>) {
        fs::remove_dir_all(path).unwrap();
    }
}
