# Importing Libraries in Codicil: The Metropolitan FFI

Codicil leverages the **Metropolitan FFI** system to allow you to import and use libraries from any ecosystem—Node.js, NPM, Python, or Native Rust—without changing your core application logic.

## The Workflow

To use an external library in Codicil:
1.  **Define the Binding**: Create a `.toml` file. Use the `[meta]` section for shared `wasm_setup` (like imports).
2.  **Provide Implementations**: Write the platform-specific code.
3.  **Declare in Brief**: Link your components to the TOML.

---

## 1. Importing NPM Libraries (The Web Way)

Use the `wasm_setup` field in the `[meta]` section for your `import` statements. This ensures they are only included once at the top of the glue file.

### Example: Using `dayjs` for Date Formatting
Create `lib/time_utils.toml`:
```toml
[meta]
wasm_setup = "import dayjs from 'https://cdn.skypack.dev/dayjs';"

[[functions]]
name = "format_date"
location = "js_format_date"
target = "wasm"
wasm_impl = """
function js_format_date(timestamp) {
    return dayjs(timestamp).format('MMMM D, YYYY');
}
"""
# ... inputs/outputs definition
```

---

## 2. Bridging to Python (The Metropolitan Bridge)

To use a Python library (like `nltk` or `pandas`), set up a simple Python API and call it synchronously from your FFI.

### Example: Text Analysis
```toml
[[functions]]
name = "analyze_sentiment"
target = "wasm"
wasm_impl = """
function analyze_sentiment(text) {
    const xhr = new XMLHttpRequest();
    xhr.open('POST', 'http://localhost:5000/sentiment', false);
    xhr.send(text);
    return xhr.responseText; // Returns "positive" or "negative"
}
"""
```

---

## 3. Dual-Target Libraries (Universal Apps)

If you want your Codicil app to work as both a Desktop binary and a Web app, provide implementations for both targets in one TOML.

```toml
# Native: Use Rust's standard library
[[functions]]
name = "get_env"
location = "std::env::var"
target = "native"

# Web: Use window.navigator
[[functions]]
name = "get_env"
location = "js_get_lang"
target = "wasm"
wasm_impl = "function js_get_lang() { return navigator.language; }"
```

---

## 4. Using the Library in Your Component

In your `landing.rbv` (or any component):

```brief
<script>
frgn format_date(t: Int) -> Result<String, TimeError> from "./lib/time_utils.toml";

let current_date: String = "";

txn update_time [true][true] {
    &current_date = format_date(1712750000);
    term;
};
</script>

<div>
  <p>The date is: {current_date}</p>
</div>
```

## Summary of Benefits

- **Consistency**: Your Brief logic never changes.
- **Speed**: Use the best tool for the job (Python for AI, JS for UI, Rust for Performance).
- **No Magic**: You can always see exactly how a foreign function is implemented by looking at the TOML.

For technical details on the FFI system, see the [Brief Metropolitan FFI Spec](../../brief-compiler/spec/METROPOLITAN-FFI.md).
