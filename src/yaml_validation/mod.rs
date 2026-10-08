use crate::doc_persistence::save_all;
use crate::service_response::failures::FailureResponse;
use crate::service_response::validation_report::ValidationReport;
use crate::table_yaml_definition::TableDoc;
use std::fs::read_to_string;
use std::path::PathBuf;

pub fn validate_yml_files(
    yaml_files: &[PathBuf],
) -> Result<ValidationReport, Vec<FailureResponse>> {
    let mut docs = Vec::new();
    let mut errors = Vec::new();

    for path in yaml_files {
        match validate_single_file(path) {
            Ok(doc) => docs.push(doc),
            Err(e) => errors.push(e),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    persist_docs(&docs).map_err(|e| vec![e])?;

    Ok(ValidationReport {
        tables_onboarded: docs.len(),
        files_processed: yaml_files.len(),
    })
}

fn persist_docs(docs: &[TableDoc]) -> Result<(), FailureResponse> {
    save_all(&docs).map_err(|e| FailureResponse::SerializationError {
        source: e,
        path: PathBuf::new(),
    })
}

fn validate_single_file(path: &PathBuf) -> Result<TableDoc, FailureResponse> {
    let content = read_file(path)?;
    let doc: TableDoc = yaml_serde::from_str(&content).map_err(|e| {
        FailureResponse::Malformed(format!("File:{}, failed at {e}", path.display()))
    })?;
    doc.validate().map_err(|problems| {
        FailureResponse::MissingField(format!("{}: {}", path.display(), problems.join(", ")))
    })?;
    Ok(doc)
}

fn read_file(file_path: &PathBuf) -> Result<String, FailureResponse> {
    match read_to_string(file_path) {
        Ok(c) => Ok(c),
        Err(e) => Err(FailureResponse::IoError {
            path: file_path.clone(),
            source: e,
        }),
    }
}

#[cfg(test)]
mod test {
    use crate::service_response::failures::FailureResponse;
    use crate::service_response::validation_report::ValidationReport;
    use crate::yaml_validation::{read_file, validate_yml_files};
    use std::fs::{create_dir_all, write};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tempfile::TempDir;

    const VALID_YAML: &str = r#"
    name: "users"
    description: "Stores core user profile data and account settings."
    use_case: "User authentication, profile management, and account authorization."
    columns:
      - name: "id"
        type: "INTEGER"
        constraints:
          - "PRIMARY KEY"
          - "AUTOINCREMENT"
        description: "Unique surrogate key for each user account."
        use_case: "Internal database joins and API resource routing."
    "#;

    const INVALID_YAML: &str = r#"
    name: "users"
    use_case: "User authentication, profile management, and account authorization."
    description: ""
    columns:
      - name: "id"
        type: "INTEGER"
        constraints:
          - "PRIMARY KEY"
          - "AUTOINCREMENT"
        description: "Unique surrogate key for each user account."
        use_case: "Internal database joins and API resource routing."
    "#;

    #[test]
    fn test_read_file_successful() {
        let temp_dir = TempDir::new().unwrap();
        let file_path =
            create_file_with_content(&temp_dir.path(), VALID_YAML).expect("Failed to create valid test file");
        assert_eq!(VALID_YAML.trim(), read_file(&file_path).unwrap().trim());
    }

    #[test]
    fn test_read_file_panics() {
        let temp_dir = PathBuf::from("cli-yaml-read-validation");
        let Err(FailureResponse::IoError { .. }) = read_file(&temp_dir) else {
            panic!("expected IoError");
        };
    }

    #[test]
    fn test_validate_yaml_successful() {
        let dir = TempDir::new().unwrap();
        let file_path =
            create_file_with_content(&dir.path(), VALID_YAML).expect("Failed to create valid test file");
        assert_eq!(
            ValidationReport::new(),
            validate_yml_files(&vec![file_path]).unwrap()
        );
    }
    #[test]
    fn test_validate_yaml_invalid_fails() {
        let dir = TempDir::new().unwrap();
        let file_path =
            create_file_with_content(&dir.path(), INVALID_YAML).expect("Failed to create valid test file");
        let response = validate_yml_files(&vec![file_path]);
        assert!(response.is_err());
    }

    fn create_file_with_content(
        dir: &Path,
        contents: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        create_dir_all(&dir)?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let file_path = dir.join(format!("{}.yml", timestamp));

        write(&file_path, contents)?;
        Ok(file_path)
    }
}
