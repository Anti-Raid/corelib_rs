use crate::Error;

/// Parses a shop template of form template_name#version
pub fn parse_shop_template(s: &str) -> Result<(String, String), Error> {
    let s = s.trim_start_matches("$shop/");
    let (template, version) = match s.split_once('#') {
        Some((template, version)) => (template, version),
        None => return Err("Invalid shop template".into()),
    };

    Ok((template.to_string(), version.to_string()))
}

/// Creates a shop template string given name and version
pub fn create_shop_template(template: &str, version: &str) -> String {
    format!("$shop/{}#{}", template, version)
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct LuaKVConstraints {
    /// Maximum number of keys allowed in the KV store
    pub max_keys: usize,
    /// Maximum length of a key
    pub max_key_length: usize,
    /// Maximum length of a value (in bytes)
    pub max_value_bytes: usize,
    /// Maximum length of a object storage path
    pub max_object_storage_path_length: usize,
    /// Maximum length of a object storage data
    pub max_object_storage_bytes: usize,
}

impl Default for LuaKVConstraints {
    fn default() -> Self {
        LuaKVConstraints {
            max_keys: 10000,
            max_key_length: 512,
            // 256kb max per value
            max_value_bytes: 256 * 1024,
            max_object_storage_path_length: 2048
            // 512kb max per value
            max_object_storage_bytes: 512 * 1024
        }
    }
}
