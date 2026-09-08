pub mod failures {
    use std::fmt;
    use std::fmt::{Debug, Formatter};

    #[derive(Debug, PartialOrd, PartialEq)]
    pub enum FailureResponse {
        DocumentationNotOnboarded(String),
        ParseError {
            contents: String,
            error: String,
        },
        Malformed(String),
        MissingField(String),
        InvalidPath(String),
        Other {
            path: String,
            error: String,
        },
        IoError {
            path: String,
            error: String,
        },
        FieldMismatch {
            field_name: String,
            field_type: String,
            doc_type: String,
            field_constraints: Vec<String>,
            doc_constraints: Vec<String>,
        },
    }

    impl std::error::Error for FailureResponse {}
    impl fmt::Display for FailureResponse {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            match self {
                FailureResponse::DocumentationNotOnboarded(table) => {
                    write!(f, "Documentation for the table: {} is not onboarded", table)
                }
                FailureResponse::Malformed(path) => {
                    write!(f, "Yaml: {} is malformed", path)
                }
                FailureResponse::MissingField(fields) => {
                    write!(f, "Yaml is missing required fields: {:?}", fields)
                }
                FailureResponse::InvalidPath(path) => {
                    write!(f, "Incorrect dir: {}", path)
                }
                FailureResponse::ParseError {
                    contents: c,
                    error: e,
                } => {
                    write!(f, "Failed to parse file: {}, Contents: {}", c, e)
                }
                FailureResponse::Other { path: p, error: e } => {
                    write!(f, "Failed to read file: {}, Err: {}", p, e)
                }
                FailureResponse::IoError { path: p, error: e } => {
                    write!(f, "Failed to read file: {}, Err: {}", p, e)
                }
                FailureResponse::FieldMismatch {
                    field_name,
                    field_type,
                    doc_type,
                    field_constraints,
                    doc_constraints,
                } => {
                    let fc = field_constraints
                        .iter()
                        .fold(String::new(), |s1, s2| s1 + ", " + s2);
                    let dc = doc_constraints
                        .iter()
                        .fold(String::new(), |s1, s2| s1 + ", " + s2);
                    write!(
                        f,
                        "Conflicting information about field: {}, \
                    Field Type: {}\n
                    Field Constraints: {}\n
                    Documented Type: {}\n
                    Documented Constraints: {}",
                        field_name, field_type, fc, doc_type, dc
                    )
                }
            }
        }
    }
}

pub mod acceptable {
    use std::fmt;
    use std::fmt::Formatter;

    #[derive(Debug, PartialOrd, PartialEq)]
    pub enum AcceptableResponse {
        MultipleDefinitions { column: String, tables: Vec<String> },
        Ok,
    }
    impl fmt::Display for AcceptableResponse {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            match self {
                AcceptableResponse::MultipleDefinitions {
                    column: c,
                    tables: t,
                } => {
                    write!(
                        f,
                        "The column: {} has multiple definitions at the tables: {}",
                        c,
                        t.into_iter()
                            .fold(String::new(), |s1, s2| s1 + "-" + s2)
                            .trim_start()
                            .to_string()
                    )
                }
                AcceptableResponse::Ok => {
                    write!(f, "Well done, LGTM!")
                }
            }
        }
    }
}
