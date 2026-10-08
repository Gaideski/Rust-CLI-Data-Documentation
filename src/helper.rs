pub fn from_str_collection_to_owned_string_vec(input: &[&str]) -> Vec<String> {
    input.iter().map(|&s| s.to_owned()).collect()
}
