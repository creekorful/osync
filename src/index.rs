use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use sha1::Digest;
use walkdir::WalkDir;

const INDEX_FILE: &str = ".osync";
const IGNORE_FILE: &str = ".osyncignore";

pub struct Index {
    directory: PathBuf,
    files: HashMap<String, String>,
}

impl Index {
    fn blank<P: AsRef<Path>>(directory: P) -> Index {
        Index {
            directory: directory.as_ref().to_path_buf(),
            files: HashMap::new(),
        }
    }

    /// Try to load the cached index for given directory
    /// this will either return the loaded index or a new blank one.
    pub fn load<P: AsRef<Path>>(directory: P) -> Result<Index, Box<dyn Error>> {
        let index_path = directory.as_ref().join(INDEX_FILE);

        // if there's no .osync file in the directory, return
        // new blank index
        if !index_path.exists() {
            return Ok(Index::blank(directory));
        }

        // otherwise read index file line by line
        let mut files: HashMap<String, String> = HashMap::new();
        let buf = BufReader::new(File::open(index_path)?);
        for line in buf.lines() {
            let line = line.unwrap();
            let parts: Vec<&str> = line.split(':').collect();
            files.insert(parts[0].to_string(), parts[1].to_string());
        }

        Ok(Index {
            directory: directory.as_ref().to_path_buf(),
            files,
        })
    }

    /// Compute the index for given directory.
    pub fn compute<P: AsRef<Path>>(directory: P) -> Result<(Index, usize), Box<dyn Error>> {
        // try to load .osyncignore file
        let mut ignored_files: HashMap<String, bool> = HashMap::new();
        if let Ok(file) = File::open(directory.as_ref().join(IGNORE_FILE)) {
            let buf = BufReader::new(file);
            for line in buf.lines() {
                ignored_files.insert(line.unwrap(), true);
            }
        }

        let mut files: HashMap<String, String> = HashMap::new();
        for entry in WalkDir::new(&directory).into_iter().filter_map(|e| e.ok()) {
            let local_path = entry.path().strip_prefix(&directory)?;
            let metadata = entry.metadata().unwrap();

            if metadata.is_file() && !ignored_files.contains_key(local_path.to_str().unwrap()) {
                let bytes = fs::read(entry.path()).expect("unable to read file");

                let mut hasher = sha1::Sha1::new();
                hasher.update(bytes);

                files.insert(
                    local_path.to_str().unwrap().to_string(),
                    format!("{:x}", hasher.finalize()),
                );
            }
        }

        Ok((
            Index {
                directory: directory.as_ref().to_path_buf(),
                files,
            },
            ignored_files.len(),
        ))
    }

    /// Save the index to the disk.
    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let mut file = File::create(self.directory.join(INDEX_FILE))?;

        // create file content
        let mut content = String::new();
        for (path, hash) in self.files.iter() {
            content += format!("{}:{}\n", path, hash).as_str();
        }

        file.write_all(content.as_bytes()).map_err(|e| e.into())
    }

    /// Compute the difference between the indexes self & b
    /// return the changed files (new, modified) and the deleted.
    pub fn diff(&self, b: &Index) -> (Vec<String>, Vec<String>) {
        let mut changed_files: Vec<String> = Vec::new();
        let mut deleted_files: Vec<String> = Vec::new();

        for (path, hash) in &b.files {
            if self.files.get(path).is_none() || self.files.get(path).unwrap() != hash {
                changed_files.push(path.to_string());
            }
        }

        for path in self.files.keys() {
            if !b.files.contains_key(path) {
                deleted_files.push(path.to_string());
            }
        }

        (changed_files, deleted_files)
    }

    /// Returns the number of files in the index.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Returns `true` if the index contains no files.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Returns the path which the index is computed for.
    pub fn path(&self) -> PathBuf {
        self.directory.clone()
    }
}

/// Allows you to access the index file directory with `[]`
impl<'a> std::ops::Index<&'a str> for Index {
    type Output = String;

    fn index(&self, index: &'a str) -> &Self::Output {
        &self.files[index]
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempdir::TempDir;

    use crate::index::{Index, IGNORE_FILE, INDEX_FILE};

    #[test]
    fn test_blank() {
        let index = Index::blank("Tests");
        assert_eq!(index.path().to_str().unwrap(), "Tests");
        assert_eq!(index.len(), 0);
        assert_eq!(index.is_empty(), true);
    }

    #[test]
    fn test_load() {
        let dir = TempDir::new("osync").expect("unable to create temp dir");

        // create dummy index
        fs::write(
            dir.path().join(INDEX_FILE),
            "test:5d41402abc4b2a76b9719d911017c592",
        )
        .expect("unable to write index");

        let index = Index::load(dir).expect("unable to load index");
        assert_eq!(index.len(), 1);
        assert_eq!(index["test"], "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn test_compute_no_files() {
        let dir = TempDir::new("osync").expect("unable to create temp dir");

        let (index, _) = Index::compute(&dir).expect("unable to compute index");
        assert_eq!(index.len(), 0);
        assert_eq!(index.is_empty(), true);
    }

    #[test]
    fn test_compute_with_files() {
        let dir = TempDir::new("osync").expect("unable to create temp dir");

        fs::write(dir.path().join("test"), "hello").expect("unable to write test file");

        let (index, _) = Index::compute(&dir).expect("unable to compute index");
        assert_eq!(index.len(), 1);
        assert_eq!(index.is_empty(), false);
        assert_eq!(index["test"], "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d");

        // create a .osyncignore
        fs::write(dir.path().join(IGNORE_FILE), "test\n").expect("unable to write ignore file");

        // re compute index
        let (index, ignored) = Index::compute(&dir).expect("unable to compute index");
        assert_eq!(index.len(), 1); // the .osyncignore file
        assert_eq!(ignored, 1);
    }

    #[test]
    fn test_diff() {
        let dir = TempDir::new("osync").expect("unable to create temp dir");

        fs::write(dir.path().join("test"), "hello").expect("unable to write test file");

        // create dummy index
        fs::write(
            dir.path().join(INDEX_FILE),
            "test:aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d",
        )
        .expect("unable to write index");

        let previous_index = Index::load(&dir).expect("unable to read index");
        let (current_index, _) = Index::compute(&dir).expect("unable to compute index");

        let (changed_files, deleted_files) = previous_index.diff(&current_index);
        assert_eq!(changed_files.len(), 1); // TODO fix this (.osync is always returned)
        assert!(deleted_files.is_empty());
    }
}
