//! Error types for the libregexp-rs crate.

use thiserror::Error;

/// Errors that can occur when working with regular expressions.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum RegexError {
    /// Error compiling a regular expression pattern.
    #[error("regex compilation error: {0}")]
    CompileError(String),

    /// Memory allocation error.
    #[error("memory allocation error")]
    MemoryError,

    /// Regex execution timeout.
    #[error("regex execution timeout")]
    TimeoutError,

    /// Invalid UTF-8 sequence in input.
    #[error("invalid UTF-8 sequence")]
    InvalidUtf8,

    /// Invalid capture group index.
    #[error("invalid capture group index: {0}")]
    InvalidCaptureIndex(usize),

    /// Invalid bytecode (possibly corrupted).
    #[error("invalid regex bytecode")]
    InvalidBytecode,

    /// Other internal error.
    #[error("internal error: {0}")]
    InternalError(String),
}

impl RegexError {
    /// Creates a compile error from a C string.
    ///
    /// # Safety
    ///
    /// The pointer must point to a valid null-terminated C string.
    pub unsafe fn from_c_str(error_msg: *const std::os::raw::c_char) -> Self {
        if error_msg.is_null() {
            RegexError::CompileError("unknown error".to_string())
        } else {
            let c_str = unsafe { std::ffi::CStr::from_ptr(error_msg) };
            RegexError::CompileError(c_str.to_string_lossy().into_owned())
        }
    }

    /// Converts a C error code to a RegexError.
    pub fn from_c_error(code: std::os::raw::c_int) -> Self {
        match code {
            -1 => RegexError::MemoryError,
            -2 => RegexError::TimeoutError,
            _ => RegexError::InternalError(format!("unknown error code: {}", code)),
        }
    }
}

/// Result type for regex operations.
pub type Result<T> = std::result::Result<T, RegexError>;
