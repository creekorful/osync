use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::path::{Path, PathBuf};

use ftp::types::FileType;
use ftp::FtpStream;
use indicatif::{ProgressBar, ProgressStyle};
use url::Url;

use crate::index::Index;

pub trait Sync {
    fn synchronize(
        &mut self,
        a: &Index,
        b: &Index,
        assume_directories: bool,
        skip_upload: bool,
    ) -> Result<(), Box<dyn Error>>;
}

/// A synchronizer which save by FTP.
pub struct FtpSync {
    ftp_session: FtpStream,
    remote_dir: String,
    // create a local cache of existing directories
    // so that we won't waste time trying to create them again
    existing_directories: HashMap<String, bool>,
}

impl Sync for FtpSync {
    fn synchronize(
        &mut self,
        current_index: &Index,
        previous_index: &Index,
        assume_directories: bool,
        skip_upload: bool,
    ) -> Result<(), Box<dyn Error>> {
        // compute diff
        let (changed_files, deleted_files) = previous_index.diff(current_index);
        println!("-> {} files changed", changed_files.len());
        println!("-> {} files deleted", deleted_files.len());

        // If set to true, use the local cache to determinate existing directories
        // this will greatly reduce upload duration since we do not need to try to create ALL directories.
        if assume_directories {
            for (path, _) in previous_index.files() {
                let path = Path::new(&path);

                // remove last component from path (the file)
                if path.parent().is_none() {
                    continue;
                }
                let path = path.parent().unwrap().to_str().unwrap();

                let mut current_dir = self.remote_dir.clone();
                for folder in path.split('/').filter(|f| !f.is_empty()) {
                    current_dir = format!("{}/{}", current_dir, folder);
                    self.existing_directories
                        .insert(current_dir.to_string(), true);
                }
            }
        }

        if !skip_upload {
            // create progress bar
            let pb = ProgressBar::new((changed_files.len() + deleted_files.len()) as u64);
            pb.set_style(ProgressStyle::default_bar().template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] ({pos}/{len}, ETA {eta})",
            ));

            self.process_changed_files(&pb, current_index.path(), &changed_files)?;
            self.process_deleted_files(&pb, &deleted_files)?;
        }

        // everything is fine, save index to file
        current_index.save()?;

        Ok(())
    }
}

impl FtpSync {
    pub fn new(dst: &Url) -> Result<FtpSync, Box<dyn Error>> {
        // open FTP connection
        let address = format!(
            "{}:{}",
            dst.host_str().expect("missing address"),
            dst.port().unwrap_or(21)
        );
        let mut ftp_session = FtpStream::connect(address)?;

        // authenticate if required
        if dst.username() != "" {
            ftp_session.login(dst.username(), dst.password().unwrap_or(""))?;
        }

        // set transfer mode to binary
        ftp_session.transfer_type(FileType::Binary)?;

        // setup custom root directory if required
        let remote_dir = if dst.path() != "" { dst.path() } else { "/" };

        Ok(FtpSync {
            ftp_session,
            remote_dir: remote_dir.to_string(),
            existing_directories: HashMap::new(),
        })
    }

    fn process_changed_files<P: AsRef<Path>>(
        &mut self,
        progress_bar: &ProgressBar,
        local_dir: P,
        files: &[String],
    ) -> Result<(), Box<dyn Error>> {
        for path in files {
            // extract parent directory
            let p = PathBuf::from(path.clone());
            let parent = p.parent().unwrap().to_str().unwrap();

            // create any missing directories (recursively)
            self.make_directories(&format!("{}/{}", &self.remote_dir, parent))?;

            // store the file on the server
            let mut content = File::open(local_dir.as_ref().join(path))?;
            self.ftp_session
                .put(&format!("{}/{}", &self.remote_dir, path), &mut content)?;

            progress_bar.println(format!("[+] {}", path));
            progress_bar.inc(1);
        }

        Ok(())
    }
    fn process_deleted_files(
        &mut self,
        progress_bar: &ProgressBar,
        files: &[String],
    ) -> Result<(), Box<dyn Error>> {
        for file in files {
            self.ftp_session
                .rm(&format!("{}/{}", &self.remote_dir, file))?;

            progress_bar.println(format!("[-] {}", file));
            progress_bar.inc(1);
        }

        // TODO: it could be great to delete empty directory too

        Ok(())
    }

    fn make_directories(&mut self, path: &str) -> Result<(), Box<dyn Error>> {
        let mut current_dir = String::new();

        for folder in path.split('/').filter(|f| !f.is_empty()) {
            let next_dir = format!("{}/{}", current_dir, folder);

            // if the directory is not yet in the cache
            if !self.existing_directories.contains_key(&next_dir) {
                // create directory if not already exist
                if !self.directory_exist(&current_dir, &folder)? {
                    self.ftp_session.mkdir(&next_dir)?;
                }

                // insert directory into cache
                self.existing_directories.insert(next_dir.to_string(), true);
            }

            current_dir = next_dir;
        }

        Ok(())
    }

    fn directory_exist(&mut self, haystack: &str, needle: &str) -> Result<bool, Box<dyn Error>> {
        for f in self.ftp_session.list(Some(haystack))? {
            let parts: Vec<&str> = f.split_whitespace().collect();
            let perm = parts[0];
            let name = parts[parts.len() - 1];
            let is_dir = perm.starts_with('d');

            if is_dir && name == needle {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
