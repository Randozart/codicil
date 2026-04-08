# FFI Guide

Codicil provides FFI (Foreign Function Interface) bindings to access external services from your Brief code. These bindings are available through the codicil-ffi crate.

## Available FFI Functions

### JSON Operations

#### `json_parse(s: String) -> Result<JsonValue, String>`

Parse a JSON string into a structured value.

```brief
txn handle [true][post] {
    term;
};
```

#### `json_stringify(data: Data) -> Result<String, String>`

Convert a value to JSON string.

```brief
txn handle [true][post] {
    term;
};
```

### HTTP Operations

#### `http_get(url: String) -> Result<HttpResponse, String>`

Make an HTTP GET request.

```brief
txn handle [true][post] {
    term;
};
```

#### `http_post(url: String, body: String) -> Result<HttpResponse, String>`

Make an HTTP POST request with JSON body.

```brief
txn handle [true][post] {
    term;
};
```

### Database Operations

#### `db_query(query: String, params: Data) -> Result<Data, String>`

Execute a PostgreSQL query.

```brief
txn handle [true][post] {
    term;
};
```

**Parameters:**
- `query` - SQL query with `$1`, `$2`, etc. placeholders
- `params` - Array of parameter values

**Returns:** Array of row objects

## Configuration

### Environment Variables

Set these in your `.env` file:

```bash
DATABASE_URL=postgresql://localhost:5432/mydb
```

## Complete Example

```brief
# Fetch user data from external API
txn fetch_user [id > 0][post] {
    term;
};
```

## Error Handling

FFI functions return `Result<T, String>` - handle errors with your `[error].bv` route.

## Type Mappings

| Brief Type | JSON Type |
|------------|-----------|
| `Int` | Number |
| `String` | String |
| `Bool` | Boolean |
| `Array` | Array |
| `Struct` | Object |
