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
const SWAP_FILE: &str = ".osync.swp";

#[derive(Clone)]
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

    fn from_file<P: AsRef<Path>>(file: P) -> Result<Index, Box<dyn Error>> {
        let mut files: HashMap<String, String> = HashMap::new();
        let buf = BufReader::new(File::open(file)?);
        for line in buf.lines() {
            let line = line.unwrap();
            let parts: Vec<&str> = line.split(':').collect();
            files.insert(parts[0].to_string(), parts[1].to_string());
        }

        Ok(Index {
            directory: Default::default(),
            files,
        })
    }

    /// Try to load the cached index for given directory
    /// this will either return the loaded index or a new blank one.
    pub fn load<P: AsRef<Path>>(directory: P) -> Result<(Index, usize), Box<dyn Error>> {
        let index_path = directory.as_ref().join(INDEX_FILE);

        // if there's no .osync file in the directory, return
        // new blank index
        if !index_path.exists() {
            return Ok((Index::blank(directory), 0));
        }

        // otherwise read index file line by line
        let mut resumed_files = 0;
        let mut index = Index::from_file(index_path)?;

        // read swap file if any
        let swap_path = directory.as_ref().join(SWAP_FILE);
        if swap_path.exists() {
            let swap_index = Index::from_file(swap_path)?;
            index = index.merge(&swap_index);
            resumed_files = swap_index.len();
        }

        Ok((index, resumed_files))
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

        // do not upload .osync{...} files
        ignored_files.insert(INDEX_FILE.to_string(), true);
        ignored_files.insert(IGNORE_FILE.to_string(), true);
        ignored_files.insert(SWAP_FILE.to_string(), true);

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

    /// Merge b into self and return the result
    pub fn merge(&self, b: &Index) -> Index {
        let mut index = self.clone();

        for (file, hash) in b.files() {
            index.files.insert(file, hash);
        }

        index
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

    pub fn files(&self) -> HashMap<String, String> {
        self.files.clone()
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

    use crate::index::{Index, IGNORE_FILE, INDEX_FILE, SWAP_FILE};

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

        let (index, _) = Index::load(dir).expect("unable to load index");
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
        assert_eq!(index.len(), 0);
        assert_eq!(ignored, 4);
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

        let (previous_index, _) = Index::load(&dir).expect("unable to read index");
        let (current_index, _) = Index::compute(&dir).expect("unable to compute index");

        let (changed_files, deleted_files) = previous_index.diff(&current_index);
        assert_eq!(changed_files.len(), 0);
        assert!(deleted_files.is_empty());
    }

    #[test]
    fn test_merge() {
        let mut a = Index::blank("");
        a.files
            .insert("Test/a.png".to_string(), "Test/a.png.a".to_string());
        a.files
            .insert("Test/b.png".to_string(), "Test/b.png.a".to_string());

        let mut b = Index::blank("");
        b.files
            .insert("Test/b.png".to_string(), "Test/b.png.b".to_string());

        let result = a.merge(&b);
        assert_eq!(result.len(), 2);

        assert_eq!(result["Test/a.png"], "Test/a.png.a");
        assert_eq!(result["Test/b.png"], "Test/b.png.b");
    }

    #[test]
    fn test_swap() {
        let dir = TempDir::new("osync").expect("unable to create temp dir");

        fs::write(dir.path().join("test-1"), "hello-1").expect("unable to write test-1 file");
        fs::write(dir.path().join("test-2"), "hello-2").expect("unable to write test-2 file");

        // create dummy index
        // test-1 is valid, test-2 is not
        fs::write(
            dir.path().join(INDEX_FILE),
            "test-1:2035422f61642bc8e6e93c4394f7501af01e7735\ntest-2:invalid-sum",
        )
        .expect("unable to write index");

        // create swap file with correct entry for test-2
        fs::write(
            dir.path().join(SWAP_FILE),
            "test-2:6ca43813c03a60dd9f7542022be9055c7cd712a5",
        )
        .expect("unable to write index");

        let (previous_index, resumed_files) = Index::load(&dir).expect("unable to read index");
        let (current_index, _) = Index::compute(&dir).expect("unable to compute index");

        let (changed_files, deleted_files) = previous_index.diff(&current_index);
        assert_eq!(resumed_files, 1);
        assert_eq!(changed_files.len(), 0);
        assert!(deleted_files.is_empty());
    }
}
