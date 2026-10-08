use crate::helper::from_str_collection_to_owned_string_vec;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct TableDoc {
    pub name: String,
    pub description: String,
    pub use_case: String,
    pub columns: Vec<ColumnDoc>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct ColumnDoc {
    pub name: String,
    pub r#type: String,
    #[serde(default)]
    pub constraints: Vec<String>,
    pub description: String,
    pub use_case: String,
}
#[derive(Debug)]
pub struct TableInfo {
    pub name: String,
}

#[derive(Debug)]
pub struct ColumnInfo {
    pub name: String,
    pub r#type: String,
    pub constraints: Vec<String>,
}

impl ColumnInfo {
    const NON_COMPARABLE_CONSTRAINTS: &'static [&'static str] =
        &["PRIMARY KEY", "FOREIGN KEY", "UNIQUE", "CHECK", "EXCLUSION"];
    pub fn is_doc_equals(&self, doc: &ColumnDoc) -> bool {
        Self::eq_ignore_ascii_case_whitespace(&self.name, &doc.name)
            && Self::eq_ignore_ascii_case_whitespace(&self.r#type, &doc.r#type)
            && self.is_doc_constraints_equals(doc)
    }
    fn is_doc_constraints_equals(&self, doc: &ColumnDoc) -> bool {
        let is_comparable = |c: &str| {
            !Self::NON_COMPARABLE_CONSTRAINTS
                .iter()
                .any(|&constraint| c.contains(constraint))
        };

        let column_comparable_constraints: HashSet<_> = self
            .constraints
            .iter()
            .map(|s| s.to_uppercase())
            .filter(|c| is_comparable(c))
            .collect();

        let doc_comparable_constraints: HashSet<_> = doc
            .constraints
            .iter()
            .map(|s| s.to_uppercase())
            .filter(|c| is_comparable(c))
            .collect();

        column_comparable_constraints == doc_comparable_constraints
    }

    fn eq_ignore_ascii_case_whitespace(str1: &str, str2: &str) -> bool {
        let first = || str1.chars().filter(|c| !c.is_whitespace());
        let second = || str2.chars().filter(|c| !c.is_whitespace());
        first().count() == second().count()
            && first()
                .zip(second())
                .all(|(c1, c2)| c1.eq_ignore_ascii_case(&c2))
    }
}

impl TableDoc {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut problems: Vec<String> = Vec::with_capacity(4);
        check_empty("name", &self.name, &mut problems);
        check_empty("description", &self.description, &mut problems);
        check_empty("use_case", &self.use_case, &mut problems);

        if self.columns.is_empty() {
            problems.push(format!(
                "Table:{} must contain at least one column!",
                &self.name
            ));
        }
        for column in self.columns.iter() {
            if let Err(mut e) = column.validate() {
                problems.append(&mut e);
            }
        }
        if !problems.is_empty() {
            Err(problems)
        } else {
            Ok(())
        }
    }
}

impl ColumnDoc {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut problems: Vec<String> = Vec::with_capacity(4);
        check_empty("name", &self.name, &mut problems);
        check_empty("type", &self.r#type, &mut problems);
        check_empty("description", &self.description, &mut problems);
        check_empty("use_case", &self.use_case, &mut problems);
        if !problems.is_empty() {
            Err(problems)
        } else {
            Ok(())
        }
    }
}
const MIN_FIELD_LEN: usize = 2;

fn check_empty(field: &str, current_value: &str, problems: &mut Vec<String>) {
    if current_value.trim().len() < MIN_FIELD_LEN {
        problems.push(format!(
            "Field: {} is empty or does not contain enough characters!",
            field
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col_info(name: &str, type_: &str, constraints: &[&str]) -> ColumnInfo {
        ColumnInfo {
            name: name.to_string(),
            r#type: type_.to_string(),
            constraints: from_str_collection_to_owned_string_vec(constraints),
        }
    }

    fn col_doc(
        name: &str,
        type_: &str,
        constraints: &[&str],
        description: &str,
        use_case: &str,
    ) -> ColumnDoc {
        ColumnDoc {
            name: name.to_string(),
            r#type: type_.to_string(),
            constraints: from_str_collection_to_owned_string_vec(constraints),
            description: description.to_string(),
            use_case: use_case.to_string(),
        }
    }

    // ── Basic ──────────────────────────────────────────────────────

    #[test]
    fn exact_match() {
        let col = col_info("id", "INTEGER", ["NOT NULL"].as_ref());
        let doc = col_doc("id", "INTEGER", ["NOT NULL"].as_ref(), "", "");
        assert!(col.is_doc_equals(&doc));
    }

    #[test]
    fn case_insensitive_name_and_type() {
        let col = col_info("UserName", "VarChar", ["NOT NULL"].as_ref());
        let doc = col_doc("username", "varchar", ["NOT NULL"].as_ref(), "", "");
        assert!(col.is_doc_equals(&doc));
    }

    // ── Substring non-comparable filtering ─────────────────────────

    #[test]
    fn check_with_expression_filtered() {
        let col = col_info("age", "INTEGER", ["CHECK (age > 0)", "NOT NULL"].as_ref());
        let doc = col_doc("age", "INTEGER", ["NOT NULL"].as_ref(), "", "");
        assert!(col.is_doc_equals(&doc));
    }

    #[test]
    fn primary_key_with_column_name_filtered() {
        let col = col_info("id", "INTEGER", ["PRIMARY KEY (id)", "NOT NULL"].as_ref());
        let doc = col_doc("id", "INTEGER", ["NOT NULL"].as_ref(), "", "");
        assert!(col.is_doc_equals(&doc));
    }

    #[test]
    fn foreign_key_with_references_filtered() {
        let col = col_info(
            "user_id",
            "INTEGER",
            ["FOREIGN KEY REFERENCES users(id)", "NOT NULL"].as_ref(),
        );
        let doc = col_doc("user_id", "INTEGER", ["NOT NULL"].as_ref(), "", "");
        assert!(col.is_doc_equals(&doc));
    }

    #[test]
    fn unique_with_columns_filtered() {
        let col = col_info(
            "email",
            "TEXT",
            ["UNIQUE (email, domain)", "NOT NULL"].as_ref(),
        );
        let doc = col_doc("email", "TEXT", ["NOT NULL"].as_ref(), "", "");
        assert!(col.is_doc_equals(&doc));
    }

    #[test]
    fn exclusion_with_expression_filtered() {
        let col = col_info(
            "period",
            "TSTZRANGE",
            ["EXCLUSION USING gist (period WITH &&)", "NOT NULL"].as_ref(),
        );
        let doc = col_doc("period", "TSTZRANGE", ["NOT NULL"].as_ref(), "", "");
        assert!(col.is_doc_equals(&doc));
    }

    // ── Case-insensitive substring filtering ───────────────────────

    #[test]
    fn lowercase_check_filtered() {
        let col = col_info("age", "INTEGER", ["check (age > 0)"].as_ref());
        let doc = col_doc("age", "INTEGER", [].as_ref(), "", "");
        assert!(col.is_doc_equals(&doc));
    }

    #[test]
    fn mixed_case_primary_key_filtered() {
        let col = col_info("id", "INTEGER", ["Primary Key"].as_ref());
        let doc = col_doc("id", "INTEGER", [].as_ref(), "", "");
        assert!(col.is_doc_equals(&doc));
    }

    // ── Non-comparable ignored on both sides ─────────────────────

    #[test]
    fn different_non_comparable_both_filtered() {
        let col = col_info("id", "INTEGER", ["PRIMARY KEY", "NOT NULL"].as_ref());
        let doc = col_doc("id", "INTEGER", ["UNIQUE", "NOT NULL"].as_ref(), "", "");
        assert!(col.is_doc_equals(&doc));
    }

    #[test]
    fn all_non_comparable_only() {
        let col = col_info("id", "INTEGER", ["PRIMARY KEY", "CHECK (x > 0)"].as_ref());
        let doc = col_doc(
            "id",
            "INTEGER",
            ["FOREIGN KEY", "EXCLUSION"].as_ref(),
            "",
            "",
        );
        assert!(col.is_doc_equals(&doc));
    }

    // ── Comparable constraints must match as sets ─────────────────

    #[test]
    fn comparable_missing_in_doc() {
        let col = col_info("id", "INTEGER", ["NOT NULL", "DEFAULT 0"].as_ref());
        let doc = col_doc("id", "INTEGER", ["NOT NULL"].as_ref(), "", "");
        assert!(!col.is_doc_equals(&doc));
    }

    #[test]
    fn comparable_extra_in_doc() {
        let col = col_info("id", "INTEGER", ["NOT NULL"].as_ref());
        let doc = col_doc("id", "INTEGER", ["NOT NULL", "DEFAULT 0"].as_ref(), "", "");
        assert!(!col.is_doc_equals(&doc));
    }

    #[test]
    fn comparable_different_values() {
        let col = col_info("id", "INTEGER", ["DEFAULT 0"].as_ref());
        let doc = col_doc("id", "INTEGER", ["DEFAULT 1"].as_ref(), "", "");
        assert!(!col.is_doc_equals(&doc));
    }

    #[test]
    fn comparable_order_independent() {
        let col = col_info("id", "INTEGER", ["NOT NULL", "DEFAULT 0"].as_ref());
        let doc = col_doc("id", "INTEGER", ["DEFAULT 0", "NOT NULL"].as_ref(), "", "");
        assert!(col.is_doc_equals(&doc));
    }

    // ── Name / type mismatches ─────────────────────────────────────

    #[test]
    fn name_mismatch() {
        let col = col_info("id", "INTEGER", [].as_ref());
        let doc = col_doc("other_id", "INTEGER", [].as_ref(), "", "");
        assert!(!col.is_doc_equals(&doc));
    }

    #[test]
    fn type_mismatch() {
        let col = col_info("id", "INTEGER", [].as_ref());
        let doc = col_doc("id", "TEXT", [].as_ref(), "", "");
        assert!(!col.is_doc_equals(&doc));
    }

    // ── description / use_case ignored ─────────────────────────────

    #[test]
    fn description_and_use_case_ignored() {
        let col = col_info("id", "INTEGER", ["NOT NULL"].as_ref());
        let doc = col_doc(
            "id",
            "INTEGER",
            ["NOT NULL"].as_ref(),
            "any desc",
            "any use case",
        );
        assert!(col.is_doc_equals(&doc));
    }
}
