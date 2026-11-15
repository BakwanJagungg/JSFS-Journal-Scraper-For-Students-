use thiserror::Error;

/// Unified application error type formatted cleanly for the native desktop UI.
#[derive(Debug, Error)]
pub enum JsfsError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Excel export error: {0}")]
    Excel(#[from] rust_xlsxwriter::XlsxError),
    #[error("Document error: {0}")]
    Document(String),
    #[error("Invalid parameter: {0}")]
    InvalidInput(String),
    #[error("Operation cancelled")]
    Cancelled,
}

impl serde::Serialize for JsfsError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
