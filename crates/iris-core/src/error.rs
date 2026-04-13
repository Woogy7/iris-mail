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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_email_error_displays_address() {
        let err = Error::InvalidEmail("not-an-email".to_owned());
        let msg = err.to_string();
        assert!(
            msg.contains("not-an-email"),
            "expected address in message: {msg}"
        );
    }

    #[test]
    fn invalid_folder_path_error_displays_path() {
        let err = Error::InvalidFolderPath("".to_owned());
        let msg = err.to_string();
        assert!(
            msg.contains("invalid folder path"),
            "expected prefix: {msg}"
        );
    }

    #[test]
    fn invalid_colour_error_displays_value() {
        let err = Error::InvalidColour("neon".to_owned());
        let msg = err.to_string();
        assert!(msg.contains("neon"), "expected value in message: {msg}");
    }
}
