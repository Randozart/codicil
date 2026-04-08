# Codicil Framework - Examples & Tutorials

This document provides practical examples for building with Codicil.

## Getting Started

### Initialize a New Project

```bash
$ codi init my-blog
$ cd my-blog
$ codi dev
```

### Your First Route

```brief
# routes/GET.index.bv
[route]
method = "GET"
path = "/"

[post]
response.status == 200

txn handle [true][post] {
  term &response {
    status: 200,
    headers: { "content-type": "text/plain" },
    body: "Welcome to Codicil!"
  };
};
```

## Basic Examples

### Simple GET with JSON

```brief
# routes/GET.status.bv
[route]
method = "GET"
path = "/status"

[post]
response.status == 200
response.headers.{"content-type"} == "application/json"

txn get_status [true][post] {
  let status_data = {
    status: "ok",
    timestamp: now(),
    version: "1.0.0"
  };
  
  term &response {
    status: 200,
    headers: { "content-type": "application/json" },
    body: to_json(status_data)
  };
};
```

### GET with Path Parameter

```brief
# routes/GET.greet.[name].bv
[route]
method = "GET"
path = "/greet/:name"

[pre]
params.name exists

[post]
response.status == 200
response.body contains "Hello"

txn greet_user [pre][post] {
  let greeting = "Hello, " + context.params.name + "!";
  
  term &response {
    status: 200,
    headers: { "content-type": "text/plain" },
    body: greeting
  };
};
```

### GET with Query Parameters

```brief
# routes/GET.search.bv
[route]
method = "GET"
path = "/search"

[post]
response.status == 200 || response.status == 400

txn search [true][post] {
  let query = context.query.{"q"};
  
  match query {
    some(q) => {
      let results = search_database(q);
      term &response {
        status: 200,
        body: to_json(results)
      };
    }
    none => {
      term &response {
        status: 400,
        body: to_json({ error: "Missing 'q' parameter" })
      };
    }
  };
};
```

## CRUD Operations

### Setup: Database Module

```brief
# lib/db.bv
frgn sig query_all_posts() -> String;
frgn sig query_post_by_id(id: Int) -> String;
frgn sig insert_post(title: String, content: String) -> String;
frgn sig update_post(id: Int, title: String, content: String) -> String;
frgn sig delete_post(id: Int) -> String;

struct Post {
  id: Int,
  title: String,
  content: String,
  created_at: String,
};

txn find_all_posts [true][result == posts || result == error] {
  let json = query_all_posts();
  let posts = parse_posts_from_json(json);
  term &posts;
};

txn find_post_by_id [id > 0][result == Found || result == NotFound] {
  let json = query_post_by_id(id);
  let post = parse_post_from_json(json);
  
  match post {
    some(p) => term &Found { post: p };
    none => term &NotFound;
  };
};

txn create_post [title.length > 0 && content.length > 0][result.id > 0] {
  let json = insert_post(title, content);
  let post = parse_post_from_json(json);
  term &post;
};

txn update_post [id > 0 && title.length > 0][result.id == id] {
  let json = update_post(id, title, content);
  let post = parse_post_from_json(json);
  term &post;
};

txn delete_post [id > 0][result == deleted || result == not_found] {
  let json = delete_post(id);
  let result = parse_delete_result(json);
  match result {
    success => term &deleted;
    _ => term &not_found;
  };
};
```

### Generate CRUD Routes

```bash
$ codi generate:model Post
```

### List All (GET)

```brief
# routes/GET.posts.bv
import lib/db.bv;

[route]
method = "GET"
path = "/posts"

[post]
response.status == 200

txn list_posts [true][post] {
  let posts = db::find_all_posts();
  term &response {
    status: 200,
    body: to_json(posts)
  };
};
```

### Create (POST)

```brief
# routes/POST.posts.bv
import lib/db.bv;
import lib/forms.bv;

[route]
method = "POST"
path = "/posts"

[pre]
context.body exists

[post]
response.status == 201 || response.status == 400

txn create_post [pre][post] {
  let form_json = parse_form(context.body);
  let validation = validate_post_form(form_json);
  
  match validation {
    valid => {
      let post = db::create_post(form_json.title, form_json.content);
      term &response {
        status: 201,
        body: to_json(post)
      };
    }
    invalid => {
      term &response {
        status: 400,
        body: to_json({ error: "Invalid form data" })
      };
    }
  };
};
```

### Read One (GET :id)

```brief
# routes/GET.posts.[id].bv
import lib/db.bv;

[route]
method = "GET"
path = "/posts/:id"

[pre]
params.id exists

[post]
response.status == 200 || response.status == 404 || response.status == 400

txn get_post [pre][post] {
  let post_id = parse_int(context.params.id);
  
  match post_id {
    some(id) => {
      let result = db::find_post_by_id(id);
      
      match result {
        Found { post: p } => {
          term &response {
            status: 200,
            body: to_json(p)
          };
        }
        NotFound => {
          term &response {
            status: 404,
            body: to_json({ error: "Post not found" })
          };
        }
      };
    }
    none => {
      term &response {
        status: 400,
        body: to_json({ error: "Invalid post ID" })
      };
    }
  };
};
```

### Update (PUT :id) and Delete (DELETE :id)

Similar pattern to above with appropriate method and status codes.

## Authentication

### Auth Module

```brief
# lib/auth.bv
struct User {
  id: Int,
  email: String,
  password_hash: String,
};

union AuthResult {
  Success { user: User, token: String },
  InvalidCredentials,
  UserNotFound,
};

frgn sig find_user_by_email(email: String) -> String;
frgn sig verify_password_hash(password: String, hash: String) -> Bool;
frgn sig jwt_encode(payload: Any, secret: String) -> String;

txn authenticate_user [email.length > 0 && password.length > 0]
                      [result == Success || result == InvalidCredentials] {
  let user_json = find_user_by_email(email);
  let user = parse_user_from_json(user_json);
  
  match user {
    some(u) => {
      let is_valid = verify_password_hash(password, u.password_hash);
      
      match is_valid {
        true => {
          let token = jwt_encode({user_id: u.id, email: u.email}, secret);
          term &Success { user: u, token: token };
        }
        false => {
          term &InvalidCredentials;
        }
      };
    }
    none => {
      term &UserNotFound;
    }
  };
};
```

### Auth Middleware

```brief
# middleware/auth.bv
frgn sig jwt_decode(token: String, secret: String) -> Any?;

struct AuthContext {
  user_id: Int,
  email: String,
};

union AuthResult {
  Authenticated { context: AuthContext },
  Unauthenticated,
};

txn verify_jwt [context.headers.{"authorization"} exists]
               [result == Authenticated || result == Unauthenticated] {
  let auth_header = context.headers.{"authorization"};
  let token = extract_bearer_token(auth_header);
  
  let decoded = jwt_decode(token, jwt_secret);
  
  match decoded {
    some(payload) => {
      let auth_context = AuthContext {
        user_id: payload.user_id,
        email: payload.email
      };
      term &Authenticated { context: auth_context };
    }
    none => {
      term &Unauthenticated;
    }
  };
};
```

### Login Route

```brief
# routes/POST.auth.login.bv
import lib/auth.bv;

[route]
method = "POST"
path = "/auth/login"

[pre]
context.body exists

[post]
response.status == 200 || response.status == 401

txn handle_login [pre][post] {
  let form = parse_login_form(context.body);
  let auth_result = auth::authenticate_user(form.email, form.password);
  
  match auth_result {
    Success { user: u, token: t } => {
      term &response {
        status: 200,
        body: to_json({
          user: { id: u.id, email: u.email },
          token: t
        })
      };
    }
    InvalidCredentials => {
      term &response {
        status: 401,
        body: to_json({ error: "Invalid email or password" })
      };
    }
    UserNotFound => {
      term &response {
        status: 401,
        body: to_json({ error: "User not found" })
      };
    }
  };
};
```

### Protected Route

```brief
# routes/GET.profile.bv
[route]
method = "GET"
path = "/profile"
middleware = ["auth"]

[pre]
context.user exists

[post]
response.status == 200

txn get_user_profile [pre][post] {
  let user_id = context.user.user_id;
  let user_profile = db::get_user_profile(user_id);
  
  term &response {
    status: 200,
    body: to_json(user_profile)
  };
};
```

## Error Handling

### Define Error Types

```brief
# lib/errors.bv
union ApiError {
  NotFound { resource: String },
  Unauthorized { reason: String },
  BadRequest { message: String },
  ServerError { details: String },
};

txn error_to_response [true][response.status >= 400] {
  match error {
    NotFound(r) => {
      term &response {
        status: 404,
        body: to_json({ code: "NOT_FOUND", message: "Resource not found: " + r.resource })
      };
    }
    Unauthorized(r) => {
      term &response {
        status: 401,
        body: to_json({ code: "UNAUTHORIZED", message: r.reason })
      };
    }
    BadRequest(m) => {
      term &response {
        status: 400,
        body: to_json({ code: "BAD_REQUEST", message: m.message })
      };
    }
    ServerError(d) => {
      term &response {
        status: 500,
        body: to_json({ code: "SERVER_ERROR", message: "Internal server error" })
      };
    }
  };
};
```

### Error Boundary Route

```brief
# routes/[error].bv
[route]
method = "ANY"
path = "*"

[post]
response.status >= 400

txn handle_error [true][post] {
  term &response {
    status: 404,
    body: to_json({
      error: "Not found",
      path: context.path
    })
  };
};
```

## Forms & Validation

### Form Validation

```brief
# lib/forms.bv
struct LoginForm {
  email: String,
  password: String,
};

union FormValidation {
  Valid,
  Invalid { errors: [String] },
};

txn validate_email [email.length > 0][result == Valid || result == Invalid] {
  let has_at = contains(email, "@");
  let has_dot = contains(email, ".");
  
  match (has_at, has_dot) {
    (true, true) => term &Valid;
    _ => term &Invalid { errors: ["Invalid email format"] };
  };
};

txn validate_password [password.length > 0][result == Valid || result == Invalid] {
  let is_long_enough = password.length >= 8;
  let has_upper = contains_uppercase(password);
  let has_digit = contains_digit(password);
  
  let errors = [];
  
  if !is_long_enough {
    errors = append(errors, "Password must be at least 8 characters");
  };
  if !has_upper {
    errors = append(errors, "Password must contain uppercase letter");
  };
  if !has_digit {
    errors = append(errors, "Password must contain digit");
  };
  
  match errors.length {
    0 => term &Valid;
    _ => term &Invalid { errors: errors };
  };
};
```

### Register Route with Validation

```brief
# routes/POST.auth.register.bv
import lib/auth.bv;
import lib/forms.bv;

[route]
method = "POST"
path = "/auth/register"

[pre]
context.body exists

[post]
response.status == 201 || response.status == 400

txn handle_register [pre][post] {
  let form_json = parse_form(context.body);
  let form = LoginForm {
    email: form_json.email,
    password: form_json.password
  };
  
  let validation = validate_login_form(form);
  
  match validation {
    Valid => {
      let new_user = auth::create_user_account(form.email, form.password);
      term &response {
        status: 201,
        body: to_json({ user: new_user })
      };
    }
    Invalid { errors: e } => {
      term &response {
        status: 400,
        body: to_json({ errors: e })
      };
    }
  };
};
```

## Middleware

### Custom Middleware

```brief
# middleware/request_logger.bv
frgn sig log_request(method: String, path: String, timestamp: String) -> Void;

txn log_incoming_request [true][true] {
  let timestamp = now();
  log_request(context.method, context.path, timestamp);
  term;
};
```

### Rate Limiting Middleware

```brief
# middleware/rate_limit.bv
union RateLimitResult {
  Allowed { remaining: Int },
  TooManyRequests,
};

frgn sig check_rate_limit(client_ip: String) -> String;

txn apply_rate_limit [context.headers.{"x-forwarded-for"} exists]
                     [result == Allowed || result == TooManyRequests] {
  let client_ip = extract_client_ip(context);
  let result_json = check_rate_limit(client_ip);
  let result = parse_rate_limit_result(result_json);
  
  match result {
    allowed(remaining) => term &Allowed { remaining: remaining };
    too_many => term &TooManyRequests;
  };
};
```

## Components

### Server-Side Component

```brief
# components/user-card.bv
[component]
context = "server"

struct Props {
  user: User,
};

txn render_user_card [user.id exists][html contains user.name] {
  let html = build_html({
    <div class="user-card">
      <h2 b-text="user.name">User Name</h2>
      <p b-text="user.email">email@example.com</p>
      <a href="/users/{user.id}">View Profile</a>
    </div>
  });
  
  term &html;
};
```

### Client-Side Component

```brief
# components/counter.bv
[component]
context = "client"

struct Props {
  initial: Int,
};

struct LocalState {
  count: Int,
};

txn render_counter [initial >= 0][html contains "Count"] {
  let state = LocalState { count: initial };
  
  term &html {
    <div class="counter">
      <p>Count: <span b-text="state.count">0</span></p>
      <button b-trigger:click="increment">+</button>
      <button b-trigger:click="decrement">−</button>
    </div>
  };
};

txn increment [true][state.count == old_count + 1] {
  state.count = state.count + 1;
  term;
};

txn decrement [state.count > 0][state.count == old_count - 1] {
  state.count = state.count - 1;
  term;
};
```

## Advanced Patterns

### Pagination

```brief
# lib/pagination.bv
struct PaginatedResult {
  items: [Any],
  page: Int,
  limit: Int,
  total: Int,
};

txn parse_pagination_params [context.query.page exists][result.page > 0] {
  let page = parse_int(context.query.page);
  let limit = parse_int(context.query.limit);
  
  let safe_page = max(page, 1);
  let safe_limit = min(limit, 100);
  
  term &PaginationParams { page: safe_page, limit: safe_limit };
};
```

### Soft Deletes

```brief
# lib/soft_delete.bv
txn soft_delete_post [id > 0][result == deleted] {
  let json = update_post_deleted_at(id, now());
  term &deleted;
};

txn find_active_posts [true][result == posts] {
  let json = query_active_posts();
  let posts = parse_posts_from_json(json);
  term &posts;
};
```

---

These examples demonstrate the core patterns for building with Codicil.
