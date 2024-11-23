
use derive_more::From;
pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From)]
pub enum Error {
    // Proc related
    ProcNoFile,
    ProcPathNotUtf8,

    // Proc external
    #[from]
    Io(std::io::Error),
}