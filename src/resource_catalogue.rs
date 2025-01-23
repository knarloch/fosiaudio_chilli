use std::collections::HashMap;
use std::fs::canonicalize;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use log::{info, warn};
use rand::seq::IndexedRandom as _;

#[derive(Default)]
pub struct ResourceCatalogue(HashMap<String, Vec<PathBuf>>);

impl ResourceCatalogue {
    pub fn try_from_dir_path(path: impl AsRef<Path>) -> Result<Self> {
        let mut catalogue: HashMap<String, Vec<PathBuf>> = HashMap::new();
        info!(
            "Reading autogrzybke resources dir {}",
            path.as_ref().to_string_lossy()
        );
        let base = canonicalize(&path).context("Can't canonicalize resources dir")?;
        for path in list_files_recursive(&base).context("Error creating resource catalogue")? {
            let Ok(path) = canonicalize(&path)
                .inspect_err(|e| warn!("Can't canonicalize `{}`: {e}", path.to_string_lossy()))
            else {
                continue;
            };
            if !path.is_file() {
                continue;
            }

            if let Some(key) = key_from_path(&path, &base) {
                // info!("Adding {} -> {}", key, path.to_string_lossy());
                catalogue.entry(key).or_default().push(path);
            }
        }
        Ok(Self(catalogue))
    }

    pub fn random_sample(&self, basename: &str) -> Option<String> {
        let mut rng = rand::rng();
        self.0
            .get(&basename.to_lowercase())
            .and_then(|matching_files| matching_files.choose(&mut rng))
            .map(|p| p.to_string_lossy().into())
    }
}

fn key_from_path(path: impl AsRef<Path>, base: impl AsRef<Path>) -> Option<String> {
    let prefix = path.as_ref().strip_prefix(base).ok()?;
    let extension = prefix.extension().unwrap_or_default().to_string_lossy();
    let prefix = prefix.to_string_lossy();
    let result = prefix[..prefix.len() - extension.len()]
        .trim_end_matches('.')
        .trim_end_matches(char::is_numeric)
        .to_lowercase();
    Some(String::from(result))
}

pub fn list_files_recursive(dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let mut output = Vec::new();
    list_files_recursive_impl(dir.as_ref(), &mut output)?;
    Ok(output)
}

fn list_files_recursive_impl(dir: &Path, list: &mut Vec<PathBuf>) -> Result<()> {
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir).context(format!("Failed to read_dir {dir:?}"))? {
            let entry = entry.context(format!("Failed to get entry from {dir:?}"))?;
            let path = entry.path();
            if path.is_dir() {
                list_files_recursive_impl(&path, list)?;
            } else {
                list.push(path);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn key_from_path_test() {
        assert_eq!(key_from_path("/dir1/file1.mp3", "/dir2"), None);
        assert_eq!(key_from_path("/dir/hypys1.mp3", "/dir").unwrap(), "hypys");
        assert_eq!(key_from_path("/dir/no_ext1", "/dir").unwrap(), "no_ext");
        assert_eq!(key_from_path("/dir/no_num.mp3", "/dir").unwrap(), "no_num");
        assert_eq!(key_from_path("/alpinus41.mp3", "/").unwrap(), "alpinus");
        assert_eq!(key_from_path("/dir/sub/f1.mp3", "/dir").unwrap(), "sub/f");
        assert_eq!(
            key_from_path("/dir/CAPITAL12.mp3", "/dir").unwrap(),
            "capital"
        );
    }
}
