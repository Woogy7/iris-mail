/// Errors that can occur when working with core domain types.
///
/// These are validation errors — things that indicate the caller provided
/// invalid data, not that something went wrong with I/O or the network.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The provided string is not a valid email address.
    #[error("invalid email address: {0}")]
    InvalidEmail(String),

    /// The provided string is not a valid folder path.
    #[error("invalid folder path: {0}")]
    InvalidFolderPath(String),

    /// The provided string is not a recognised accent colour name.
    #[error("invalid accent colour: {0}")]
    InvalidColour(String),
}

/// Convenience alias for results using the core [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;
