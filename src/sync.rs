use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

use ftp::FtpStream;
use url::Url;

use crate::index::Index;

pub trait Sync {
    fn synchronize(&self, a: &Index, b: &Index, dst: &Url) -> Result<(), Box<dyn Error>>;
}

/// A synchronizer which save by FTP.
pub struct FtpSync {}

impl Sync for FtpSync {
    fn synchronize(&self, a: &Index, b: &Index, dst: &Url) -> Result<(), Box<dyn Error>> {
        // compute diff
        let (changed_files, deleted_files) = a.diff(b);
        println!("{} files changed", changed_files.len());
        println!("{} files deleted", deleted_files.len());

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

        // change root directory if required
        if dst.path() != "/" {
            ftp_session.cwd(dst.path())?;
        }

        self.process_changed_files(&mut ftp_session, b.path(), &changed_files)?;
        self.process_deleted_files(&mut ftp_session, &deleted_files)?;

        Ok(())
    }
}

impl FtpSync {
    fn process_changed_files<P: AsRef<Path>>(
        &self,
        session: &mut FtpStream,
        path: P,
        files: &[String],
    ) -> Result<(), Box<dyn Error>> {
        for file in files {}

        unimplemented!()
    }
    fn process_deleted_files(
        &self,
        session: &mut FtpStream,
        files: &[String],
    ) -> Result<(), Box<dyn Error>> {
        for file in files {
            session.rm(file)?;
        }

        Ok(())
    }
}
