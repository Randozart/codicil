# Brief Language Guide for LLMs

Brief is a declarative language for buildable, verifiable state machines. As an LLM, you are likely not trained on Brief. This guide provides the necessary context, syntax, and semantics to write correct Brief code.

---

## 1. Core Philosophy
- **Readable State Machine**: Code should describe state transitions clearly.
- **Verifiable Contracts**: Every transaction has a precondition (when it can run) and a postcondition (what it guarantees).
- **Explicit Initialization**: ALL values must be initialized. No hidden or uninitialized state.
- **Single Source of Truth**: State is data; UI is a reflection of that data.

---

## 2. File Types
- **`.bv` (Brief)**: Logic-only files. Used for server routes or CLI tools.
- **`.rbv` (Rendered Brief)**: Component files containing a `<script>` block (Brief logic) and a `<view>` block (HTML with reactive bindings).

---

## 3. Basic Syntax

### Comments
- Use `//` for single-line comments.
- **NEVER** use `#` for comments.

### State Declaration
```brief
let name: String = "Brief";
let count: Int = 0;
let items: List<String> = [];
```
- Every variable MUST have a type and an initial value.
- Types: `Int`, `Float`, `String`, `Bool`, `List<T>`, `Set<T>`, `Map<K, V>`, and custom `struct` names.

### Mutation
- Mutations ONLY happen inside transactions.
- Use the `&` prefix to write to a variable.
- Reading a variable does NOT use a prefix.
```brief
&count = count + 1; // Correct
count = count + 1;  // ERROR
```

---

## 4. Transactions & Contracts

### Structure
```brief
txn name [precondition][postcondition] {
    // Body
    term;
};
```
- `[precondition]`: Boolean expression. If false, the transaction will not execute.
- `[postcondition]`: Assertion checked after execution. Use `@var` to refer to the value of `var` *before* the transaction started.
- `term;`: Marks successful completion and commits changes.
- `escape;`: Aborts and rolls back all changes.

### Reactive Transactions (`rct txn`)
Auto-fire whenever their precondition is met.
```brief
rct txn tick @60Hz [count > 0][count == @count - 1] {
    &count = count - 1;
    term;
};
```
- `@Hz`: Optional timing (e.g., `@1Hz`, `@60Hz`). Default is 10Hz.
- Global timing can be set at the top of the file: `reactor @60Hz;`.

---

## 5. Structs and Containers

### Struct Definitions
Structs require `let` for fields and trailing semicolons.
```brief
struct User {
    let id: Int = 0;
    let username: String = "anonymous";
};
```

### Container Shorthand
When the type is known (e.g., `List<User>`), you can use shorthand object literals. Missing fields are filled from the struct's defaults.
```brief
let users: List<User> = [
    { username: "alice" }, // id defaults to 0
    { id: 1, username: "bob" },
    {} // both default
];
```

---

## 6. Rendered Brief (.rbv)

### The `<script>` Block
Contains declarations, transactions, and `rstruct` definitions.

### The `rstruct` (Render Struct)
A self-contained stateful component.
```brief
rstruct Counter {
    let val: Int = 0;
    
    txn increment [true][val == @val + 1] {
        &val = val + 1;
        term;
    };
    
    <div>
        <span b-text="val">0</span>
        <button b-trigger:click="increment">+</button>
    </div>
}; // NOTE: trailing semicolon required
```

### Reactive Directives
- `b-text="var"`: Updates element text content. Supports property access: `b-text="item.name"`.
- `b-trigger:event="txn"`: Binds DOM events to transactions.
- `b-show="expr"` / `b-hide="expr"`: Toggles visibility.
- `b-each:item="list"`: Iterates over a list.

### Universal Property Access
Directives support recursive property access on iteration items:
```html
<ul b-each:item="users">
    <li b-show="item.active">
        <span b-text="item.profile.name">Name</span>
    </li>
</ul>
```

---

## 7. Hallucination Prevention Checklist

1. **Braces**: Always use single braces `{ }` for blocks and interpolation. NEVER double braces `{{ }}`.
2. **Semicolons**: 
   - After `let` declarations.
   - After `term;` and `escape;`.
   - After `struct { ... };` and `rstruct { ... };`.
3. **Mutation**: Always use `&var = ...` for assignment.
4. **Comments**: Use `//`, not `#`.
5. **Types**: Capitalize basic types: `Int`, `String`, `Bool` (not `int`, `string`, `boolean`).
6. **Imports**: Path must be a quoted string: `import "./file.css";`.
7. **rstruct name**: Use short names in triggers. Inside `Counter`, use `b-trigger:click="increment"`, not `b-trigger:click="Counter.increment"`.

---

## 8. Quick Workflow for LLMs
1. Define your data `struct`s with defaults.
2. Declare your state `let` variables.
3. Write your `txn` and `rct txn` logic with strict contracts.
4. If writing UI, add the `<view>` block using `b-` directives.
5. Use `brief run file.rbv --no-cache` to test.
