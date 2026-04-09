# Brief Language Reference

Brief is a declarative language for building verifiable state machines with contracts. Used by Codicil for web framework logic.

## File Types

| Extension | Description | Context |
|-----------|-------------|---------|
| `.bv` | Pure Brief | Server-side, CLI tools, Codicil routes |
| `.rbv` | Rendered Brief | Client-side, HTML + bindings |

## Comments

```brief
// Single-line comment
let x: Int = 1;  // Inline comment
```

Note: Brief uses `//` for comments. The `#` comment style was removed.

## State & Mutation

```brief
let counter: Int = 0;
let name: String = "Codicil";
let items: List<String> = ["a", "b"];

&counter = counter + 1;  // Mutation requires &
```

## Transactions

```brief
txn increment [count < 100][count == @count + 1] {
    &count = count + 1;
    term;
};
```

- `[pre]` - Precondition: when transaction can fire
- `[post]` - Postcondition: guaranteed after `term`
- `@x` - Value of x at transaction start (prior state)
- `term;` - Success (satisfies postcondition)
- `term value;` - Return a value
- `escape;` - Rollback

Note: Use single braces `{ }` - not double braces `{{ }}`.

## Reactive Transactions

```brief
rct txn auto_increment [count < 100][count == @count + 1] { ... };
```

Fires automatically when preconditions are met.

## Rendered Brief (.rbv)

Three blocks:

```html
<script>
let clicks: Int = 0;

txn add [clicks >= 0][clicks == @clicks + 1] {
    &clicks = clicks + 1;
    term;
};

rstruct Counter {
    count: Int;
    txn tick [count > 0][count == @count - 1] { ... };
    <div><span b-text="count">0</span></div>
}

import "./styles.css";
</script>

<view>
    <button b-trigger:click="add">Click: <span b-text="clicks">0</span></button>
    <Counter />
</view>
```

Note: For `.rbv` files, use `<script>` - not `<script type="brief">`.

## CSS Import

```brief
import "./styles.css";
```

The CSS file content is extracted and injected into the compiled output. Works with both inline `<style>` blocks and imported CSS files.

## Bindings

| Directive | Example | Purpose |
|-----------|---------|---------|
| `b-text` | `<span b-text="count">0</span>` | Reactive text |
| `b-trigger:click` | `<button b-trigger:click="add">` | Click handler |
| `b-each:item` | `<div b-each:item="items">` | List iteration |
| `b-show` | `<span b-show="visible">` | Conditional display |

## rstruct (Render Struct)

Combines state with inline HTML view. All fields MUST have initializers.

```brief
rstruct Counter {
    let count: Int = 0;           // REQUIRED - must have default value
    
    txn tick [count > 0][count == @count - 1] {  // "tick" auto-expands to "Counter.tick"
        &count = count - 1;
        term;
    };
    
    <div><span b-text="count">0</span></div>
};
```

### Reactive Transactions

Use `rct txn` for reactive transactions that auto-run:

```brief
rstruct Timer {
    let value: Int = 10;
    rct txn decrement [value > 0][value == @value - 1] {
        &value = value - 1;
        term;
    };
    
    <span b-text="value">10</span>
};
```

### Key Points
- Fields use `let name: Type = value;` syntax (required initializer)
- Transactions inside rstruct can use short name `txn tick` (auto-expands to `rstructname.tick`)
- Reactive transactions use `rct txn` prefix

## Codicil Routes (.bv)

Codicil routes return **data** (e.g., component name), not HTML. The HTML comes from compiled `.rbv` components.

```brief
[route]
method = "GET"
path = "/"

[post]
response.status == 200

txn handle [true][post] {
    term "landing";
};
```

The response body (`"landing"`) tells Codicil which component to render.

## Quick Reference

- `let x: T = v;` - State declaration
- `&x = v` - Write access
- `@x` - Prior state in transaction
- `txn name [pre][post] { }` - Transaction (single braces!)
- `rct txn` - Reactive transaction
- `term;` - Success
- `term value;` - Return value
- `escape;` - Rollback
- `// comment` - Comment (not `#`)
- `b-text`, `b-trigger`, `b-each`, `b-show` - Bindings
- `import "./file.css";` - Import CSS (injected into output)