use std::env;
use crate::errors::{Result,Error};

// https://stackoverflow.com/questions/36848132/how-to-get-the-name-of-current-program-without-the-directory-part/36859137#36859137
pub fn prog_name() -> Result<String> {
    Ok(env::current_exe()?
        .file_name().ok_or(Error::ProcNoFile)?
        .to_str().ok_or(Error::ProcPathNotUtf8)?
        .to_owned())
}