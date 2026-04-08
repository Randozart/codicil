# Routing Guide

## Route File Naming

Codicil uses a file-based routing system. Route files are named `{METHOD}.{path}.bv`:

| File | Method | Path |
|------|--------|------|
| `GET.index.bv` | GET | `/` |
| `GET.users.bv` | GET | `/users` |
| `GET.users.[id].bv` | GET | `/users/:id` |
| `GET.users.[id].posts.[post_id].bv` | GET | `/users/:id/posts/:post_id` |
| `POST.users.bv` | POST | `/users` |
| `PUT.users.[id].bv` | PUT | `/users/:id` |
| `DELETE.users.[id].bv` | DELETE | `/users/:id` |
| `[error].bv` | * | Error catch-all |

## Supported HTTP Methods

- `GET` - Read/retrieve resources
- `POST` - Create new resources
- `PUT` - Full update/replace resources
- `DELETE` - Remove resources
- `PATCH` - Partial update

## Route Parameters

Use `[param]` in filenames to capture path segments:

```
routes/GET.users.[id].bv     → /users/:id
routes/GET.posts.[post_id].bv → /posts/:post_id
```

Access parameters in handlers via `params.{name}`:

```brief
[route]
method = "GET"
path = "/users/:id"

txn handle [true][post] {
    term &response {
        status: 200,
        body: { "user_id": params.id }
    };
};
```

## Query Parameters

Access query parameters via `query.{name}`:

```
/search?q=codicil&lang=en
```

```brief
txn handle [true][post] {
    term &response {
        status: 200,
        body: { "query": query.q, "lang": query.lang }
    };
};
```

## Request Body

Access the raw request body via `body`:

```brief
txn handle [true][post] {
    term &response {
        status: 200,
        body: { "received": body }
    };
};
```

## Route Metadata

Use TOML sections to define route metadata:

```brief
[route]
method = "GET"
path = "/api/users"
middleware = ["auth", "cors"]
context = "server"
handler = "handle"
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `method` | String | HTTP method (GET, POST, etc.) |
| `path` | String | URL path pattern |
| `middleware` | Array | List of middleware names |
| `context` | String | Execution context (default: "server") |
| `handler` | String | Handler function name (default: "handle") |

## Nested Routes

Files nested in subdirectories under `routes/` are also discovered:

```
routes/
├── api/
│   ├── GET.users.bv      → /api/users
│   └── GET.users.[id].bv → /api/users/:id
└── GET.index.bv           → /
```

## Error Catch-All

The `[error].bv` route handles all errors:

```brief
txn handle [true][response.status > 0] {
    term &response {
        status: error.details.status | 500,
        body: {
            code: error.code,
            message: error.message
        }
    };
};
```

Available error fields:
- `error.code` - Error code string
- `error.message` - Human-readable message
- `error.details` - Additional error data

## Static Files

Files in `public/` are served automatically:

```
public/
├── favicon.svg      → /favicon.svg
├── styles.css       → /styles.css
└── images/
    └── logo.png     → /images/logo.png
```

The favicon is also available at `/favicon.svg` for convenience.
