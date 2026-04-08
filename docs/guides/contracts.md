# Contracts Guide

Codicil brings contract-driven programming to web development. Contracts are specifications that define what must be true before (preconditions) and after (postconditions) a handler executes.

## Preconditions (`[pre]`)

Preconditions define requirements that must be satisfied before the handler runs. If a precondition fails, the handler is not executed.

```brief
[route]
method = "GET"
path = "/users/:id"

[pre]
params.id > 0
```

### Common Precondition Patterns

#### Parameter Validation

```brief
[pre]
params.id > 0 && params.id < 1000000
```

```brief
[pre]
params.email.contains("@")
```

```brief
[pre]
params.name.len() > 0 && params.name.len() < 100
```

#### Authentication Check

```brief
[pre]
session.user_id > 0
```

```brief
[pre]
headers.authorization.starts_with("Bearer ")
```

#### Content Validation

```brief
[pre]
body.len() > 0
```

```brief
[pre]
query.page is int
```

## Postconditions (`[post]`)

Postconditions define guarantees about the response. They are verified after the handler completes.

```brief
[post]
response.status == 200
```

### Common Postcondition Patterns

#### Status Code Verification

```brief
[post]
response.status == 200 || response.status == 404
```

```brief
[post]
response.status >= 200 && response.status < 300
```

#### Response Shape

```brief
[post]
response.body.contains("id")
```

```brief
[post]
response.headers["content-type"].starts_with("application/json")
```

#### Business Rules

```brief
[post]
response.body.result.len() > 0
```

```brief
[post]
response.body.total >= 0
```

## Full Example

```brief
[route]
method = "POST"
path = "/users"

[pre]
body.contains("name") && body.contains("email")

[post]
response.status == 201 && response.body.id > 0

txn handle [pre][post] {
    term &response {
        status: 201,
        body: {
            id: 1,
            name: "Alice",
            email: "alice@example.com"
        }
    };
};
```

## Contract Evaluation

Contracts are evaluated by the Brief compiler:

1. **Preconditions** are checked before the handler executes
2. **Handler body** executes if preconditions pass
3. **Postconditions** are checked after the handler completes
4. If any contract fails, an error is returned

## Error Handling

When a contract fails, the error is passed to `[error].bv`:

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

### Error Codes

| Error | Code | Cause |
|-------|------|-------|
| PreconditionFailed | `BAD_REQUEST` | Precondition evaluated to false |
| PostconditionFailed | `INTERNAL_ERROR` | Postcondition evaluated to false |
| CompilationFailed | `INTERNAL_ERROR` | Brief code failed to compile |

## Benefits of Contracts

1. **Self-documenting code** - Contracts serve as executable documentation
2. **Fail fast** - Invalid requests are rejected early
3. **Refactoring safety** - Changes that break contracts are caught immediately
4. **Verified behavior** - Postconditions ensure responses meet expectations
