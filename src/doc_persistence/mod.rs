use crate::service_response::failures::FailureResponse;
use crate::table_yaml_definition::{TableDoc};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::{env, fs};

/// Sanity cap to prevent bincode from trying to allocate petabytes
/// if the file is corrupted or tampered with.
const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;
static TABLE_CACHE: OnceLock<Result<Arc<HashMap<String, TableDoc>>, FailureResponse>> =
    OnceLock::new();


pub fn data_dir() -> PathBuf {
    env::var("WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::temp_dir())
        .join("data")
}

pub fn table_cache() -> Result<Arc<HashMap<String, TableDoc>>, FailureResponse> {
    TABLE_CACHE
        .get_or_init(|| {
            load_all().map_err(|e| FailureResponse::SerializationError {
                source: e,
                path: data_dir(),
            })
        })
        .clone()
}



pub fn save_all(docs: &[TableDoc]) -> bincode::Result<()> {
    save_all_to_path(docs, data_dir())
}

pub fn load_all() -> bincode::Result<Arc<HashMap<String, TableDoc>>> {
    load_all_from_path(data_dir())
}

fn save_all_to_path(docs: &[TableDoc], path: impl AsRef<Path>) -> bincode::Result<()> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    let data: HashMap<String, TableDoc> = docs
        .iter()
        .map(|entry| (entry.name.clone(), entry.clone()))
        .collect();
    bincode::serialize_into(writer, &data)
}

fn load_all_from_path(path: impl AsRef<Path>) -> bincode::Result<Arc<HashMap<String, TableDoc>>> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(Box::new(bincode::ErrorKind::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "data file not found",
        ))));
    }

    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_FILE_SIZE {
        return Err(Box::new(bincode::ErrorKind::Custom(format!(
            "data file too large ({} bytes); refusing to deserialize",
            metadata.len()
        ))));
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let map: HashMap<String, TableDoc> = bincode::deserialize_from(reader)?;
    Ok(Arc::new(map))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table_yaml_definition::ColumnDoc;
    use tempfile::TempDir;
    use crate::helper::from_str_collection_to_owned_string_vec;

    fn table(name: &str, cols: Vec<ColumnDoc>) -> TableDoc {
        TableDoc {
            name: name.to_string(),
            description: format!("{name} desc"),
            use_case: format!("{name} use"),
            columns: cols,
        }
    }

    fn col(name: &str, ty: &str, constraints: &[&str]) -> ColumnDoc {
        ColumnDoc {
            name: name.to_string(),
            r#type: ty.to_string(),
            constraints: from_str_collection_to_owned_string_vec(constraints),
            description: format!("{name} desc"),
            use_case: format!("{name} use"),
        }
    }

    // ── Roundtrip ────────────────────────────────────────────────

    #[test]
    fn roundtrip_single() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("db.bin");

        let docs = vec![table(
            "users",
            vec![col("id", "INTEGER", vec!["PRIMARY KEY"].as_ref())],
        )];

        save_all_to_path(&docs, &path).unwrap();
        let loaded = load_all_from_path(&path).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded["users"], docs[0]);
    }

    #[test]
    fn roundtrip_multiple() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("db.bin");

        let docs = vec![
            table(
                "users",
                vec![
                    col("id", "INTEGER", vec!["PRIMARY KEY"].as_ref()),
                    col("email", "TEXT", vec!["UNIQUE", "NOT NULL"].as_ref()),
                ],
            ),
            table(
                "posts",
                vec![
                    col("id", "INTEGER", vec!["PRIMARY KEY"].as_ref()),
                    col("title", "VARCHAR", vec![].as_ref()),
                ],
            ),
        ];

        save_all_to_path(&docs, &path).unwrap();
        let loaded = load_all_from_path(&path).unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded["users"], docs[0]);
        assert_eq!(loaded["posts"], docs[1]);
    }

    #[test]
    fn roundtrip_empty() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("empty.bin");

        let docs: Vec<TableDoc> = vec![];
        save_all_to_path(&docs, &path).unwrap();

        let loaded = load_all_from_path(&path).unwrap();
        assert!(loaded.is_empty());
    }

    // ── Overwrite ────────────────────────────────────────────────

    #[test]
    fn overwrite_replaces_data() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("db.bin");

        save_all_to_path(&[table("first", vec![])], &path).unwrap();
        save_all_to_path(&[table("second", vec![])], &path).unwrap();

        let loaded = load_all_from_path(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(loaded.contains_key("second"));
    }

    // ── Error cases (safe — no OOM) ─────────────────────────────

    #[test]
    fn load_missing_file_fails() {
        let dir = TempDir::new().unwrap();
        let result = load_all_from_path(dir.path().join("ghost.bin"));
        assert!(result.is_err());
    }

    #[test]
    fn load_empty_file_fails() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("zero.bin");
        File::create(&path).unwrap(); // 0 bytes

        let result = load_all_from_path(&path);
        assert!(result.is_err());
    }

    #[test]
    fn load_oversized_file_fails_early() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("huge.bin");
        // Create a sparse file larger than MAX_FILE_SIZE
        let file = File::create(&path).unwrap();
        file.set_len(MAX_FILE_SIZE + 1).unwrap();

        let result = load_all_from_path(&path);
        assert!(result.is_err());
    }

    // ── Integrity ────────────────────────────────────────────────

    #[test]
    fn preserves_all_fields() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("db.bin");

        let docs = vec![table(
            "products",
            vec![ColumnDoc {
                name: "price".to_string(),
                r#type: "DECIMAL(10,2)".to_string(),
                constraints: vec!["NOT NULL".into(), "CHECK (price > 0)".into()],
                description: "Product price".to_string(),
                use_case: "Pricing".to_string(),
            }],
        )];

        save_all_to_path(&docs, &path).unwrap();
        let loaded = load_all_from_path(&path).unwrap();

        let c = &loaded["products"].columns[0];
        assert_eq!(c.name, "price");
        assert_eq!(c.r#type, "DECIMAL(10,2)");
        assert_eq!(c.constraints, vec!["NOT NULL", "CHECK (price > 0)"]);
        assert_eq!(c.description, "Product price");
        assert_eq!(c.use_case, "Pricing");
    }
}
