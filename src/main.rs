pub mod doc_persistence;
pub mod doc_validation;
pub mod helper;
pub mod service_mode;
pub mod service_response;
pub mod table_yaml_definition;
pub mod yaml_validation;

use crate::doc_validation::validate;
use crate::service_mode::Mode;
use crate::service_response::failures::FailureResponse;
use crate::service_response::validation_report::ValidationReport;
use clap::Parser;
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
use yaml_validation::validate_yml_files;

#[derive(Parser, Debug)]
#[command(name = "cli-data-doc", version, about)]
struct Args {
    #[arg(default_value = ".", help = "Document repository")]
    repo: PathBuf,
    #[arg(short, long, value_enum, default_value_t = Mode::Validation)]
    mode: Mode,
    #[arg(long, default_value = ".", help = "Workspace root for persisted data")]
    workspace: PathBuf,
}

fn main() -> Result<ValidationReport, Vec<FailureResponse>> {
    let args = Args::parse();
    let hard_path = r"C:\Users\BRUNOH~1\AppData\Local\Temp\rust_test_data".to_string();

    //let repo_dir = env::args_os().nth(1).unwrap_or_else(|| ".".into());
    let path = &PathBuf::new().join(&hard_path);
    let mode = Mode::Validation;

    match mode {
        Mode::Documentation => run_documentation(&args.repo),
        Mode::Validation => run_validation(&args.repo),
    }
}

fn run_documentation(path: &PathBuf) -> Result<ValidationReport, Vec<FailureResponse>> {
    println!("Starting repo scanning at directory: {}", path.display());

    let available_files = scan_for_file_extension(path, is_yaml)?;
    println!("Found {} YAML file(s).", available_files.len());
    report(validate_yml_files(&available_files),"Documentation")
}

fn run_validation(path: &PathBuf) -> Result<ValidationReport, Vec<FailureResponse>> {
    println!("Starting repo scanning at directory: {}", path.display());
    let available_files = scan_for_file_extension(path, is_sql)?;
    println!("Found {} SQL file(s).", available_files.len());
    report(validate(&available_files), "Validation")
}

fn report(
    result: Result<ValidationReport, Vec<FailureResponse>>,
    noun: &str,
) -> Result<ValidationReport, Vec<FailureResponse>> {
    match result {
        Ok(r) => {
            println!("{noun} approved");
            Ok(r)
        }
        Err(errors) => {
            eprint!("The following failures were found:");
            for e in &errors {
                eprint!("{e}");
            }
            Err(errors)
        }
    }
}

fn scan_for_file_extension<F>(
    path: &PathBuf,
    filter: F,
) -> Result<Vec<PathBuf>, Vec<FailureResponse>>
where
    F: Fn(&PathBuf) -> bool,
{
    if !path.is_dir() {
        return Err(vec![FailureResponse::InvalidPath(format!(
            "The directory: {} is not valid!",
            path.display()
        ))]);
    }
    let entries = fs::read_dir(path).map_err(|e| vec![io_failure(path.clone(), e)])?;

    let mut matches = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| vec![io_failure(path.clone(), e)])?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            matches.extend(scan_for_file_extension(&entry_path, &filter)?);
        } else if filter(&entry_path) {
            matches.push(entry_path);
        }
    }
    Ok(matches)
}

fn is_yaml(path: &PathBuf) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("yml") || ext.eq_ignore_ascii_case("yaml"))
}

fn is_sql(path: &PathBuf) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("sql"))
}

fn io_failure(path_buf: PathBuf, e: std::io::Error) -> FailureResponse {
    FailureResponse::IoError {
        path: path_buf,
        source: e,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs::{create_dir_all, write};
    use rand::RngExt;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tempfile::TempDir;

    #[test]
    fn test_is_yaml() {
        assert!(is_yaml(&PathBuf::new().join("/tmp/test.yml")));
        assert!(is_yaml(&PathBuf::new().join("/tmp/test.yaml")));
        assert!(!is_yaml(&PathBuf::new().join("/tmp/test.txt")));
    }

    #[test]
    #[should_panic]
    fn test_scan_and_print_paths_fails_when_path_not_exists() {
        scan_for_file_extension(&PathBuf::new().join("false_path"), is_yaml)
            .expect("Missing directory");
    }

    #[test]
    fn scan_recursively_finds_only_yaml() {
        let dir = TempDir::new().unwrap();
        for f in ["a.yml", "b.yaml", "sub/c.yml", "sub/d.txt", "e.json"] {
            let p = dir.path().join(f);
            create_dir_all(p.parent().unwrap()).unwrap();
            write(&p, "x").unwrap();
        }
        assert_eq!(scan_for_file_extension(&dir.path().join(""), is_yaml).unwrap().len(), 3);
    }

}
