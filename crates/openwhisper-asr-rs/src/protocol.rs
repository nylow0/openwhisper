use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct Request {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub timestamp: Option<u64>,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Serialize)]
pub struct Response<'a> {
    #[serde(rename = "type")]
    pub kind: &'a str,
    pub ok: bool,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}
