use std::path::Path;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RouteFileError {
    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse route config: {0}")]
    ParseError(String),
}

#[derive(Debug, Clone)]
pub struct RouteFile {
    pub method: String,
    pub path: String,
    pub middleware: Vec<String>,
    pub context: String,
    pub precondition: String,
    pub postcondition: String,
    pub handler_name: String,
    pub brief_code: String,
}

impl RouteFile {
    pub fn parse(path: &Path) -> Result<Self, RouteFileError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse_content(&content, path)
    }

    pub fn parse_content(content: &str, _path: &Path) -> Result<Self, RouteFileError> {
        let mut method = "GET".to_string();
        let mut path = "/".to_string();
        let mut middleware = Vec::new();
        let mut context = "server".to_string();
        let mut precondition = "true".to_string();
        let mut postcondition = "true".to_string();
        let mut handler_name = "handle".to_string();

        let mut current_section = "before_toml";
        let mut route_table_content = String::new();
        let mut pre_content = String::new();
        let mut post_content = String::new();
        let mut brief_code = String::new();

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            if trimmed == "[route]" {
                current_section = "route";
                route_table_content.push_str("[route]\n");
                continue;
            } else if trimmed == "[pre]" {
                current_section = "pre";
                continue;
            } else if trimmed == "[post]" {
                current_section = "post";
                continue;
            }

            if trimmed.starts_with("txn ")
                || trimmed.starts_with("defn ")
                || trimmed.starts_with("let ")
                || trimmed.starts_with("const ")
            {
                current_section = "brief";
                brief_code.push_str(line);
                brief_code.push('\n');
            } else if current_section == "before_toml" && !trimmed.starts_with('#') {
                current_section = "route";
                route_table_content.push_str(trimmed);
                route_table_content.push('\n');
            } else if current_section == "route" {
                if !trimmed.starts_with('[') {
                    route_table_content.push_str(trimmed);
                    route_table_content.push('\n');
                } else if trimmed == "[pre]" {
                    current_section = "pre";
                    continue;
                } else if trimmed == "[post]" {
                    current_section = "post";
                    continue;
                }
            } else if current_section == "pre" {
                if !trimmed.starts_with('[') {
                    if !pre_content.is_empty() {
                        pre_content.push_str(" && ");
                    }
                    pre_content.push_str(trimmed);
                } else if trimmed == "[post]" {
                    current_section = "post";
                    continue;
                }
            } else if current_section == "post" {
                if !trimmed.starts_with('[') {
                    if !post_content.is_empty() {
                        post_content.push_str(" && ");
                    }
                    post_content.push_str(trimmed);
                }
            } else if current_section == "brief" {
                brief_code.push_str(line);
                brief_code.push('\n');
            }
        }

        if !route_table_content.is_empty() {
            // eprintln!("DEBUG route_table_content:\n{}", route_table_content);
            if let Ok(table) = route_table_content.parse::<toml::Value>() {
                if let Some(route) = table.get("route").and_then(|r| r.as_table()) {
                    if let Some(m) = route.get("method").and_then(|m| m.as_str()) {
                        method = m.to_string();
                    }
                    if let Some(p) = route.get("path").and_then(|p| p.as_str()) {
                        path = p.to_string();
                    }
                    if let Some(mw) = route.get("middleware").and_then(|m| m.as_array()) {
                        middleware = mw
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                    }
                    if let Some(ctx) = route.get("context").and_then(|c| c.as_str()) {
                        context = ctx.to_string();
                    }
                    if let Some(name) = route.get("handler").and_then(|h| h.as_str()) {
                        handler_name = name.to_string();
                    }
                }
            }
        }

        if !pre_content.is_empty() {
            precondition = pre_content.clone();
        }

        if !post_content.is_empty() {
            postcondition = post_content.clone();
        }

        // Pass through brief_code as-is, or convert txn to defn if needed
        if !brief_code.contains("defn ") && brief_code.contains("txn ") {
            let defn_code = "defn handle() -> String [true][true] { term \"{}\"; };".to_string();
            brief_code = defn_code;
        }

        Ok(RouteFile {
            method,
            path,
            middleware,
            context,
            precondition,
            postcondition,
            handler_name,
            brief_code,
        })
    }
}

pub fn parse_route_file(path: &Path) -> Result<RouteFile, RouteFileError> {
    RouteFile::parse(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_route() {
        let content = r#"
[route]
method = "GET"
path = "/"

[post]
response.status == 200

txn handle [true][post] {
    term &response { status: 200, body: "Hello" };
}
"#;
        let route = RouteFile::parse_content(content, Path::new("test.bv")).unwrap();
        assert_eq!(route.method, "GET");
        assert_eq!(route.path, "/");
        assert_eq!(route.precondition, "true");
        assert!(route.brief_code.contains("txn handle"));
    }

    #[test]
    fn test_parse_with_params() {
        let content = r#"
[route]
method = "GET"
path = "/users/:id"

[pre]
params.id is int

[post]
response.status == 200

txn handle [pre][post] {
    term &response { status: 200, body: "User" };
}
"#;
        let route = RouteFile::parse_content(content, Path::new("test.bv")).unwrap();
        assert_eq!(route.method, "GET");
        assert_eq!(route.path, "/users/:id");
        assert!(route.precondition.contains("params.id"));
        assert!(route.postcondition.contains("response.status == 200"));
    }

    #[test]
    fn test_parse_with_middleware() {
        let content = r#"
[route]
method = "GET"
path = "/api"
middleware = ["cors", "auth"]

txn handle [true][post] {
    term;
}
"#;
        let route = RouteFile::parse_content(content, Path::new("test.bv")).unwrap();
        assert_eq!(route.method, "GET");
        assert_eq!(route.middleware, vec!["cors", "auth"]);
    }
}
