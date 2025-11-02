//! Property system for flexible node and edge metadata.
//!
//! Provides type-safe property storage with a builder pattern.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Strongly-typed property value for node/edge metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyValue {
    /// String value (names, paths, signatures)
    String(String),
    /// Integer value (line numbers, counts)
    Int(i64),
    /// Floating point value (metrics, scores)
    Float(f64),
    /// Boolean flag (is_public, is_async, is_test)
    Bool(bool),
    /// List of strings (imported symbols, tags)
    StringList(Vec<String>),
    /// List of integers (line ranges, counts)
    IntList(Vec<i64>),
    /// Explicit null/absence of value
    Null,
}

impl From<String> for PropertyValue {
    fn from(value: String) -> Self {
        PropertyValue::String(value)
    }
}

impl From<&str> for PropertyValue {
    fn from(value: &str) -> Self {
        PropertyValue::String(value.to_string())
    }
}

impl From<i64> for PropertyValue {
    fn from(value: i64) -> Self {
        PropertyValue::Int(value)
    }
}

impl From<i32> for PropertyValue {
    fn from(value: i32) -> Self {
        PropertyValue::Int(value as i64)
    }
}

impl From<f64> for PropertyValue {
    fn from(value: f64) -> Self {
        PropertyValue::Float(value)
    }
}

impl From<bool> for PropertyValue {
    fn from(value: bool) -> Self {
        PropertyValue::Bool(value)
    }
}

impl From<Vec<String>> for PropertyValue {
    fn from(value: Vec<String>) -> Self {
        PropertyValue::StringList(value)
    }
}

impl From<Vec<i64>> for PropertyValue {
    fn from(value: Vec<i64>) -> Self {
        PropertyValue::IntList(value)
    }
}

/// Flexible key-value metadata store for nodes and edges.
///
/// Provides builder pattern and type-safe getters for properties.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PropertyMap {
    data: HashMap<String, PropertyValue>,
}

impl PropertyMap {
    /// Create a new empty property map.
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Builder pattern: add a property and return self.
    pub fn with(mut self, key: impl Into<String>, value: impl Into<PropertyValue>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }

    /// Insert a property value.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<PropertyValue>) {
        self.data.insert(key.into(), value.into());
    }

    /// Get a property value by key.
    pub fn get(&self, key: &str) -> Option<&PropertyValue> {
        self.data.get(key)
    }

    /// Remove a property by key.
    pub fn remove(&mut self, key: &str) -> Option<PropertyValue> {
        self.data.remove(key)
    }

    /// Check if a property exists.
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Get the number of properties.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the property map is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Iterate over all properties.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &PropertyValue)> {
        self.data.iter()
    }

    /// Type-safe getter for string properties.
    pub fn get_string(&self, key: &str) -> Option<&str> {
        match self.data.get(key) {
            Some(PropertyValue::String(s)) => Some(s),
            _ => None,
        }
    }

    /// Type-safe getter for integer properties.
    pub fn get_int(&self, key: &str) -> Option<i64> {
        match self.data.get(key) {
            Some(PropertyValue::Int(i)) => Some(*i),
            _ => None,
        }
    }

    /// Type-safe getter for float properties.
    pub fn get_float(&self, key: &str) -> Option<f64> {
        match self.data.get(key) {
            Some(PropertyValue::Float(f)) => Some(*f),
            _ => None,
        }
    }

    /// Type-safe getter for boolean properties.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.data.get(key) {
            Some(PropertyValue::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    /// Type-safe getter for string list properties.
    pub fn get_string_list(&self, key: &str) -> Option<&[String]> {
        match self.data.get(key) {
            Some(PropertyValue::StringList(list)) => Some(list),
            _ => None,
        }
    }

    /// Type-safe getter for integer list properties.
    pub fn get_int_list(&self, key: &str) -> Option<&[i64]> {
        match self.data.get(key) {
            Some(PropertyValue::IntList(list)) => Some(list),
            _ => None,
        }
    }
}

impl FromIterator<(String, PropertyValue)> for PropertyMap {
    fn from_iter<T: IntoIterator<Item = (String, PropertyValue)>>(iter: T) -> Self {
        Self {
            data: HashMap::from_iter(iter),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_value_conversions() {
        let s: PropertyValue = "test".into();
        assert!(matches!(s, PropertyValue::String(_)));

        let i: PropertyValue = 42i64.into();
        assert!(matches!(i, PropertyValue::Int(42)));

        let f: PropertyValue = 3.14.into();
        assert!(matches!(f, PropertyValue::Float(_)));

        let b: PropertyValue = true.into();
        assert!(matches!(b, PropertyValue::Bool(true)));
    }

    #[test]
    fn test_property_map_builder() {
        let props = PropertyMap::new()
            .with("name", "test_function")
            .with("line", 42i64)
            .with("is_async", true);

        assert_eq!(props.get_string("name"), Some("test_function"));
        assert_eq!(props.get_int("line"), Some(42));
        assert_eq!(props.get_bool("is_async"), Some(true));
    }

    #[test]
    fn test_property_map_insert() {
        let mut props = PropertyMap::new();
        props.insert("key1", "value1");
        props.insert("key2", 123i64);

        assert_eq!(props.get_string("key1"), Some("value1"));
        assert_eq!(props.get_int("key2"), Some(123));
    }

    #[test]
    fn test_property_map_type_safety() {
        let props = PropertyMap::new()
            .with("name", "function")
            .with("line", 10i64);

        // Wrong type returns None
        assert_eq!(props.get_int("name"), None);
        assert_eq!(props.get_string("line"), None);
    }

    #[test]
    fn test_property_map_remove() {
        let mut props = PropertyMap::new().with("temp", "value");
        assert!(props.contains_key("temp"));

        let removed = props.remove("temp");
        assert!(matches!(removed, Some(PropertyValue::String(_))));
        assert!(!props.contains_key("temp"));
    }

    #[test]
    fn test_property_map_lists() {
        let props = PropertyMap::new()
            .with("symbols", vec!["foo".to_string(), "bar".to_string()])
            .with("lines", vec![1i64, 2i64, 3i64]);

        assert_eq!(props.get_string_list("symbols").map(|s| s.len()), Some(2));
        assert_eq!(props.get_int_list("lines").map(|l| l.len()), Some(3));
    }
}
