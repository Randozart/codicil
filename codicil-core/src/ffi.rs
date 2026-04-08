use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonValue {
    #[serde(rename = "type")]
    pub value_type: String,
    pub value: serde_json::Value,
}

impl JsonValue {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|e| format!("JSON parse error: {}", e))?;
        Ok(Self::from_serde_value(value))
    }

    pub fn from_serde_value(value: serde_json::Value) -> Self {
        let value_type = match &value {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "bool",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::String(_) => "string",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
        }
        .to_string();

        Self { value_type, value }
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.value.get(key)?.as_str().map(String::from)
    }

    pub fn get_number(&self, key: &str) -> Option<f64> {
        self.value.get(key)?.as_f64()
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.value.get(key)?.as_bool()
    }

    pub fn get_object(&self, key: &str) -> Option<JsonValue> {
        self.value
            .get(key)
            .map(|v| Self::from_serde_value(v.clone()))
    }
}

pub fn parse_json(json_str: &str) -> Result<JsonValue, String> {
    JsonValue::from_json(json_str)
}

pub fn to_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| format!("JSON serialize error: {}", e))
}

pub fn get_json_string(json: &JsonValue, key: &str) -> Option<String> {
    json.get_string(key)
}

pub fn get_json_number(json: &JsonValue, key: &str) -> Option<f64> {
    json.get_number(key)
}

pub fn get_json_bool(json: &JsonValue, key: &str) -> Option<bool> {
    json.get_bool(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json() {
        let json = r#"{"name": "test", "count": 42}"#;
        let parsed = parse_json(json).unwrap();
        assert_eq!(parsed.get_string("name"), Some("test".to_string()));
        assert_eq!(parsed.get_number("count"), Some(42.0));
    }

    #[test]
    fn test_to_json() {
        let obj = serde_json::json!({"key": "value"});
        let json = to_json(&obj).unwrap();
        assert!(json.contains("key"));
    }
}
