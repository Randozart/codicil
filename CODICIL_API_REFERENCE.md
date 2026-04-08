# Codicil Framework API Reference

This document provides detailed API reference for Codicil framework components, built-in types, and standard library functions.

---

## Table of Contents

1. [Request/Response Objects](#requestresponse-objects)
2. [Route Decorators & Headers](#route-decorators--headers)
3. [Built-in Types](#built-in-types)
4. [Middleware API](#middleware-api)
5. [Standard Library Functions](#standard-library-functions)
6. [Error Types](#error-types)
7. [Component API](#component-api)

---

## Request/Response Objects

### RequestContext

Implicit parameter available to all route handlers.

```brief
struct RequestContext {
  method: String,                    # HTTP method: "GET", "POST", "PUT", "DELETE", "PATCH"
  path: String,                      # Full request path: "/users/123/posts?page=1"
  params: {[String]: String},        # Dynamic path segments
  query: {[String]: String},         # Query string parameters
  body: String,                      # Raw request body
  headers: {[String]: String},       # HTTP request headers (lowercase keys)
  user: Any?,                        # User object from auth middleware (optional)
  session: Any?,                     # Session object from session middleware (optional)
};
```

**Accessing request data:**

```brief
txn handle [true][response.status >= 200] {
  # Path parameters
  let id = context.params.{"id"};
  
  # Query parameters
  let page = context.query.{"page"};
  
  # Headers
  let auth_header = context.headers.{"authorization"};
  let content_type = context.headers.{"content-type"};
  
  # Request body (must be parsed manually)
  let body_json = parse_json(context.body);
  
  # From middleware
  let current_user = context.user;
  
  term &response { ... };
};
```

### Response

Structure returned from all route handlers.

```brief
struct Response {
  status: Int,                       # HTTP status code (200, 404, 500, etc.)
  headers: {[String]: String},       # Response headers
  body: String,                      # Response body (must be serialized string)
};
```

**Common response patterns:**

```brief
# JSON response
term &response {
  status: 200,
  headers: { "content-type": "application/json" },
  body: to_json(user_object)
};

# HTML response
term &response {
  status: 200,
  headers: { "content-type": "text/html; charset=utf-8" },
  body: render_to_html(component)
};

# Redirect
term &response {
  status: 302,
  headers: { "location": "/new-path" },
  body: ""
};

# Error response
term &response {
  status: 500,
  headers: { "content-type": "application/json" },
  body: to_json({ error: "Internal server error" })
};
```

---

## Route Decorators & Headers

Every route file starts with a `[route]` header block in TOML.

### Method Options

| Method | Usage |
|--------|-------|
| `GET` | Retrieve resource |
| `POST` | Create resource |
| `PUT` | Replace resource |
| `PATCH` | Partial update |
| `DELETE` | Remove resource |
| `OPTIONS` | CORS preflight |
| `HEAD` | Like GET, no body |

### Context Options

| Context | Behavior |
|---------|----------|
| `"server"` | Runs on server, has DB access, cannot import client code |
| `"client"` | Runs in browser, no DB access, renders to JavaScript |

---

## Built-in Types

### Basic Types

| Type | Example | Usage |
|------|---------|-------|
| `Int` | `42` | Integers |
| `Float` | `3.14` | Decimals |
| `String` | `"hello"` | Text |
| `Bool` | `true`, `false` | True/false |
| `Void` | `void` | No return value |

### Collection Types

#### Lists (Homogeneous)

```brief
struct Response {
  items: [User],    # List of User structs
  count: Int,
};
```

#### Maps (Key-Value)

```brief
struct RequestHeaders {
  headers: {[String]: String},  # String key, String value
};
```

#### Unions (Multiple Types)

```brief
union Result {
  Success { value: Int },
  Failure { error: String },
};
```

#### Optionals

```brief
struct User {
  id: Int,
  email: String,
  phone: String?,        # Optional: may be null
  bio: String?,          # Optional
};
```

---

## Middleware API

### Middleware Structure

Middleware modules are Brief files in `middleware/` that define transactions:

```brief
# middleware/custom.bv

struct CustomContext {
  request_id: String,
};

txn apply_custom_middleware [true]
                            [result == applied(context) || result == rejected] {
  let request_id = generate_uuid();
  let context = CustomContext { request_id: request_id };
  
  term &applied(context);
};
```

### Middleware Result Types

```brief
union MiddlewareResult {
  Approved { context: Any },
  Rejected { reason: String },
};

union AuthResult {
  Authenticated { user: User },
  Unauthenticated,
};
```

### Built-in Middleware

| Middleware | Purpose |
|-----------|---------|
| `cors` | Cross-origin requests, adds headers |
| `rate_limit` | Request throttling |
| `session` | Cookie-based sessions, provides `context.session` |
| `auth` | JWT authentication, provides `context.user` |
| `logging` | Request/response logging |

---

## Standard Library Functions

### Type Conversion

```brief
to_string(value: Any) -> String
parse_int(value: String) -> Int?
parse_float(value: String) -> Float?
```

### String Functions

```brief
length(str: String) -> Int
contains(str: String, substr: String) -> Bool
starts_with(str: String, prefix: String) -> Bool
ends_with(str: String, suffix: String) -> Bool
trim(str: String) -> String
to_lowercase(str: String) -> String
to_uppercase(str: String) -> String
```

### Serialization (FFI-based)

```brief
frgn sig to_json(obj: Any) -> String;
frgn sig from_json(json: String, type: Any) -> Any;
frgn sig parse_form(body: String) -> Any;
```

### Authentication (FFI-based)

```brief
frgn sig jwt_encode(payload: Any, secret: String) -> String;
frgn sig jwt_decode(token: String, secret: String) -> Any?;
frgn sig hash_password(password: String) -> String;
frgn sig verify_password(password: String, hash: String) -> Bool;
```

### Database (FFI-based)

FFI signatures declared in FFI bindings and wrapped in Brief.

### Utilities

```brief
frgn sig generate_uuid() -> String;
frgn sig now() -> String;              # ISO 8601 timestamp
frgn sig timestamp() -> Int;           # Unix timestamp
frgn sig http_get(url: String) -> String;
frgn sig http_post(url: String, body: String) -> String;
frgn sig log(level: String, message: String) -> Void;
```

---

## Error Types

### Standard Error Union

```brief
union ApiError {
  NotFound { resource: String },
  Unauthorized { reason: String },
  BadRequest { message: String },
  Conflict { message: String },
  ServerError { details: String },
};
```

### Error Response Structure

```brief
struct ErrorResponse {
  status: Int,
  code: String,
  message: String,
  details: String?,
};
```

### HTTP Status Codes

| Code | Meaning |
|------|---------|
| `200` | OK |
| `201` | Created |
| `204` | No Content |
| `400` | Bad Request |
| `401` | Unauthorized |
| `403` | Forbidden |
| `404` | Not Found |
| `409` | Conflict |
| `422` | Unprocessable |
| `429` | Too Many Requests |
| `500` | Server Error |
| `503` | Unavailable |

---

## Component API

### Server-Side Component

```brief
[component]
context = "server"

struct Props {
  user: User,
};

txn render_user_profile [user.id exists]
                        [html contains user.name] {
  # Server component: can access any data
  let user_full = db::get_full_user_profile(user.id);
  
  # Render to HTML string for SSR
  term &html { ... };
};
```

### Client-Side Component

```brief
[component]
context = "client"

struct Props {
  post: Post,
};

struct LocalState {
  liked: Bool,
};

txn render_post [post.id exists]
                [html contains post.title] {
  let state = LocalState { liked: false };
  
  term &html {
    <div class="post">
      <h2 b-text="post.title">Post</h2>
      <button b-trigger:click="toggle_like">Like</button>
    </div>
  };
};

txn toggle_like [true][state.liked != old_liked] {
  state.liked = !state.liked;
  term;
};
```

### Component Events

Built-in event directives (client-side only):

```html
<!-- Click events -->
<button b-trigger:click="handler_name">Click me</button>

<!-- Form events -->
<form b-trigger:submit="handle_submit">
  <input type="text" b-model="state.field" />
</form>

<!-- Visibility binding -->
<div b-show="condition">Show if true</div>

<!-- Text binding -->
<span b-text="variable">Default text</span>

<!-- Class binding -->
<div b-class:active="is_active">Element</div>

<!-- Model binding (two-way) -->
<input type="text" b-model="state.field" />
```

---

## Common Patterns

### CRUD Operations

```brief
txn create_user [name exists && email exists]
                [result.id > 0] { ... };

txn get_user [id > 0]
             [result.id == id] { ... };

txn update_user [id > 0 && data exists]
                [result.id == id] { ... };

txn delete_user [id > 0]
                [result == deleted] { ... };
```

### Request Validation

```brief
txn validate_request [context.body exists]
                     [result == valid || result == invalid] {
  let data = parse_json(context.body);
  
  let has_email = data.email exists && data.email.length > 0;
  let has_password = data.password exists && data.password.length >= 8;
  
  match (has_email, has_password) {
    (true, true) => term &valid;
    _ => term &invalid;
  };
};
```

### Authorization Check

```brief
txn check_permission [context.user exists]
                     [result == allowed || result == denied] {
  let user_id = context.user.id;
  let resource_owner = db::get_resource_owner(resource_id);
  
  match user_id == resource_owner {
    true => term &allowed;
    false => term &denied;
  };
};
```

---

## Summary

The Codicil API consists of:

- **Request/Response** — Structured types for HTTP lifecycle
- **Routes** — TOML-declared transactions with contracts
- **Middleware** — Composable transaction pipelines
- **Standard Library** — Type conversion, serialization, auth, DB access
- **Components** — Server-side and client-side view composition
- **Error Handling** — Union types for structured error propagation

All APIs prioritize **type safety**, **explicit contracts**, and **compile-time verification**.
