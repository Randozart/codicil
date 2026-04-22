use std::fs;
use tempfile::TempDir;

#[test]
fn test_init_creates_src_directory() {
    use crate::cmd_init;

    let temp = TempDir::new().unwrap();
    let project_path = temp.path().join("test-project");

    cmd_init(project_path.to_str().unwrap(), true).unwrap();

    assert!(project_path.join("src").exists());
    assert!(project_path.join("src/page.rbv").exists());
}

#[test]
fn test_init_creates_codicil_config() {
    use crate::cmd_init;

    let temp = TempDir::new().unwrap();
    let project_path = temp.path().join("test-project");

    cmd_init(project_path.to_str().unwrap(), true).unwrap();

    assert!(project_path.join(".codicil").exists());
    assert!(project_path.join(".codicil/config.toml").exists());
}

#[test]
fn test_init_creates_page_rbv() {
    use crate::cmd_init;

    let temp = TempDir::new().unwrap();
    let project_path = temp.path().join("test-project");

    cmd_init(project_path.to_str().unwrap(), true).unwrap();

    let content = fs::read_to_string(project_path.join("src/page.rbv")).unwrap();
    assert!(content.contains("txn handle"));
}

#[test]
fn test_init_fails_if_directory_exists() {
    use crate::cmd_init;

    let temp = TempDir::new().unwrap();
    let project_path = temp.path().join("test-project");
    fs::create_dir(&project_path).unwrap();

    let result = cmd_init(project_path.to_str().unwrap(), true);
    assert!(result.is_err());
}

#[test]
fn test_dev_server_page_rbv_returns_200() {
    use crate::cmd_init;
    use codicil_core::{HttpMethod, Router};

    let temp = TempDir::new().unwrap();
    let project_path = temp.path().join("test-project");

    cmd_init(project_path.to_str().unwrap(), true).unwrap();

    fs::write(
        project_path.join("src/page.rbv"),
        r#"
txn handle [true][true] {
    term "Hello, World!";
};
"#,
    )
    .unwrap();

    let router = Router::discover_routes(&project_path).unwrap();
    let routes: Vec<_> = router.routes().collect();

    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].path, "/");
    assert_eq!(routes[0].method, HttpMethod::GET);
}

#[test]
fn test_dev_server_route_rbv_all_methods() {
    use crate::cmd_init;
    use codicil_core::{HttpMethod, Router};

    let temp = TempDir::new().unwrap();
    let project_path = temp.path().join("test-project");

    cmd_init(project_path.to_str().unwrap(), true).unwrap();

    // Remove default page.rbv to test only our route
    fs::remove_file(project_path.join("src/page.rbv")).unwrap();

    fs::create_dir_all(project_path.join("src/api")).unwrap();
    fs::write(
        project_path.join("src/api/route.rbv"),
        r#"
txn handle [true][true] {
    term "API response";
};
"#,
    )
    .unwrap();

    let router = Router::discover_routes(&project_path).unwrap();
    let routes: Vec<_> = router.routes().collect();

    // route.rbv creates 5 routes (GET, POST, PUT, DELETE, PATCH)
    assert_eq!(routes.len(), 5);

    let paths: Vec<_> = routes.iter().map(|r| r.path.clone()).collect();
    assert!(paths.contains(&"/api".to_string()));

    let methods: Vec<_> = routes.iter().map(|r| r.method.clone()).collect();
    assert!(methods.contains(&HttpMethod::GET));
    assert!(methods.contains(&HttpMethod::POST));
    assert!(methods.contains(&HttpMethod::PUT));
    assert!(methods.contains(&HttpMethod::DELETE));
}

#[test]
fn test_dev_server_dynamic_segment() {
    use crate::cmd_init;
    use codicil_core::{HttpMethod, Router};

    let temp = TempDir::new().unwrap();
    let project_path = temp.path().join("test-project");

    cmd_init(project_path.to_str().unwrap(), true).unwrap();

    fs::create_dir_all(project_path.join("src/users/[id]")).unwrap();
    fs::write(
        project_path.join("src/users/[id]/page.rbv"),
        r#"
txn handle [true][true] {
    term "User";
};
"#,
    )
    .unwrap();

    let router = Router::discover_routes(&project_path).unwrap();

    let found = router.find_route(&HttpMethod::GET, "/users/123");
    assert!(found.is_some());
    assert_eq!(found.unwrap().params.get("id"), Some(&"123".to_string()));
}

#[test]
fn test_dev_server_nested_routes() {
    use crate::cmd_init;
    use codicil_core::{HttpMethod, Router};

    let temp = TempDir::new().unwrap();
    let project_path = temp.path().join("test-project");

    cmd_init(project_path.to_str().unwrap(), true).unwrap();

    // Remove default page.rbv to test only our route
    fs::remove_file(project_path.join("src/page.rbv")).unwrap();

    fs::create_dir_all(project_path.join("src/users/[userId]/posts")).unwrap();
    fs::write(
        project_path.join("src/users/[userId]/posts/page.rbv"),
        r#"
txn handle [true][true] {
    term "posts";
};
"#,
    )
    .unwrap();

    let router = Router::discover_routes(&project_path).unwrap();
    let routes: Vec<_> = router.routes().collect();

    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].path, "/users/:userId/posts");

    let found = router.find_route(&HttpMethod::GET, "/users/abc/posts");
    assert!(found.is_some());
    assert_eq!(
        found.unwrap().params.get("userId"),
        Some(&"abc".to_string())
    );
}

#[test]
fn test_dev_server_route_group() {
    use crate::cmd_init;
    use codicil_core::{HttpMethod, Router};

    let temp = TempDir::new().unwrap();
    let project_path = temp.path().join("test-project");

    cmd_init(project_path.to_str().unwrap(), true).unwrap();

    fs::create_dir_all(project_path.join("src/(admin)")).unwrap();
    fs::write(
        project_path.join("src/(admin)/page.rbv"),
        r#"
txn handle [true][true] {
    term "admin";
};
"#,
    )
    .unwrap();

    let router = Router::discover_routes(&project_path).unwrap();
    let routes: Vec<_> = router.routes().collect();

    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].path, "/");
}

#[test]
fn test_dev_server_404_for_unknown_path() {
    use crate::cmd_init;
    use codicil_core::{HttpMethod, Router};

    let temp = TempDir::new().unwrap();
    let project_path = temp.path().join("test-project");

    cmd_init(project_path.to_str().unwrap(), true).unwrap();

    fs::write(
        project_path.join("src/page.rbv"),
        r#"
txn handle [true][true] {
    term "home";
};
"#,
    )
    .unwrap();

    let router = Router::discover_routes(&project_path).unwrap();

    let found = router.find_route(&HttpMethod::GET, "/nonexistent");
    assert!(found.is_none());
}
