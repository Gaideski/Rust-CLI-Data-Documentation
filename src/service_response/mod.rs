pub mod failures {
    use std::fmt;
    use std::fmt::{Debug, Formatter};
    use thiserror::Error;

    #[derive(Debug, PartialOrd, PartialEq, Error, Clone)]
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

        #[error("I/O error at {path}: {error}")]
        IoError { path: String, error: String },
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
