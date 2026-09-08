use crate::doc_persistence::save_all;
use crate::service_response::{acceptable::AcceptableResponse, failures::FailureResponse};
use crate::table_yaml_definition::TableDoc;
use std::fs::read_to_string;
use std::path::PathBuf;

pub fn validate_yml_files(
    yaml_files: &[PathBuf],
) -> Result<AcceptableResponse, Vec<FailureResponse>> {
    let mut docs = Vec::new();
    let mut errors = Vec::new();

    for path in yaml_files {
        match validate_single_file(path) {
            Ok(doc) => docs.push(doc),
            Err(e) => errors.extend(e),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    save_all(&docs).map_err(|e| {
        vec![FailureResponse::IoError {
            error: e.to_string(),
            path: "unknown_path".to_string(),
        }]
    })?;

    Ok(AcceptableResponse::Ok)
}

fn validate_single_file(path: &PathBuf) -> Result<TableDoc, Vec<FailureResponse>> {
    let content = read_file(path).map_err(|e| vec![e])?;
    let table_doc = yaml_serde::from_str::<TableDoc>(&content).map_err(|e| {
        vec![FailureResponse::MissingField(format!(
            "File: {}, failed at {e}.",
            path.display()
        ))]
    })?;

    match table_doc.validate() {
        Ok(_) => Ok(table_doc),
        Err(e) => Err(e.into_iter().map(FailureResponse::Malformed).collect()),
    }
}

fn read_file(file_path: &PathBuf) -> Result<String, FailureResponse> {
    match read_to_string(file_path) {
        Ok(c) => Ok(c),
        Err(e) => Err(FailureResponse::Other {
            path: file_path.display().to_string(),
            error: e.to_string(),
        }),
    }
}

#[cfg(test)]
mod test {
    use crate::service_response::acceptable::AcceptableResponse;
    use crate::yaml_validation::{read_file, validate_yml_files};
    use std::env;
    use std::fs::{create_dir_all, remove_dir_all, write};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

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
        let temp_dir = "cli-yaml-read-validation".to_string();
        let dir = env::temp_dir().join(temp_dir);
        let file_path =
            create_file_with_content(&dir, VALID_YAML).expect("Failed to create valid test file");
        assert_eq!(VALID_YAML.trim(), read_file(&file_path).unwrap().trim());
        remove_dir_all(&dir).expect("Failed to remove test data at temp dir");
    }

    #[test]
    #[should_panic]
    fn test_read_file_panics() {
        let temp_dir = PathBuf::from("cli-yaml-read-validation");
        read_file(&temp_dir).unwrap();
    }

    #[test]
    fn test_validate_yaml_successful() {
        let temp_dir = "cli-yaml-read-validation".to_string();
        let dir = env::temp_dir().join(temp_dir);
        let file_path =
            create_file_with_content(&dir, VALID_YAML).expect("Failed to create valid test file");
        assert_eq!(
            AcceptableResponse::Ok,
            validate_yml_files(&vec![file_path]).unwrap()
        );
        remove_dir_all(&dir).expect("Failed to remove test data at temp dir");
    }
    #[test]
    fn test_validate_yaml_invalid_fails() {
        let temp_dir = "cli-yaml-read-validation".to_string();
        let dir = env::temp_dir().join(temp_dir);
        let file_path =
            create_file_with_content(&dir, INVALID_YAML).expect("Failed to create valid test file");
        let response = validate_yml_files(&vec![file_path]);
        assert!(response.is_err());
        remove_dir_all(&dir).expect("Failed to remove test data at temp dir");
    }

    fn create_file_with_content(
        dir: &PathBuf,
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
