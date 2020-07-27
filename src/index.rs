use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

const INDEX_FILE: &str = ".osync";

pub struct Index {
    directory: PathBuf,
    files: HashMap<String, String>,
}

impl Index {
    /// Create a blank index for the given directory
    fn blank<P: AsRef<Path>>(directory: P) -> Index {
        Index {
            directory: directory.as_ref().to_path_buf(),
            files: HashMap::new(),
        }
    }

    /// Try to load the cached index for given directory
    /// this will either return the loaded index or a new blank one
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

    /// Compute the index for given directory
    pub fn compute<P: AsRef<Path>>(directory: P) -> Result<Index, Box<dyn Error>> {
        let mut files: HashMap<String, String> = HashMap::new();
        for entry in WalkDir::new(&directory).into_iter().filter_map(|e| e.ok()) {
            let local_path = entry.path().strip_prefix(&directory)?;
            let metadata = entry.metadata().unwrap();

            if metadata.is_file() {
                let bytes = fs::read(entry.path()).expect("unable to read file");
                let digest = md5::compute(bytes);

                files.insert(
                    local_path.to_str().unwrap().to_string(),
                    format!("{:x}", digest),
                );
            }
        }

        Ok(Index {
            directory: directory.as_ref().to_path_buf(),
            files,
        })
    }

    /// Save the index to the disk
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
    /// return the changed files (new, modified) and the deleted
    pub fn diff(&self, b: &Index) -> (HashMap<String, String>, HashMap<String, String>) {
        let mut changed_files: HashMap<String, String> = HashMap::new();
        let mut deleted_files: HashMap<String, String> = HashMap::new();

        for (path, hash) in &b.files {
            if self.files.get(path).is_none() || self.files.get(path).unwrap() != hash {
                changed_files.insert(path.to_string(), hash.to_string());
            }
        }

        for (path, hash) in &self.files {
            if !b.files.contains_key(path) {
                deleted_files.insert(path.to_string(), hash.to_string());
            }
        }

        (changed_files, deleted_files)
    }
}
