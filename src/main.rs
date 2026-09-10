pub mod doc_persistence;
pub mod doc_validation;
pub mod service_response;
pub mod table_yaml_definition;
pub mod yaml_validation;
pub mod service_mode;

use crate::doc_validation::validate;
use crate::service_mode::MODE;
use crate::service_response::failures::FailureResponse;
use crate::service_response::validation_report::ValidationReport;
use clap::Parser;
use std::path::PathBuf;
use std::{fs, path::Path};
use yaml_validation::validate_yml_files;

#[derive(Parser, Debug)]
#[command(name = "cli-data-doc", version, about)]
struct Args {
    #[arg(default_value = ".", help = "Document repository")]
    repo: PathBuf,
    #[arg(short, long, value_enum, default_value_t = MODE::VALIDATION)]
    mode: MODE,
    #[arg(long, default_value = ".", help = "Workspace root for persisted data")]
    workspace: PathBuf,
}

fn main() -> Result<ValidationReport, Vec<FailureResponse>> {
    let args = Args::parse();
    let hard_path = r"C:\Users\BRUNOH~1\AppData\Local\Temp\rust_test_data".to_string();

    //let repo_dir = env::args_os().nth(1).unwrap_or_else(|| ".".into());
    let path = Path::new(&hard_path);
    let mode = MODE::VALIDATION;

    match mode {
        MODE::DOCUMENTATION => run_documentation(&args.repo),
        MODE::VALIDATION => run_validation(&args.repo),
    }
}

fn run_documentation(path: &Path) -> Result<ValidationReport, Vec<FailureResponse>> {
    println!("Starting repo scanning at directory: {}", path.display());

    let available_files = scan_for_file_extension(path, is_yaml)?;
    println!("Found {} YAML file(s).", available_files.len());

    match validate_yml_files(&available_files) {
        Ok(report) => {
            println!("Documentation approved");
            Ok(report)
        }
        Err(errors) => {
            print!("The following failures were found:");
            println!(
                "{}",
                errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            Err(errors)
        }
    }
}

fn run_validation(path: &Path) -> Result<ValidationReport, Vec<FailureResponse>> {
    println!("Starting repo scanning at directory: {}", path.display());
    let available_files = scan_for_file_extension(path, is_sql)?;
    println!("Found {} SQL file(s).", available_files.len());
    match validate(&available_files) {
        Ok(report) => {
            println!("SQL files approved");
            Ok(report)
        }
        Err(errors) => {
            print!("The following failures were found:");
            println!(
                "{}",
                errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            Err(errors)
        }
    }
}
fn scan_for_file_extension<F>(path: &Path, filter: F) -> Result<Vec<PathBuf>, Vec<FailureResponse>>
where
    F: Fn(&Path) -> bool,
{
    if !path.is_dir() {
        return Err(vec![FailureResponse::InvalidPath(format!(
            "The directory: {} is not valid!",
            path.display()
        ))]);
    }
    let entries = fs::read_dir(path).map_err(|e| vec![io_failure(path, &e)])?;

    let mut yaml_files = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| vec![io_failure(path, &e)])?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            yaml_files.extend(scan_for_file_extension(&entry_path, &filter)?);
        } else if filter(&entry_path) {
            yaml_files.push(entry_path);
        }
    }
    Ok(yaml_files)
}

fn is_yaml(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map_or(false, |ext| {
            ext.eq_ignore_ascii_case("yml") || ext.eq_ignore_ascii_case("yaml")
        })
}

fn is_sql(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map_or(false, |ext| ext.eq_ignore_ascii_case("sql"))
}

fn io_failure(path_buf: &Path, e: &std::io::Error) -> FailureResponse {
    FailureResponse::IoError {
        path: path_buf.display().to_string(),
        error: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs::{create_dir_all, remove_dir_all, write};
    use std::env;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};
    use rand::RngExt;
    use tempfile::TempDir;

    #[test]
    fn test_is_yaml() {
        assert!(is_yaml(Path::new("/tmp/test.yml")));
        assert!(is_yaml(Path::new("/tmp/test.yaml")));
        assert!(!is_yaml(Path::new("/tmp/test.txt")));
    }

    #[test]
    #[should_panic]
    fn test_scan_and_print_paths_fails_when_path_not_exists() {
        scan_for_file_extension(Path::new("false_path"), is_yaml).expect("Missing directory");
    }

    #[test]
    fn test_scan_and_print_paths_successful_detection() {
        let test_dir = TempDir::new().unwrap();

        let expected_yaml_count =
            generate_random_dir_struct(&test_dir.path()).expect("Failed to setup test dir");
        let found_files = scan_for_file_extension(&test_dir.path(), is_yaml).expect("Scan failed");
        assert_eq!(expected_yaml_count, found_files.len());

        // Clean up ONLY the test directory created for this run
        let _ = remove_dir_all(&test_dir);
    }

    fn generate_random_dir_struct(temp_dir: &Path) -> Result<usize, Box<dyn std::error::Error>> {
        let file_extensions = vec![".txt", ".yml", ".yaml", ".jpg", ".json", ".png"];
        let mut rng = rand::rng();

        create_dir_all(temp_dir)?;

        let mut expected_yaml_count = 0;

        for index in 0..100 {
            let mut file_dir = temp_dir.to_path_buf();
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();

            if index % 10 == 0 {
                file_dir.push(format!("{}_{}", index, timestamp));
                create_dir_all(&file_dir)?;
            }

            let sorted_ext = file_extensions[rng.random_range(0..file_extensions.len())];
            file_dir.push(format!("file{}_{}", timestamp, sorted_ext));
            write(&file_dir, "PLACEHOLDER")?;

            if is_yaml(&file_dir) {
                expected_yaml_count += 1;
            }
        }

        Ok(expected_yaml_count)
    }
}

