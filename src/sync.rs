use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::path::{Path, PathBuf};

use ftp::FtpStream;
use indicatif::{ProgressBar, ProgressStyle};
use url::Url;

use crate::index::Index;

pub trait Sync {
    fn synchronize(&self, a: &Index, b: &Index, dst: &Url) -> Result<(), Box<dyn Error>>;
}

/// A synchronizer which save by FTP.
pub struct FtpSync {}

impl Sync for FtpSync {
    fn synchronize(
        &self,
        current_index: &Index,
        previous_index: &Index,
        dst: &Url,
    ) -> Result<(), Box<dyn Error>> {
        // compute diff
        let (changed_files, deleted_files) = previous_index.diff(current_index);
        println!("-> {} files changed", changed_files.len());
        println!("-> {} files deleted", deleted_files.len());

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

        // setup custom root directory if required
        let remote_dir = if dst.path() != "" { dst.path() } else { "/" };

        // create progress bar
        let pb = ProgressBar::new((changed_files.len() + deleted_files.len()) as u64);
        pb.set_style(ProgressStyle::default_bar().template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] ({pos}/{len}, ETA {eta})",
        ));

        self.process_changed_files(
            &mut ftp_session,
            &pb,
            current_index.path(),
            &remote_dir,
            &changed_files,
        )?;
        self.process_deleted_files(&mut ftp_session, &pb, &remote_dir, &deleted_files)?;

        // everything is fine, save index to file
        current_index.save()?;

        Ok(())
    }
}

impl FtpSync {
    fn process_changed_files<P: AsRef<Path>>(
        &self,
        session: &mut FtpStream,
        progress_bar: &ProgressBar,
        local_dir: P,
        remote_dir: &str,
        files: &[String],
    ) -> Result<(), Box<dyn Error>> {
        // create a local cache of existing directories
        // so that we won't waste time trying to create them again
        let mut existing_directories: HashMap<String, bool> = HashMap::new();

        for path in files {
            // extract parent directory
            let p = PathBuf::from(path.clone());
            let parent = p.parent().unwrap().to_str().unwrap();

            // create any missing directories (recursively)
            if !existing_directories.contains_key(parent) {
                self.make_directories(session, &format!("{}/{}", remote_dir, parent))?;
                existing_directories.insert(parent.to_string(), true);
            }

            // store the file on the server
            let mut content = File::open(local_dir.as_ref().join(path))?;
            session.put(&format!("{}/{}", remote_dir, path), &mut content)?;

            progress_bar.println(format!("[+] {}", path));
            progress_bar.inc(1);
        }

        Ok(())
    }
    fn process_deleted_files(
        &self,
        session: &mut FtpStream,
        progress_bar: &ProgressBar,
        remote_dir: &str,
        files: &[String],
    ) -> Result<(), Box<dyn Error>> {
        for file in files {
            session.rm(&format!("{}/{}", remote_dir, file))?;

            progress_bar.println(format!("[-] {}", file));
            progress_bar.inc(1);
        }

        // TODO: it could be great to delete empty directory too

        Ok(())
    }

    fn make_directories(&self, session: &mut FtpStream, path: &str) -> Result<(), Box<dyn Error>> {
        let mut current_dir = String::new();

        for folder in path.split('/').filter(|f| !f.is_empty()) {
            let next_dir = format!("{}/{}", current_dir, folder);

            if !self.directory_exist(session, &current_dir, &folder)? {
                session.mkdir(&next_dir)?;
            }

            current_dir = next_dir;
        }

        Ok(())
    }

    fn directory_exist(
        &self,
        session: &mut FtpStream,
        haystack: &str,
        needle: &str,
    ) -> Result<bool, Box<dyn Error>> {
        for f in session.list(Some(haystack)).unwrap() {
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
