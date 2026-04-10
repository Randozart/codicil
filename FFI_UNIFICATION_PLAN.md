# FFI Unification Plan: Eliminate `frgn sig`

## Context

Brief currently has two FFI mechanisms:
- **`frgn sig __http_get(url: String) -> String;`** — type-checks only, no binding, no location, no error handling. Silent magic.
- **`frgn __parse(s: String) -> Result<Data, JsonError> from "json.toml";`** — full binding with TOML file declaring types, error structure, and implementation location.

`frgn sig` violates Brief's philosophy: every foreign function must have an explicit contract. The TOML file is that contract. There must be no magic words.

**Goal:** Remove `frgn sig` entirely. Every FFI function uses `frgn name(params) -> Result<T, Error> from "path.toml"`. All ~120 functions in the standard library are converted. All hardcoded FFI implementations in the WASM codegen are removed and replaced by TOML-driven generation.

---

## Rules

1. **`frgn sig` is removed from the language.** If a programmer writes it, the parser emits: `"frgn sig is removed. Use frgn name(...) -> Result<T, Error> from 'file.toml';"`
2. **All `frgn` functions return `Result<T, Error>.`** External code can always fail. The programmer must handle every error path.
3. **The postcondition must resolve on every code path**, including error paths. Example:
   ```
   [response Ok(body)] { &hint_result = body; };
   [response Err(e)] { &hint_result = "Error"; };
   ```
   Postcondition `[~""]` is satisfied because `hint_result` is set on all paths.
4. **The TOML file is the single source of truth** for the FFI boundary: function name, input types, success output, error type, error fields, implementation location, target platform.
5. **TOML paths resolve relative to the declaring `.bv` file.** `from "http.toml"` in `lib/std/http.bv` resolves to `lib/std/http.toml`. `from "std/bindings/json.toml"` resolves via the standard search path.
6. **No hardcoded FFI functions.** Every JS function in the WASM glue code must be traceable to a TOML binding. No function names appear in `wasm_gen.rs` without corresponding TOML metadata.

---

## Current State

### Files that use `frgn sig` (all must be converted)

**Standard library `.bv` files** (in `brief-compiler/lib/std/`):
- `json.bv` — 31 functions (`__parse`, `__stringify`, `__is_null`, `__get_string`, etc.)
- `http.bv` — 2 functions (`__http_get`, `__http_post`)
- `math.bv` — 38 functions (`__sin`, `__cos`, `__sqrt`, `__random`, etc.)
- `io.bv` — 3 functions (`__print`, `__println`, `__input`)
- `string.bv` — 28 functions (`__to_lower`, `__to_upper`, `__contains_at`, etc.)
- `time.bv` — 21 functions (`__now`, `__year`, `__format_timestamp`, etc.)
- `encoding.bv` — 22 functions (`__base64_encode`, `__md5`, `__uuid_v4`, etc.)
- `collections.bv` — 6 functions (`__filter`, `__map`, `__reduce`, etc.)

**Total: ~151 `frgn sig` declarations to convert.**

### Existing TOML bindings (in `brief-compiler/lib/ffi/bindings/`)

- `json.toml` — 31 functions, `target = "native"` only, `location = "brief_ffi_native::__parse"`
- `math.toml` — 38 functions, `target = "native"` only
- `io.toml` — 3 functions, `target = "native"` only
- `string.toml` — 28 functions, `target = "native"` only
- `time.toml` — 21 functions, `target = "native"` only
- `encoding.toml` — 22 functions, `target = "native"` only

**Missing TOML files:**
- `http.toml` — does not exist
- `collections.toml` — does not exist

**Each existing TOML needs a `wasm` target entry added** (currently only has `native`).

### Compiler code referencing `frgn sig`

- `src/parser.rs` — `parse_frgn_sig()` (lines 392-425), `parse_top_level()` branch (lines 207-221)
- `src/ast.rs` — `TopLevel::ForeignSig` variant, `ForeignSig` struct
- `src/typechecker.rs` — `TopLevel::ForeignSig` handling, `foreign_sigs` HashMap population
- `src/wasm_gen.rs` — hardcoded JS FFI functions (`__http_get`, `__json_decode`, etc.)
- `src/wrapper/generator.rs` — `"Foreign function declarations (frgn sig)"` comment (line 23)
- `src/wrapper/wasm_analyzer.rs` — `"frgn sig ..."` format string (line 180)
- `src/wrapper/c_analyzer.rs` — `"frgn sig ..."` format strings (lines 98, 101)
- `src/wrapper/python_analyzer.rs` — `"frgn sig ..."` format string (line 445)
- `src/wrapper/rust_analyzer.rs` — `"frgn sig ..."` format string (line 174)
- `src/wrapper/js_analyzer.rs` — `"frgn sig ..."` format string (line 540)
- `src/main.rs` — wrapper template comment (line 281)
- `src/annotator.rs` — annotation output (line 378)

### WASM codegen gaps

`statement_to_rust()` in `src/wasm_gen.rs` (line 709) only handles `Statement::Assignment` and `Statement::Term`. The wildcard `_ => {}` (line 752) silently drops:
- `Statement::Let` — `let raw = __http_get("/hints");` produces no output
- `Statement::Expression` — `__http_get(url)` call produces no output
- `Statement::Guarded` — `[parsed != null] { ... }` produces no output
- `Statement::Escape` — `escape value;` produces no output
- `Statement::Unification` — produces no output

---

## Implementation Phases

### Phase 1: Remove `frgn sig` from the parser and AST

**File: `brief-compiler/src/ast.rs`**

1. Remove the `ForeignSig` struct definition (around line 303-308):
   ```rust
   pub struct ForeignSig {
       pub name: String,
       pub input_types: Vec<Type>,
       pub outputs: Vec<Type>,
   }
   ```

2. Remove the `TopLevel::ForeignSig` variant from the `TopLevel` enum (around line 311). The `ForeignBinding` variant remains.

3. Remove `ForeignSig` from any `use` statements or imports.

**File: `brief-compiler/src/parser.rs`**

4. In `parse_top_level()` (around line 176), change the `Token::Frgn` branch. Remove the `is_frgn_sig` peek logic (lines 207-221). All `frgn` tokens now route to `parse_frgn_binding()`:
   ```rust
   Some(Ok(Token::Frgn)) => {
       let frgn_binding = self.parse_frgn_binding()?;
       Ok(frgn_binding)
   }
   ```

5. Delete the `parse_frgn_sig()` function entirely (lines 392-425).

6. In `parse_frgn_binding()`, verify it handles the `frgn name(params) -> Result<T, E> from "path.toml";` syntax correctly. The existing function (lines 441-551) already parses this. Confirm:
   - It expects `Result` keyword after `->`
   - It expects `<SuccessType, ErrorType>` generic syntax
   - It expects `from "path"` clause
   - It produces `TopLevel::ForeignBinding` with correct fields

**Verification:** Write a test Brief file with `frgn __http_get(url: String) -> Result<String, HttpError> from "http.toml";` and confirm it parses. Confirm that `frgn sig __test() -> String;` produces a clear error message.

---

### Phase 2: Update the type checker

**File: `brief-compiler/src/typechecker.rs`**

7. Remove the `TopLevel::ForeignSig` match arm in `check_program()` first pass (around line 94-96):
   ```rust
   // DELETE THIS:
   TopLevel::ForeignSig(frgn_sig) => {
       self.foreign_sigs.insert(frgn_sig.name.clone(), frgn_sig.clone());
   }
   ```

8. Remove the `TopLevel::ForeignSig` match arm in `check_program()` second pass (around line 155-158).

9. In the `TopLevel::ForeignBinding` handling (around line 159-170), after loading and validating the TOML binding, also populate `self.foreign_sigs` from the binding's signature:
   ```rust
   TopLevel::ForeignBinding { name, toml_path, signature, .. } => {
       self.check_frgn_binding(name, toml_path, signature);
       // Also register as a foreign sig for type inference
       let frgn_sig = ForeignSig {
           name: name.clone(),
           input_types: signature.inputs.iter().map(|(_, ty)| ty.clone()).collect(),
           outputs: signature.success_output.iter().map(|(_, ty)| ty.clone()).collect(),
       };
       self.foreign_sigs.insert(name.clone(), frgn_sig);
   }
   ```

   This ensures type inference for FFI calls works when they use `frgn ... from` instead of `frgn sig`.

10. Remove the `ForeignSig` import if present.

**Verification:** Compile a Brief file with `frgn __http_get(url: String) -> Result<String, HttpError> from "http.toml";` and confirm type checking passes. Confirm calling `__http_get("/hints")` in a transaction type-checks.

---

### Phase 3: Update TOML binding files

**Goal:** Each TOML file has both `native` and `wasm` target entries. The `wasm` target's `location` is the JavaScript function name (which matches the function's Brief name).

#### 3a: Create `lib/ffi/bindings/http.toml`

```toml
# HTTP Bindings

[[functions]]
name = "__http_get"
location = "brief_ffi_native::__http_get"
target = "native"
mapper = "rust"
description = "HTTP GET request"

[functions.input]
url = "String"

[functions.output.success]
result = "String"

[functions.output.error]
type = "HttpError"
code = "Int"
message = "String"

[[functions]]
name = "__http_get"
location = "__http_get"
target = "wasm"
mapper = "wasm"
description = "HTTP GET request (WASM - synchronous XMLHttpRequest)"

[functions.input]
url = "String"

[functions.output.success]
result = "String"

[functions.output.error]
type = "HttpError"
code = "Int"
message = "String"

[[functions]]
name = "__http_post"
location = "brief_ffi_native::__http_post"
target = "native"
mapper = "rust"
description = "HTTP POST request"

[functions.input]
url = "String"
body = "String"

[functions.output.success]
result = "String"

[functions.output.error]
type = "HttpError"
code = "Int"
message = "String"

[[functions]]
name = "__http_post"
location = "__http_post"
target = "wasm"
mapper = "wasm"
description = "HTTP POST request (WASM)"

[functions.input]
url = "String"
body = "String"

[functions.output.success]
result = "String"

[functions.output.error]
type = "HttpError"
code = "Int"
message = "String"
```

#### 3b: Create `lib/ffi/bindings/collections.toml`

Create following the same format as other TOML files. Each function has `native` and `wasm` entries. `wasm` target `location` = the function's Brief name (e.g., `__filter`).

Functions to include: `__filter`, `__map`, `__reduce`, `__unique`, `__sort`, `__reverse`. Use `CollectionsError` as the error type with `code = "Int"` and `message = "String"`.

#### 3c: Add `wasm` target entries to ALL existing TOML files

For each function in each TOML file, add a second `[[functions]]` entry with:
- `target = "wasm"`
- `mapper = "wasm"`
- `location = "__function_name"` (the Brief function name itself — this is how the JS glue knows which function to call)
- Same `input`, `output.success`, `output.error` as the `native` entry

Files to update:
- `lib/ffi/bindings/json.toml` — add 31 `wasm` entries
- `lib/ffi/bindings/math.toml` — add 38 `wasm` entries
- `lib/ffi/bindings/io.toml` — add 3 `wasm` entries
- `lib/ffi/bindings/string.toml` — add 28 `wasm` entries
- `lib/ffi/bindings/time.toml` — add 21 `wasm` entries
- `lib/ffi/bindings/encoding.toml` — add 22 `wasm` entries

The `wasm` entries are exact copies of the `native` entries except for `target` and `location`.

**Verification:** Load each TOML file with `toml::from_str()` and confirm it parses without errors. Confirm each file has exactly 2x the number of functions (one native, one wasm).

---

### Phase 4: Update TOML path resolver

**File: `brief-compiler/src/ffi/resolver.rs`**

11. Add file-relative resolution. The `resolve_binding_path` function currently takes `binding_path` and `project_root`. Add a third parameter `source_file_path: &Option<PathBuf>`:

    ```rust
    pub fn resolve_binding_path(
        binding_path: &str,
        project_root: &Option<PathBuf>,
        source_file_path: &Option<PathBuf>,  // NEW
    ) -> Result<PathBuf, FfiError> {
    ```

12. Add a new Case 0 at the top of the function, before Case 1 (absolute path):

    ```rust
    // Case 0: Relative to the declaring source file
    if let Some(source_path) = source_file_path {
        if let Some(source_dir) = source_path.parent() {
            let resolved = source_dir.join(binding_path);
            if resolved.exists() {
                return Ok(resolved);
            }
        }
    }
    ```

13. Update all callers of `resolve_binding_path` to pass the source file path. The primary caller is in `typechecker.rs`'s `check_frgn_binding` method (around line 456). Pass the type checker's current file path.

**File: `brief-compiler/src/typechecker.rs`**

14. In `check_frgn_binding()` (around line 456), update the call to `resolve_binding_path`:
    ```rust
    let resolved_path = match ffi::resolver::resolve_binding_path(
        toml_path,
        &None,
        &Some(self.current_file.clone()),  // Pass the .bv file path
    ) { ... };
    ```

    If the type checker doesn't have a `current_file` field, add one and set it during initialization.

**Verification:** Put `http.toml` next to `http.bv`. In `http.bv`, write `frgn __http_get(url: String) -> Result<String, HttpError> from "http.toml";`. Confirm the resolver finds the TOML file relative to the `.bv` file.

---

### Phase 5: Update WASM codegen

**File: `brief-compiler/src/wasm_gen.rs`**

#### 5a: Add missing statement types to `statement_to_rust`

The function at line 709 currently has:

```rust
fn statement_to_rust(&self, output: &mut String, stmt: &Statement) {
    match stmt {
        Statement::Assignment { ... } => { ... }
        Statement::Term(_) => { ... }
        _ => {}  // <-- SILENTLY DROPS EVERYTHING ELSE
    }
}
```

Replace the `_ => {}` with proper arms:

```rust
Statement::Let { name, ty, expr } => {
    if let Some(e) = expr {
        let value_js = self.expr_to_js_value(e);
        // Track the variable for later use
        self.var_map.insert(name.clone(), self.var_counter);
        let signal_id = self.var_counter;
        self.var_counter += 1;

        // Generate: let name = value_js;
        output.push_str(&format!("        let {name} = {value_js};\n"));
    }
}

Statement::Expression(expr) => {
    let expr_js = self.expr_to_js_value(expr);
    output.push_str(&format!("        {expr_js};\n"));
}

Statement::Guarded { condition, statements } => {
    let cond_js = self.expr_to_js_value(condition);
    output.push_str(&format!("        if {cond_js} {{\n"));
    for stmt in statements {
        self.statement_to_rust(output, stmt);
    }
    output.push_str("        }\n");
}

Statement::Escape(expr) => {
    if let Some(e) = expr {
        let value_js = self.expr_to_js_value(e);
        output.push_str(&format!("        return {value_js};\n"));
    } else {
        output.push_str("        return;\n");
    }
}

Statement::Unification { name, pattern, expr } => {
    // Unification: name(pattern) = expr
    let value_js = self.expr_to_js_value(expr);
    output.push_str(&format!("        // unification: {name}({pattern}) = {value_js}\n"));
}
```

**Note:** The exact code will depend on how `self` tracks variables and signals. Look at how `Statement::Assignment` works (lines 711-748) and follow the same pattern. The key principle: each statement type must produce valid Rust/WASM code. No statement type may be silently dropped.

#### 5b: Fix `Expr::Call` for FFI functions returning `Result`

Currently (line 941):
```rust
Expr::Call(name, args) => {
    let args_vals: Vec<String> = args.iter().map(|a| self.expr_to_js_value(a)).collect();
    format!("{}({})", name, args_vals.join(", "))
}
```

This generates a raw JS function call. For FFI functions that return `Result<T, E>`, the call needs to be wrapped.

Check if the called function is a `frgn` binding (look up in `self.foreign_bindings` or similar). If it is:
- Generate code that calls the JS function and wraps the result in `Ok()` on success
- Catch JS exceptions and wrap in `Err(...)` on failure

Example generated Rust:
```rust
// Brief: let raw = __http_get("/hints");
// Generated:
let raw: Result<String, JsValue> = {
    let fn_ref = js_sys::Reflect::get(&js_sys::global(), &JsValue::from("__http_get"));
    match fn_ref {
        Ok(f) => {
            match f.dyn_into::<js_sys::Function>() {
                Ok(func) => {
                    match func.call1(&JsValue::NULL, &JsValue::from("/hints")) {
                        Ok(val) => Ok(val.as_string().unwrap_or_default()),
                        Err(e) => Err(e),
                    }
                }
                Err(_) => Err(JsValue::from_str("Not a function")),
            }
        }
        Err(e) => Err(e),
    }
};
```

If the function is NOT a `frgn` binding (a regular Brief function call), generate the existing `name(args)` format.

#### 5c: Remove hardcoded FFI from JS glue generation

The `generate_js_glue()` function (around line 1050-1111) hardcodes JS implementations for:
- `__json_decode`, `__json_encode`, `__json_get_string`, `__json_get_number`, `__json_get_bool`, `__json_has_key`
- `__http_get`, `__http_post`

**Replace with TOML-driven generation:**

1. Add a method `collect_frgn_bindings(&self)` that reads all TOML binding files referenced by the program and collects the `wasm` target entries.

2. For each `wasm` target entry:
   - Look up the JS implementation. Options:
     a. The TOML includes a `wasm_impl` field with the JS code directly
     b. The TOML includes a `wasm_path` field pointing to a `.js` file
     c. There's a JS implementation registry (a HashMap mapping function names to JS code strings)
   - Generate the JS function in the glue code

3. For the initial implementation, add a `wasm_impl` field to the TOML for functions that need custom JS implementations:

   ```toml
   [[functions]]
   name = "__http_get"
   location = "__http_get"
   target = "wasm"
   mapper = "wasm"
   description = "HTTP GET request (WASM)"
   wasm_impl = """
   function __http_get(url) {
       try {
           const xhr = new XMLHttpRequest();
           xhr.open('GET', url, false);
           xhr.send();
           if (xhr.status >= 200 && xhr.status < 300) {
               return xhr.responseText;
           }
           return '';
       } catch(e) {
           console.error('HTTP GET error:', e.message);
           return '';
       }
   }
   """

   [functions.input]
   url = "String"

   [functions.output.success]
   result = "String"

   [functions.output.error]
   type = "HttpError"
   code = "Int"
   message = "String"
   ```

4. In `generate_js_glue()`, iterate over collected wasm bindings and emit:
   ```rust
   for binding in self.wasm_bindings.iter() {
       if let Some(impl_code) = &binding.wasm_impl {
           output.push_str(impl_code);
           output.push_str("\n");
       }
   }
   ```

5. For functions that don't need custom JS implementations (simple math, string operations), the `wasm_impl` field is omitted. The WASM codegen generates the Rust implementation directly using `wasm_bindgen` or native Rust code.

**Important:** The hardcoded `__json_decode`, `__json_encode`, etc. functions MUST be moved to their respective TOML `wasm_impl` fields before removing them from `wasm_gen.rs`. Do not remove the hardcoded implementations until the TOML replacements are verified to work.

#### 5d: Add `wasm_bindgen` extern declarations

For each `wasm` target TOML binding, generate a `#[wasm_bindgen] extern` block in the Rust output:

```rust
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = __http_get)]
    fn __http_get_wasm(url: &str) -> JsValue;
}
```

This makes the JS functions available to the Rust/WASM code via FFI imports, rather than relying on them being in the global scope.

**Verification:** Recompile the landing page RBV. Confirm the generated `landing.rs` includes `Statement::Let` output, `Statement::Expression` output, and `Statement::Guarded` output. Confirm the JS glue code includes all FFI functions from the TOML bindings.

---

### Phase 6: Update standard library `.bv` files

**Goal:** Convert all ~151 `frgn sig` declarations to `frgn ... from` syntax. Update all wrapper `defn` functions to handle `Result` returns.

#### 6a: `lib/std/http.bv`

Replace entire file with:

```brief
// Standard HTTP Library
// Foreign function declarations for HTTP operations.

frgn __http_get(url: String) -> Result<String, HttpError> from "http.toml";
frgn __http_post(url: String, body: String) -> Result<String, HttpError> from "http.toml";

defn http_get(url: String) -> Result<String, HttpError> [true][true] {
    term __http_get(url);
};

defn http_post(url: String, body: String) -> Result<String, HttpError> [true][true] {
    term __http_post(url, body);
};
```

Note: The `defn` wrappers can simply pass through the `Result`. The caller handles the error.

#### 6b: `lib/std/json.bv`

Convert each of the 31 `frgn sig` declarations. Example:

```brief
// Before:
frgn sig __parse(s: String) -> Data;

// After:
frgn __parse(s: String) -> Result<Data, JsonError> from "json.toml";
```

Update the `defn` wrapper functions. Example:

```brief
// Before:
defn json_parse(s: String) [true][true] -> Data {
  term __parse(s);
};

// After:
defn json_parse(s: String) [true][true] -> Result<Data, JsonError> {
  term __parse(s);
};
```

#### 6c: `lib/std/math.bv`

Convert 38 functions. Math functions typically can't fail (e.g., `__sin` always succeeds for valid Float inputs). Use `Result<Float, MathError>` anyway per the rule. The `MathError` type is defined in `math.toml` with `code = "Int"` and `message = "String"`.

#### 6d: `lib/std/io.bv`

Convert 3 functions. `__print` and `__println` return `Result<Bool, IoError>`. `__input` returns `Result<String, IoError>`.

#### 6e: `lib/std/string.bv`

Convert 28 functions. Use `Result<T, StringError>`.

#### 6f: `lib/std/time.bv`

Convert 21 functions. Use `Result<Int, TimeError>` or `Result<String, TimeError>`.

#### 6g: `lib/std/encoding.bv`

Convert 22 functions. Use `Result<T, EncodingError>`.

#### 6h: `lib/std/collections.bv`

Convert 6 functions. Use `Result<List<T>, CollectionsError>`.

**Verification:** Compile each `.bv` file with the updated compiler. Confirm no `frgn sig` syntax remains anywhere in the codebase. Run:
```bash
grep -rn "frgn sig" brief-compiler/lib/ brief-compiler/src/ brief-compiler/spec/ brief-compiler/examples/
```
This should return zero results.

---

### Phase 7: Update landing page RBV

**File: `codicil/landing-page/components/landing.rbv`**

#### 7a: Update FFI declarations

```brief
// Before:
frgn sig __http_get(url: String) -> String;
frgn sig __json_decode(json_str: String) -> Data;
frgn sig __json_get_string(data: Data, key: String) -> String;

// After:
frgn __http_get(url: String) -> Result<String, HttpError> from "std/bindings/http.toml";
frgn __json_decode(json_str: String) -> Result<Data, JsonError> from "std/bindings/json.toml";
frgn __json_get_string(data: Data, key: String) -> Result<String, JsonError> from "std/bindings/json.toml";
```

#### 7b: Update `get_hint` transaction with Result handling

```brief
txn get_hint [hint_result == ""][~hint_result] {
    let response = __http_get("/hints");
    [response Ok(body)] {
        let parsed = __json_decode(body);
        [parsed Ok(data)] {
            let hint = __json_get_string(data, "hint");
            [hint Ok(m)] {
                &hint_result = m;
            };
            [hint Err(e)] {
                &hint_result = "Error: hint field missing";
            };
        };
        [parsed Err(e)] {
            &hint_result = "Error: invalid response";
        };
    };
    [response Err(e)] {
        &hint_result = "Error: could not fetch hint";
    };
    term;
};
```

Postcondition `[~hint_result]` means "hint_result is not empty" (logical NOT of empty string). Every error path sets `hint_result`, so the postcondition is guaranteed to resolve.

#### 7c: Ensure the landing page routes to std bindings

The `from "std/bindings/http.toml"` path must be resolvable. Either:
- Copy the TOML files to the landing page's working directory under `std/bindings/`
- Or use an absolute path in the `from` clause
- Or rely on the file-relative resolution (if `http.toml` is next to `http.bv`, and the compiler resolves relative to the `.bv` file, this works automatically when the compiler's lib path is known)

The simplest approach: the compiler should have a built-in standard library path (like `brief-compiler/lib/ffi/bindings/`). When the `from` path starts with `std/bindings/`, resolve it against this built-in path. Update the resolver accordingly.

---

### Phase 8: Update the FFI resolver to know about the standard library path

**File: `brief-compiler/src/ffi/resolver.rs`**

15. Add a `STANDARD_LIB_PATH` constant or configuration that points to `brief-compiler/lib/ffi/bindings/`. This can be:
    - A compile-time constant pointing to the crate's source directory
    - An environment variable `BRIEF_STDLIB_PATH`
    - Detected from the binary's location

16. Update the `std/bindings/` resolution case (existing lines 28-46):
    ```rust
    if binding_path.starts_with("std/bindings/") {
        // Strip prefix to get just the filename
        let filename = binding_path.strip_prefix("std/bindings/").unwrap();

        // Try standard library path
        let stdlib_path = PathBuf::from(STANDARD_LIB_PATH).join(filename);
        if stdlib_path.exists() {
            return Ok(stdlib_path);
        }

        // Fall back to existing resolution
        // ... (keep existing fallback logic)
    }
    ```

**Verification:** From any directory, compile a Brief file that uses `from "std/bindings/http.toml"`. Confirm the resolver finds the file in the compiler's standard library directory.

---

### Phase 9: Build and test

17. Rebuild the brief compiler:
    ```bash
    cd brief-compiler && cargo build --release
    ```

18. Install the updated brief:
    ```bash
    ./target/release/brief-compiler install
    ```

19. Rebuild codi:
    ```bash
    cd codicil && cargo build --release
    ```

20. Recompile the landing page:
    ```bash
    ./target/release/codi dev landing-page
    ```

21. Test in browser at `http://localhost:3000`:
    - Page loads with correct styling
    - Counter increment/reset works
    - "Get Random Hint" button fetches from `/hints` and displays the hint
    - If the server is unreachable, the error message is displayed

22. Verify no magic words remain:
    ```bash
    grep -rn "frgn sig" brief-compiler/lib/ brief-compiler/src/ brief-compiler/spec/ brief-compiler/examples/
    # Should return 0 results
    
    grep -rn "__http_get\|__json_decode\|__json_get_string" brief-compiler/src/wasm_gen.rs
    # Should return 0 results (all moved to TOML)
    ```

---

## Commit strategy

One commit per phase, with clear messages:

1. `"Remove frgn sig: update parser and AST"`
2. `"Remove frgn sig: update type checker"`
3. `"Create/update TOML bindings with wasm targets"`
4. `"Add file-relative TOML path resolution"`
5. `"Update WASM codegen: handle all statement types, remove hardcoded FFI"`
6. `"Convert all standard library .bv files from frgn sig to frgn from"`
7. `"Update landing page RBV with Result-based FFI and error handling"`
8. `"Add standard library path resolution to FFI resolver"`
9. `"Build, install, and verify"`

Each phase should compile and pass basic tests before proceeding to the next.

---

## Files changed summary

| File | Change |
|------|--------|
| `brief-compiler/src/ast.rs` | Remove `ForeignSig`, `TopLevel::ForeignSig` |
| `brief-compiler/src/parser.rs` | Remove `parse_frgn_sig()`, remove `frgn sig` branch |
| `brief-compiler/src/typechecker.rs` | Remove `ForeignSig` handling, populate `foreign_sigs` from bindings |
| `brief-compiler/src/wasm_gen.rs` | Add `Let`/`Expression`/`Guarded`/`Escape` statement handling, remove hardcoded FFI, generate from TOML |
| `brief-compiler/src/ffi/resolver.rs` | Add file-relative resolution, add stdlib path |
| `brief-compiler/src/wrapper/generator.rs` | Update `frgn sig` strings to `frgn ... from` |
| `brief-compiler/src/wrapper/wasm_analyzer.rs` | Update format strings |
| `brief-compiler/src/wrapper/c_analyzer.rs` | Update format strings |
| `brief-compiler/src/wrapper/python_analyzer.rs` | Update format strings |
| `brief-compiler/src/wrapper/rust_analyzer.rs` | Update format strings |
| `brief-compiler/src/wrapper/js_analyzer.rs` | Update format strings |
| `brief-compiler/src/main.rs` | Update template comments |
| `brief-compiler/src/annotator.rs` | Update annotation output |
| `brief-compiler/lib/ffi/bindings/http.toml` | **NEW** — HTTP bindings with native+wasm targets |
| `brief-compiler/lib/ffi/bindings/collections.toml` | **NEW** — Collections bindings |
| `brief-compiler/lib/ffi/bindings/json.toml` | Add wasm target entries |
| `brief-compiler/lib/ffi/bindings/math.toml` | Add wasm target entries |
| `brief-compiler/lib/ffi/bindings/io.toml` | Add wasm target entries |
| `brief-compiler/lib/ffi/bindings/string.toml` | Add wasm target entries |
| `brief-compiler/lib/ffi/bindings/time.toml` | Add wasm target entries |
| `brief-compiler/lib/ffi/bindings/encoding.toml` | Add wasm target entries |
| `brief-compiler/lib/std/http.bv` | Convert `frgn sig` to `frgn ... from` |
| `brief-compiler/lib/std/json.bv` | Convert 31 functions |
| `brief-compiler/lib/std/math.bv` | Convert 38 functions |
| `brief-compiler/lib/std/io.bv` | Convert 3 functions |
| `brief-compiler/lib/std/string.bv` | Convert 28 functions |
| `brief-compiler/lib/std/time.bv` | Convert 21 functions |
| `brief-compiler/lib/std/encoding.bv` | Convert 22 functions |
| `brief-compiler/lib/std/collections.bv` | Convert 6 functions |
| `codicil/landing-page/components/landing.rbv` | Update FFI declarations and error handling |
