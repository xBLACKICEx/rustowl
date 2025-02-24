# owlsp specification

owlsp, or cargo-owlsp, is an LSP server which provides RustOwl information.
To display various types of decorations, owlsp supports some custom methods from the client.

Here, we describe the specifications of those custom methods.

## Types

Here, we describe the types we will use in this document.

### `OprType`

```typescript
"lifetime" | "imm_borrow" | "mut_borrow" | "move" | "call" | "outlive"
```

### `Decoration`

<pre><code>
{
    "type": OprType,
    "range": <a href="https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#range">Range</a>,
    "hover_text": Option&lt;String&gt;,
    "is_display": bool
}
</code></pre>

## Methods

We describe the custom methods used in owlsp.

### `rustowl/cursor`

#### Request payload

<pre><code>
{
    "position": <a href="https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#position">Position</a>,
    "document": {
        "uri": <a href="https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#documentUri">DocumentUri</a>
    }
}
</code></pre>

#### Response payload

<pre><code>
{
    "decorations": [Decoration]
}
</code></pre>
