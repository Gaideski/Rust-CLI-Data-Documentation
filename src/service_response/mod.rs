pub mod failures {
    use std::fmt::Debug;
    use std::path::PathBuf;
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum FailureResponse {
        #[error("Documentation not onboarded for table: {0}")]
        DocumentationNotOnboarded(String),

        #[error("Failed to parse {path}, error: {error}")]
        ParseError { path: String, error: String },

        #[error("Yaml malformed at path: {0}")]
        Malformed(String),

        #[error("Missing required field: {0}")]
        MissingField(String),

        #[error("Invalid path: {0}")]
        InvalidPath(String),

        #[error("I/O error at {path}")]
        IoError { path: PathBuf, #[source] source: std::io::Error },

        #[error("Serialization error at {path}")]
        SerializationError { path: PathBuf, #[source] source: bincode::Error },

        #[error(
            "Field mismatch for {field_name}: type={field_type} vs doc={doc_type}, constraints={field_constraints:?} vs doc={doc_constraints:?}"
        )]
        FieldMismatch {
            field_name: String,
            field_type: String,
            doc_type: String,
            field_constraints: Vec<String>,
            doc_constraints: Vec<String>,
        },
    }

    impl Clone for FailureResponse{
        fn clone(&self) -> Self {
            match self {
                Self::DocumentationNotOnboarded(s) => Self::DocumentationNotOnboarded(s.clone()),
                Self::ParseError { path, error } => Self::ParseError {
                    path: path.clone(),
                    error: error.clone(),
                },
                Self::Malformed(s) => Self::Malformed(s.clone()),
                Self::MissingField(s) => Self::MissingField(s.clone()),
                Self::InvalidPath(s) => Self::InvalidPath(s.clone()),
                Self::IoError { path, source } => Self::IoError {
                    path: path.clone(),
                    // io::Error isn't Clone; rebuild one that preserves kind + message.
                    source: std::io::Error::new(source.kind(), source.to_string()),
                },
                Self::SerializationError { path, source } => Self::SerializationError {
                    path: path.clone(),
                    // bincode::Error isn't Clone; rebuild via its Custom variant, preserving the message.
                    source: Box::new(bincode::ErrorKind::Custom(source.to_string())),
                },
                Self::FieldMismatch {
                    field_name,
                    field_type,
                    doc_type,
                    field_constraints,
                    doc_constraints,
                } => Self::FieldMismatch {
                    field_name: field_name.clone(),
                    field_type: field_type.clone(),
                    doc_type: doc_type.clone(),
                    field_constraints: field_constraints.clone(),
                    doc_constraints: doc_constraints.clone(),
                },
            }
        }
    }
}

pub mod validation_report {
    use std::process::{ExitCode, Termination};

    #[derive(Debug, PartialEq)]
    pub struct ValidationReport {
        pub files_processed: usize,
        pub tables_onboarded: usize,
    }

    impl Termination for ValidationReport {
        fn report(self) -> ExitCode {
            ExitCode::SUCCESS
        }
    }

    impl ValidationReport {
        pub fn new() -> ValidationReport {
            ValidationReport {
                files_processed: 0,
                tables_onboarded: 0,
            }
        }
    }
}
