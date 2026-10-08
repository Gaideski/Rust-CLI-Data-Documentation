use crate::doc_persistence::table_cache;
use crate::service_response::failures::FailureResponse;
use crate::service_response::validation_report::ValidationReport;
use crate::table_yaml_definition::{ColumnInfo, TableDoc, TableInfo};
use sqlparser::ast::{CreateTable, ObjectNamePart, Statement};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const DIALECT: GenericDialect = GenericDialect {};

pub fn validate(files: &[PathBuf]) -> Result<ValidationReport, Vec<FailureResponse>> {
    let mut errors: Vec<FailureResponse> = Vec::new();
    let mut validation_report: ValidationReport = ValidationReport::new();

    for file in files {
        match validate_file(file) {
            Ok(report) => {
                validation_report.tables_onboarded += report.tables_onboarded;
                validation_report.files_processed += report.files_processed;
            }
            Err(e) => {
                errors.extend(e);
            }
        }
    }

    if !errors.is_empty() {
        Err(errors)
    } else {
        Ok(validation_report)
    }
}

fn validate_file(file: &PathBuf) -> Result<ValidationReport, Vec<FailureResponse>> {
    let contents = fs::read_to_string(file).map_err(|e| {
        vec![FailureResponse::IoError {
            path: file.clone(),
            source: e,
        }]
    })?;
    validate_single_file(&contents, file)
}

fn validate_single_file(
    query: &str,
    filename: &PathBuf,
) -> Result<ValidationReport, Vec<FailureResponse>> {
    let mut errors = Vec::new();
    let statements = match Parser::parse_sql(&DIALECT, query) {
        Ok(stms) => filter_create_table(stms),
        Err(e) => {
            errors.push(FailureResponse::ParseError {
                path: filename.display().to_string(),
                error: e.to_string(),
            });
            return Err(errors);
        }
    };
    let create_table_single_file_count = statements.len();

    if let Err(mut info_errors) = extract_and_compare_info(&statements) {
        errors.append(&mut info_errors);
    }

    if errors.is_empty() {
        Ok(ValidationReport {
            files_processed: 1,
            tables_onboarded: create_table_single_file_count,
        })
    } else {
        Err(errors)
    }
}

fn extract_and_compare_info(statements: &[CreateTable]) -> Result<(), Vec<FailureResponse>> {
    let mut errors: Vec<FailureResponse> = Vec::new();
    for stm in statements {
        let table = extract_table_info(&stm);
        let cache = table_cache().map_err(|e| vec![e]);
        match verify_table_doc(&table, cache?.as_ref()) {
            Ok(table_doc) => {
                let columns = extract_columns_info(&stm);
                verify_columns_doc(&table_doc, &columns, &mut errors)
            }
            Err(e) => errors.push(e),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn verify_table_doc(
    query_table: &TableInfo,
    cache: &HashMap<String, TableDoc>,
) -> Result<TableDoc, FailureResponse> {
    cache.get(&query_table.name).cloned().ok_or_else(|| {
        FailureResponse::DocumentationNotOnboarded(format!(
            "Documentation not found for table: {}",
            query_table.name
        ))
    })
}

fn verify_columns_doc(
    table_doc: &TableDoc,
    columns: &[ColumnInfo],
    errors: &mut Vec<FailureResponse>,
) {
    for column in columns {
        match table_doc
            .columns
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(&column.name))
        {
            None => errors.push(FailureResponse::MissingField(column.name.clone())),
            Some(doc) if column.is_doc_equals(doc) => {}
            Some(doc) => errors.push(FailureResponse::FieldMismatch {
                field_name: column.name.to_owned(),
                field_type: column.r#type.to_owned(),
                doc_type: doc.r#type.to_owned(),
                field_constraints: column.constraints.clone(),
                doc_constraints: doc.constraints.clone(),
            }),
        }
    }
}

fn filter_create_table(statements: Vec<Statement>) -> Vec<CreateTable> {
    statements
        .into_iter()
        .filter_map(|stmt| match stmt {
            Statement::CreateTable(ct) => Some(ct),
            _ => None,
        })
        .collect()
}

fn extract_table_info(table: &CreateTable) -> TableInfo {
    let table_name = table
        .name
        .0
        .last()
        .and_then(|part| match part {
            ObjectNamePart::Identifier(ident) => Some(ident.value.clone()),
            _ => None,
        })
        .unwrap_or_default();
    TableInfo { name: table_name }
}

fn extract_columns_info(table: &CreateTable) -> Vec<ColumnInfo> {
    table
        .columns
        .iter()
        .map(|column| ColumnInfo {
            name: column.name.value.clone(),
            r#type: column.data_type.to_string(),
            constraints: column
                .options
                .iter()
                .map(|opt| opt.option.to_string())
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helper::from_str_collection_to_owned_string_vec;
    use crate::table_yaml_definition::ColumnDoc;

    const VALID_CREATE_QUERY: &str = r#"
    CREATE TABLE Persons (
      PersonID int PRIMARY KEY,
      LastName varchar(255) NOT NULL,
      FirstName varchar(255),
      Address varchar(255),
      City varchar(255)
    );
    "#;

    #[test]
    fn test_extract_table_names_success() {
        let create_statement =
            filter_create_table(Parser::parse_sql(&DIALECT, VALID_CREATE_QUERY).unwrap());
        let result = extract_table_info(&create_statement.first().unwrap().clone());
        assert_eq!(result.name, "Persons")
    }

    #[test]
    fn test_extract_columns_info_success() {
        let create_statement =
            filter_create_table(Parser::parse_sql(&DIALECT, VALID_CREATE_QUERY).unwrap());
        let result = extract_columns_info(&create_statement.first().unwrap().clone());
        assert!(!result.is_empty());
        assert_eq!(result.get(0).unwrap().name, "PersonID");
    }

    #[test]
    fn test_verify_table_doc_not_onboarded() {
        let mut mocked_table_info = create_sample_table_info();
        mocked_table_info.name = "Not what I was expecting".to_string();
        let cache = HashMap::new();
        let Err(e) = verify_table_doc(&mocked_table_info, &cache) else {
            panic!("Should fail here")
        };
        assert!(matches!(e, FailureResponse::DocumentationNotOnboarded(_)))
    }

    // #[test]
    // fn test_verify_table_doc_onboarded() {
    //     let mocked_table_info = create_sample_table_info();
    //     let Ok(e) = verify_table_doc(&mocked_table_info) else {
    //         panic!("Should fail here")
    //     };
    //     assert!(matches!(e, e))
    // }

    #[test]
    fn test_verify_column_doc_missing() {
        let mut mocked_column_info = create_sample_column_info();
        let mocked_table_doc = create_sample_table_doc();
        let mut err = Vec::new();
        mocked_column_info
            .get_mut(0)
            .expect("sample column info should have at least one entry")
            .name = "Not what I was expecting".to_string();
        verify_columns_doc(&mocked_table_doc, &mocked_column_info, &mut err);
        assert_eq!(err.len(), 1);
        assert!(matches!(
            err.get(0).unwrap(),
            FailureResponse::MissingField(_)
        ));
    }

    #[test]
    fn test_verify_column_doc_fails_details_unmatched() {
        let mut mocked_column_info = create_sample_column_info();
        let mocked_table_doc = create_sample_table_doc();
        let mut err = vec![];
        mocked_column_info
            .get_mut(0)
            .expect("sample column info should have at least one entry")
            .constraints
            .push("NOT NULL".to_string());

        verify_columns_doc(&mocked_table_doc, &mocked_column_info, &mut err);
        assert_eq!(err.len(), 1);
        assert!(matches!(
            err.get(0).unwrap(),
            FailureResponse::FieldMismatch {
                field_name: _,
                field_type: _,
                doc_type: _,
                field_constraints: _,
                doc_constraints: _
            }
        ));
    }

    #[test]
    fn test_verify_column_doc_success() {
        let mocked_column_info = create_sample_column_info();
        let mocked_table_doc = create_sample_table_doc();
        let mut err = vec![];
        verify_columns_doc(&mocked_table_doc, &mocked_column_info, &mut err);
        assert!(err.is_empty());
    }

    #[test]
    fn test_verify_column_doc_success_with_non_comparable_constraints() {
        let mut mocked_column_info = create_sample_column_info();
        let mocked_table_doc = create_sample_table_doc();
        let mut err = vec![];

        mocked_column_info
            .get_mut(0)
            .expect("sample column info should have at least one entry")
            .constraints
            .push("UNIQUE".to_string());

        verify_columns_doc(&mocked_table_doc, &mocked_column_info, &mut err);
        assert!(err.is_empty());
    }

    fn create_sample_table_info() -> TableInfo {
        TableInfo {
            name: "Person".to_string(),
        }
    }

    fn column_info_builder(name: &str, r#type: &str, constraints: &[&str]) -> ColumnInfo {
        ColumnInfo {
            name: name.to_string(),
            r#type: r#type.to_string(),
            constraints: from_str_collection_to_owned_string_vec(constraints),
        }
    }
    fn create_sample_column_info() -> Vec<ColumnInfo> {
        vec![
            column_info_builder("PersonID", "int", ["PRIMARY KEY"].as_ref()),
            column_info_builder("LastName", "varchar(255)", ["NOT NULL"].as_ref()),
            column_info_builder("FirstName", "varchar(255)", [].as_ref()),
            column_info_builder("Address", "varchar(255)", [].as_ref()),
            column_info_builder("City", "varchar(255)", [].as_ref()),
        ]
    }

    fn create_sample_table_doc() -> TableDoc {
        TableDoc {
            name: "Person".to_string(),
            description: "This is a test sample".to_string(),
            use_case: "test".to_string(),
            columns: create_sample_column_doc(),
        }
    }

    fn column_doc_builder(
        name: &str,
        r#type: &str,
        constraints: &[&str],
        description: &str,
        use_case: &str,
    ) -> ColumnDoc {
        ColumnDoc {
            name: name.to_string(),
            r#type: r#type.to_string(),
            constraints: from_str_collection_to_owned_string_vec(constraints),
            description: description.to_string(),
            use_case: use_case.to_string(),
        }
    }
    fn create_sample_column_doc() -> Vec<ColumnDoc> {
        vec![
            column_doc_builder(
                "PersonID",
                "int",
                ["PRIMARY KEY"].as_ref(),
                "Id",
                "table identifier",
            ),
            column_doc_builder(
                "LastName",
                "varchar(255)",
                ["NOT NULL"].as_ref(),
                "Last name",
                "person's surname",
            ),
            column_doc_builder(
                "FirstName",
                "varchar(255)",
                [].as_ref(),
                "First name",
                "person's given name",
            ),
            column_doc_builder(
                "Address",
                "varchar(255)",
                [].as_ref(),
                "Address",
                "person's residential address",
            ),
            column_doc_builder(
                "City",
                "varchar(255)",
                [].as_ref(),
                "City",
                "person's city of residence",
            ),
        ]
    }
}
