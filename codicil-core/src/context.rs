use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub method: String,
    pub path: String,
    pub params: HashMap<String, String>,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub user: Option<serde_json::Value>,
    pub session: Option<serde_json::Value>,
}

impl RequestContext {
    pub fn new(method: String, path: String) -> Self {
        Self {
            method,
            path,
            params: HashMap::new(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: String::new(),
            user: None,
            session: None,
        }
    }

    pub fn with_params(mut self, params: HashMap<String, String>) -> Self {
        self.params = params;
        self
    }

    pub fn with_query(mut self, query: HashMap<String, String>) -> Self {
        self.query = query;
        self
    }

    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    pub fn with_body(mut self, body: String) -> Self {
        self.body = body;
        self
    }

    pub fn with_user(mut self, user: serde_json::Value) -> Self {
        self.user = Some(user);
        self
    }

    pub fn with_session(mut self, session: serde_json::Value) -> Self {
        self.session = Some(session);
        self
    }
}

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl Response {
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: body.into(),
        }
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn json<T: serde::Serialize>(status: u16, body: &T) -> Result<Self, serde_json::Error> {
        let body_str = serde_json::to_string(body)?;
        Ok(Self {
            status,
            headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
            body: body_str,
        })
    }

    pub fn redirect(url: &str) -> Self {
        Self {
            status: 302,
            headers: HashMap::from([("Location".to_string(), url.to_string())]),
            body: String::new(),
        }
    }
}

use serde::Serialize;
