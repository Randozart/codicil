# Middleware Guide

Middleware allows you to intercept and process requests before they reach your handlers. Use middleware for cross-cutting concerns like authentication, logging, and CORS.

## Creating Middleware

Create a `.bv` file in the `middleware/` directory:

```brief
# middleware/auth.bv
[route]

txn handle [true][post] {
    term;
};
```

## Using Middleware in Routes

Reference middleware by name in your route:

```brief
[route]
method = "GET"
path = "/protected"
middleware = ["auth"]

txn handle [true][post] {
    term &response { status: 200, body: "Protected content" };
};
```

Apply multiple middleware:

```brief
[route]
middleware = ["auth", "cors", "ratelimit"]
```

## Middleware Execution Order

Middleware executes in the order specified:

```brief
middleware = ["auth", "cors", "ratelimit"]
# auth runs first, then cors, then ratelimit, then handler
```

## Common Middleware Patterns

### Authentication

```brief
# middleware/auth.bv
[route]

txn handle [true][post] {
    term;
};
```

### CORS Headers

```brief
# middleware/cors.bv
[route]

txn handle [true][post] {
    term;
};
```

### Rate Limiting

```brief
# middleware/ratelimit.bv
[route]

txn handle [true][post] {
    term;
};
```

### Request Logging

```brief
# middleware/log.bv
[route]

txn handle [true][post] {
    term;
};
```

## Middleware and Context

Access request context in middleware:

```brief
[route]

txn handle [true][post] {
    term;
};
```

The context includes:
- `method` - HTTP method
- `path` - Request path
- `params` - Route parameters
- `query` - Query parameters
- `headers` - Request headers
- `body` - Request body
- `user` - Authenticated user (if set)
- `session` - Session data (if set)

## Modifying Context

Middleware can modify the context that subsequent middleware and handlers receive:

```brief
[route]

txn handle [true][post] {
    term;
};
```

## Built-in Middleware

Codicil doesn't include built-in middleware - you create them as needed. This keeps the framework lightweight and lets you define exactly what your application needs.
