use std::collections::HashMap;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RouteDiscoveryError {
    #[error("Failed to read directory: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid route filename: {0}")]
    InvalidFilename(String),
}

#[derive(Debug, Clone)]
pub struct Route {
    pub method: HttpMethod,
    pub path: String,
    pub file_path: PathBuf,
    pub handler_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

impl HttpMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(HttpMethod::GET),
            "POST" => Some(HttpMethod::POST),
            "PUT" => Some(HttpMethod::PUT),
            "DELETE" => Some(HttpMethod::DELETE),
            "PATCH" => Some(HttpMethod::PATCH),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct RouteMatch<'a> {
    pub route: &'a Route,
    pub params: HashMap<String, String>,
}

pub struct Router {
    routes: HashMap<(HttpMethod, String), Route>,
    pattern_routes: Vec<Route>,
    error_route: Option<PathBuf>,
}

impl Router {
    pub fn discover_routes(project_root: &Path) -> Result<Self, RouteDiscoveryError> {
        let routes_dir = project_root.join("routes");
        if !routes_dir.exists() {
            return Ok(Router {
                routes: HashMap::new(),
                pattern_routes: Vec::new(),
                error_route: None,
            });
        }

        let mut router = Router {
            routes: HashMap::new(),
            pattern_routes: Vec::new(),
            error_route: None,
        };

        for entry in walkdir(&routes_dir)? {
            let entry_path = entry.path();
            if entry_path.extension().and_then(|s| s.to_str()) == Some("bv") {
                if let Some(route) = parse_route_file(&entry_path) {
                    let path_key = (route.method.clone(), route.path.clone());
                    if route.path.contains('[') {
                        router.pattern_routes.push(route);
                    } else {
                        router.routes.insert(path_key, route);
                    }
                }
            } else if entry_path.extension().and_then(|s| s.to_str()) == Some("bv")
                && entry_path.file_stem().and_then(|s| s.to_str()) == Some("[error]")
            {
                router.error_route = Some(entry_path);
            }
        }

        let error_path = routes_dir.join("[error].bv");
        if error_path.exists() {
            router.error_route = Some(error_path);
        }

        Ok(router)
    }

    pub fn error_route(&self) -> Option<&PathBuf> {
        self.error_route.as_ref()
    }

    pub fn find_route(&self, method: &HttpMethod, path: &str) -> Option<RouteMatch<'_>> {
        let key = (method.clone(), path.to_string());
        if let Some(route) = self.routes.get(&key) {
            return Some(RouteMatch {
                route,
                params: HashMap::new(),
            });
        }

        for route in &self.pattern_routes {
            if route.method == *method {
                if let Some(params) = match_path_pattern(&route.path, path) {
                    return Some(RouteMatch { route, params });
                }
            }
        }

        None
    }

    pub fn routes(&self) -> impl Iterator<Item = &Route> {
        self.routes.values().chain(self.pattern_routes.iter())
    }
}

fn walkdir(dir: &Path) -> Result<Vec<std::fs::DirEntry>, RouteDiscoveryError> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            entries.extend(walkdir(&path)?);
        } else {
            entries.push(entry);
        }
    }
    Ok(entries)
}

fn parse_route_file(file_path: &Path) -> Option<Route> {
    let filename = file_path.file_stem()?.to_str()?;

    let method = filename.split('.').next()?;
    let method = HttpMethod::from_str(method)?;

    let route_part = filename
        .strip_prefix("GET.")
        .or_else(|| filename.strip_prefix("POST."))
        .or_else(|| filename.strip_prefix("PUT."))
        .or_else(|| filename.strip_prefix("DELETE."))
        .or_else(|| filename.strip_prefix("PATCH."))?;

    let segments: Vec<&str> = route_part.split('.').collect();
    let mut path_segments: Vec<String> = Vec::new();

    for segment in segments {
        if segment.starts_with('[') && segment.ends_with(']') {
            let inner = &segment[1..segment.len() - 1];
            path_segments.push(format!(":{}", inner));
        } else {
            path_segments.push(segment.to_string());
        }
    }

    let path = if path_segments.len() == 1 && path_segments[0] == "index" {
        "/".to_string()
    } else {
        "/".to_string() + &path_segments.join("/")
    };

    Some(Route {
        method,
        path,
        file_path: file_path.to_path_buf(),
        handler_name: "handle".to_string(),
    })
}

fn match_path_pattern(pattern: &str, path: &str) -> Option<HashMap<String, String>> {
    let pattern_parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if pattern_parts.len() != path_parts.len() {
        return None;
    }

    let mut params = HashMap::new();

    for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
        if pattern_part.starts_with(':') {
            params.insert(pattern_part[1..].to_string(), path_part.to_string());
        } else if *pattern_part != *path_part {
            return None;
        }
    }

    Some(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_get_index() {
        let route = parse_route_file(Path::new("routes/GET.index.bv")).unwrap();
        assert_eq!(route.method, HttpMethod::GET);
        assert_eq!(route.path, "/");
    }

    #[test]
    fn test_parse_get_users() {
        let route = parse_route_file(Path::new("routes/GET.users.bv")).unwrap();
        assert_eq!(route.method, HttpMethod::GET);
        assert_eq!(route.path, "/users");
    }

    #[test]
    fn test_parse_get_users_id() {
        let route = parse_route_file(Path::new("routes/GET.users.[id].bv")).unwrap();
        assert_eq!(route.method, HttpMethod::GET);
        assert_eq!(route.path, "/users/:id");
    }

    #[test]
    fn test_parse_nested() {
        let route =
            parse_route_file(Path::new("routes/GET.users.[id].posts.[post_id].bv")).unwrap();
        assert_eq!(route.method, HttpMethod::GET);
        assert_eq!(route.path, "/users/:id/posts/:post_id");
    }

    #[test]
    fn test_match_path_pattern() {
        let params = match_path_pattern("/users/:id", "/users/123").unwrap();
        assert_eq!(params.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_match_path_pattern_nested() {
        let params = match_path_pattern("/users/:id/posts/:post_id", "/users/1/posts/2").unwrap();
        assert_eq!(params.get("id"), Some(&"1".to_string()));
        assert_eq!(params.get("post_id"), Some(&"2".to_string()));
    }
}
