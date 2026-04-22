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

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::GET => write!(f, "GET"),
            HttpMethod::POST => write!(f, "POST"),
            HttpMethod::PUT => write!(f, "PUT"),
            HttpMethod::DELETE => write!(f, "DELETE"),
            HttpMethod::PATCH => write!(f, "PATCH"),
        }
    }
}

impl HttpMethod {
    pub fn from_method(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(HttpMethod::GET),
            "POST" => Some(HttpMethod::POST),
            "PUT" => Some(HttpMethod::PUT),
            "DELETE" => Some(HttpMethod::DELETE),
            "PATCH" => Some(HttpMethod::PATCH),
            _ => None,
        }
    }

    pub fn all() -> Vec<HttpMethod> {
        vec![
            HttpMethod::GET,
            HttpMethod::POST,
            HttpMethod::PUT,
            HttpMethod::DELETE,
            HttpMethod::PATCH,
        ]
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
        let mut router = Router {
            routes: HashMap::new(),
            pattern_routes: Vec::new(),
            error_route: None,
        };

        // Check for new folder-based routing in src/
        let src_dir = project_root.join("src");
        if src_dir.exists() {
            discover_folder_routes(&src_dir, &mut router, "")?;
        } else {
            // Fall back to legacy routes/ directory
            let routes_dir = project_root.join("routes");
            if routes_dir.exists() {
                for entry in walkdir(&routes_dir)? {
                    let entry_path = entry.path();
                    if entry_path.extension().and_then(|s| s.to_str()) == Some("bv") {
                        if let Some(route) = parse_legacy_route_file(&entry_path) {
                            let path_key = (route.method.clone(), route.path.clone());
                            if route.path.contains(':') {
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
            }
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

fn discover_folder_routes(
    dir: &Path,
    router: &mut Router,
    parent_path: &str,
) -> Result<(), RouteDiscoveryError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip private folders (starting with underscore)
        if file_name.starts_with('_') {
            continue;
        }

        // Check if it's a route group (wrapped in parentheses)
        let is_route_group = file_name.starts_with('(') && file_name.ends_with(')');

        if path.is_dir() {
            let folder_name = if is_route_group {
                // Route group: strip parentheses but don't include in URL
                &file_name[1..file_name.len() - 1]
            } else {
                file_name
            };

            // For route groups, use the parent path unchanged
            // For regular folders, add the folder name to the path
            let new_path = if is_route_group {
                parent_path.to_string()
            } else {
                let segment = if folder_name.starts_with('[') && folder_name.ends_with(']') {
                    // Dynamic segment: [id] -> :id
                    format!(":{}", &folder_name[1..folder_name.len() - 1])
                } else {
                    folder_name.to_string()
                };
                if parent_path.is_empty() {
                    format!("/{}", segment)
                } else {
                    format!("{}/{}", parent_path, segment)
                }
            };

            // Recursively discover routes in subfolder
            discover_folder_routes(&path, router, &new_path)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("rbv") {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

            if stem == "page" || stem == "index" {
                // page.rbv or index.rbv = GET only
                let route = Route {
                    method: HttpMethod::GET,
                    path: if parent_path.is_empty() {
                        "/".to_string()
                    } else {
                        parent_path.to_string()
                    },
                    file_path: path.clone(),
                    handler_name: "handle".to_string(),
                };
                let path_key = (route.method.clone(), route.path.clone());
                // Check if path contains dynamic segments
                if route.path.contains(':') {
                    router.pattern_routes.push(route);
                } else {
                    router.routes.insert(path_key, route);
                }
            } else if stem == "route" {
                // route.rbv = all methods
                for method in HttpMethod::all() {
                    let route = Route {
                        method: method.clone(),
                        path: if parent_path.is_empty() {
                            "/".to_string()
                        } else {
                            parent_path.to_string()
                        },
                        file_path: path.clone(),
                        handler_name: format!("handle_{}", method.to_string().to_lowercase()),
                    };
                    let path_key = (route.method.clone(), route.path.clone());
                    if route.path.contains(':') {
                        router.pattern_routes.push(route);
                    } else {
                        router.routes.insert(path_key, route);
                    }
                }
            } else if stem == "layout" {
                // layout.rbv = layout file, not a route
            }
        }
    }

    Ok(())
}

fn parse_legacy_route_file(file_path: &Path) -> Option<Route> {
    parse_route_file(file_path)
}

fn parse_route_file(file_path: &Path) -> Option<Route> {
    let filename = file_path.file_stem()?.to_str()?;

    let method = filename.split('.').next()?;
    let method = HttpMethod::from_method(method)?;

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
        if let Some(stripped) = pattern_part.strip_prefix(':') {
            params.insert(stripped.to_string(), path_part.to_string());
        } else if *pattern_part != *path_part {
            return None;
        }
    }

    Some(params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

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

    #[test]
    fn test_folder_page_rbv_creates_get_route() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(
            temp.path().join("src/page.rbv"),
            "txn handle [true][true] { term; };",
        )
        .unwrap();

        let router = Router::discover_routes(temp.path()).unwrap();
        let routes: Vec<_> = router.routes().collect();

        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].method, HttpMethod::GET);
        assert_eq!(routes[0].path, "/");
    }

    #[test]
    fn test_folder_users_page_rbv() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("src/users")).unwrap();
        fs::write(
            temp.path().join("src/users/page.rbv"),
            "txn handle [true][true] { term; };",
        )
        .unwrap();

        let router = Router::discover_routes(temp.path()).unwrap();
        let routes: Vec<_> = router.routes().collect();

        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].method, HttpMethod::GET);
        assert_eq!(routes[0].path, "/users");
    }

    #[test]
    fn test_folder_route_rbv_creates_all_methods() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("src/api")).unwrap();
        fs::write(
            temp.path().join("src/api/route.rbv"),
            "txn handle [true][true] { term; };",
        )
        .unwrap();

        let router = Router::discover_routes(temp.path()).unwrap();
        let routes: Vec<_> = router.routes().collect();

        assert_eq!(routes.len(), 5); // GET, POST, PUT, DELETE, PATCH
        let methods: Vec<_> = routes.iter().map(|r| r.method.clone()).collect();
        assert!(methods.contains(&HttpMethod::GET));
        assert!(methods.contains(&HttpMethod::POST));
        assert!(methods.contains(&HttpMethod::PUT));
        assert!(methods.contains(&HttpMethod::DELETE));
        assert!(methods.contains(&HttpMethod::PATCH));
    }

    #[test]
    fn test_dynamic_segment_in_page() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("src/users/[id]")).unwrap();
        fs::write(
            temp.path().join("src/users/[id]/page.rbv"),
            "txn handle [true][true] { term; };",
        )
        .unwrap();

        let router = Router::discover_routes(temp.path()).unwrap();
        let routes: Vec<_> = router.routes().collect();

        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].path, "/users/:id");

        let found = router.find_route(&HttpMethod::GET, "/users/123");
        assert!(found.is_some());
        assert_eq!(found.unwrap().params.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_route_group_excluded_from_url() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("src/(admin)")).unwrap();
        fs::write(
            temp.path().join("src/(admin)/page.rbv"),
            "txn handle [true][true] { term; };",
        )
        .unwrap();

        let router = Router::discover_routes(temp.path()).unwrap();
        let routes: Vec<_> = router.routes().collect();

        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].path, "/");
    }

    #[test]
    fn test_private_folder_ignored() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("src/_internal")).unwrap();
        fs::write(
            temp.path().join("src/_internal/page.rbv"),
            "txn handle [true][true] { term; };",
        )
        .unwrap();

        let router = Router::discover_routes(temp.path()).unwrap();
        let routes: Vec<_> = router.routes().collect();

        assert_eq!(routes.len(), 0);
    }

    #[test]
    fn test_legacy_routes_fallback() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("routes")).unwrap();
        fs::write(
            temp.path().join("routes/GET.legacy.bv"),
            "txn handle [true][true] { term; };",
        )
        .unwrap();

        let router = Router::discover_routes(temp.path()).unwrap();
        let routes: Vec<_> = router.routes().collect();

        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].path, "/legacy");
    }

    #[test]
    fn test_src_takes_precedence_over_routes() {
        let temp = TempDir::new().unwrap();

        // Create src/ directory
        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(
            temp.path().join("src/page.rbv"),
            "txn handle [true][true] { term \"src\"; };",
        )
        .unwrap();

        // Create routes/ directory (legacy)
        fs::create_dir_all(temp.path().join("routes")).unwrap();
        fs::write(
            temp.path().join("routes/GET.legacy.bv"),
            "txn handle [true][true] { term \"routes\"; };",
        )
        .unwrap();

        let router = Router::discover_routes(temp.path()).unwrap();
        let routes: Vec<_> = router.routes().collect();

        // src/ should take precedence, so we should only get GET /, not /legacy
        assert!(routes.iter().any(|r| r.path == "/"));
        assert!(!routes.iter().any(|r| r.path == "/legacy"));
    }

    #[test]
    fn test_nested_dynamic_segments() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("src/users/[userId]/posts/[postId]")).unwrap();
        fs::write(
            temp.path()
                .join("src/users/[userId]/posts/[postId]/page.rbv"),
            "txn handle [true][true] { term; };",
        )
        .unwrap();

        let router = Router::discover_routes(temp.path()).unwrap();
        let routes: Vec<_> = router.routes().collect();

        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].path, "/users/:userId/posts/:postId");

        let found = router.find_route(&HttpMethod::GET, "/users/abc/posts/123");
        assert!(found.is_some());
        let params = found.unwrap().params;
        assert_eq!(params.get("userId"), Some(&"abc".to_string()));
        assert_eq!(params.get("postId"), Some(&"123".to_string()));
    }

    #[test]
    fn test_find_route_exact_match() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("src/users")).unwrap();
        fs::write(
            temp.path().join("src/users/page.rbv"),
            "txn handle [true][true] { term; };",
        )
        .unwrap();

        let router = Router::discover_routes(temp.path()).unwrap();

        let found = router.find_route(&HttpMethod::GET, "/users");
        assert!(found.is_some());
        assert_eq!(found.unwrap().params.len(), 0);
    }

    #[test]
    fn test_find_route_not_found() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("src/users")).unwrap();
        fs::write(
            temp.path().join("src/users/page.rbv"),
            "txn handle [true][true] { term; };",
        )
        .unwrap();

        let router = Router::discover_routes(temp.path()).unwrap();

        let found = router.find_route(&HttpMethod::GET, "/nonexistent");
        assert!(found.is_none());
    }

    #[test]
    fn test_layout_file_not_a_route() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(
            temp.path().join("src/layout.rbv"),
            "txn handle [true][true] { term; };",
        )
        .unwrap();
        fs::write(
            temp.path().join("src/page.rbv"),
            "txn handle [true][true] { term; };",
        )
        .unwrap();

        let router = Router::discover_routes(temp.path()).unwrap();
        let routes: Vec<_> = router.routes().collect();

        // Should only have the page.rbv route, not layout.rbv
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].path, "/");
    }

    #[test]
    fn test_empty_src_directory() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("src")).unwrap();
        // No files in src/

        let router = Router::discover_routes(temp.path()).unwrap();
        let routes: Vec<_> = router.routes().collect();

        assert_eq!(routes.len(), 0);
    }
}
