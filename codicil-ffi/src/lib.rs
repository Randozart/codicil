use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{Column, Row};
use std::collections::HashMap;
use std::env;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub fn json_parse(s: &str) -> Result<JsonValue, String> {
    JsonValue::from_json(s)
}

pub fn json_stringify(data: &serde_json::Value) -> Result<String, String> {
    serde_json::to_string(data).map_err(|e| format!("JSON stringify error: {}", e))
}

pub async fn http_get_async(url: &str) -> Result<HttpResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("HTTP GET failed: {}", e))?;

    let status = response.status().as_u16();
    let mut headers = HashMap::new();
    for (name, value) in response.headers() {
        if let Ok(v) = value.to_str() {
            headers.insert(name.to_string(), v.to_string());
        }
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    Ok(HttpResponse {
        status,
        headers,
        body,
    })
}

pub fn http_get(url: &str) -> Result<HttpResponse, String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(http_get_async(url))
}

pub async fn http_post_async(url: &str, body: &str) -> Result<HttpResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(url)
        .body(body.to_string())
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("HTTP POST failed: {}", e))?;

    let status = response.status().as_u16();
    let mut headers = HashMap::new();
    for (name, value) in response.headers() {
        if let Ok(v) = value.to_str() {
            headers.insert(name.to_string(), v.to_string());
        }
    }

    let resp_body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    Ok(HttpResponse {
        status,
        headers,
        body: resp_body,
    })
}

pub fn http_post(url: &str, body: &str) -> Result<HttpResponse, String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(http_post_async(url, body))
}

pub async fn db_query(
    query: &str,
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let database_url = env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL environment variable not set".to_string())?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .map_err(|e| format!("Failed to connect to database: {}", e))?;

    let params_array = params
        .as_array()
        .ok_or("db_query params must be an array")?;

    let rows: Vec<PgRow> = match params_array.len() {
        0 => sqlx::query(query)
            .fetch_all(&pool)
            .await
            .map_err(|e| e.to_string())?,
        1 => {
            let p0 = params_array[0].clone();
            sqlx::query(query)
                .bind(p0)
                .fetch_all(&pool)
                .await
                .map_err(|e| e.to_string())?
        }
        2 => {
            let p0 = params_array[0].clone();
            let p1 = params_array[1].clone();
            sqlx::query(query)
                .bind(p0)
                .bind(p1)
                .fetch_all(&pool)
                .await
                .map_err(|e| e.to_string())?
        }
        3 => {
            let p0 = params_array[0].clone();
            let p1 = params_array[1].clone();
            let p2 = params_array[2].clone();
            sqlx::query(query)
                .bind(p0)
                .bind(p1)
                .bind(p2)
                .fetch_all(&pool)
                .await
                .map_err(|e| e.to_string())?
        }
        _ => {
            let mut q = sqlx::query(query);
            for p in params_array {
                q = q.bind(p.clone());
            }
            q.fetch_all(&pool).await.map_err(|e| e.to_string())?
        }
    };

    pool.close().await;

    let json_rows: Vec<serde_json::Value> = rows
        .iter()
        .map(|row: &PgRow| {
            let mut map = serde_json::Map::new();
            for i in 0..row.len() {
                let col = row.columns().get(i).unwrap();
                let name = col.name();
                let value: serde_json::Value = match row.try_get::<String, _>(i) {
                    Ok(v) => serde_json::Value::String(v),
                    Err(_) => match row.try_get::<i64, _>(i) {
                        Ok(v) => serde_json::Value::Number(v.into()),
                        Err(_) => match row.try_get::<f64, _>(i) {
                            Ok(v) => serde_json::json!(v),
                            Err(_) => serde_json::Value::Null,
                        },
                    },
                };
                map.insert(name.to_string(), value);
            }
            serde_json::Value::Object(map)
        })
        .collect();

    Ok(serde_json::Value::Array(json_rows))
}

pub fn db_query_blocking(
    query: &str,
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(db_query(query, params))
}
