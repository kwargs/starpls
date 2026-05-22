# Custom Builtins

Custom builtins let a non-Bazel Starlark runtime describe its predeclared
globals, functions, namespace objects, and runtime-specific types to `starpls`.
This is for `Dialect::Standard` files such as `.star` and `.sky`; Bazel `.bzl`
files keep the normal Bazel builtins.

## Files To Add

For a repository where all custom Starlark files share one runtime, add these
two files at the repository root:

```text
<repo>/.starpls.json
<repo>/example.starpls.json
```

For a repository where only a subtree uses the runtime, put the manifest at the
nearest common parent:

```text
<repo>/rendering/.starpls.json
<repo>/rendering/rendering.starpls.json
<repo>/rendering/**/*.star
```

For multiple runtimes in one repository, use one manifest with multiple schemas:

```text
<repo>/.starpls.json
<repo>/schemas/rendering.starpls.json
<repo>/schemas/workflow.starpls.json
<repo>/rendering/**/*.star
<repo>/workflows/**/*.sky
```

`starpls` walks upward from the opened, checked, or loaded source file and uses
the nearest `.starpls.json`. `include` globs and `builtins` paths are relative
to that manifest. If multiple schemas match, the first one wins. Restart the
language server after changing the manifest or schema file.

## Manifest

Create `.starpls.json`:

```json
{
  "version": 1,
  "schemas": [
    {
      "name": "example-runtime",
      "include": ["**/*.star", "**/*.sky"],
      "builtins": "example.starpls.json"
    }
  ]
}
```

Subtree example:

```json
{
  "version": 1,
  "schemas": [
    {
      "name": "rendering",
      "include": ["templates/**/*.star"],
      "builtins": "rendering.starpls.json"
    }
  ]
}
```

Multi-runtime example:

```json
{
  "version": 1,
  "schemas": [
    {
      "name": "rendering",
      "include": ["rendering/**/*.star"],
      "builtins": "schemas/rendering.starpls.json"
    },
    {
      "name": "workflow",
      "include": ["workflows/**/*.sky"],
      "builtins": "schemas/workflow.starpls.json"
    }
  ]
}
```

## Builtins Stub

Create the file referenced by `builtins`, for example
`example.starpls.json`:

```json
{
  "global": [
    {
      "name": "make_document",
      "doc": "Creates a document.",
      "callable": {
        "params": [
          {
            "name": "title",
            "type": "string",
            "doc": "Document title.",
            "is_mandatory": true
          }
        ],
        "return_type": "example.Document"
      }
    },
    {
      "name": "runtime",
      "doc": "Runtime helper namespace.",
      "type": "example.runtime"
    }
  ],
  "type": [
    {
      "name": "example.runtime",
      "field": [
        {
          "name": "decode",
          "doc": "Decodes a serialized payload.",
          "callable": {
            "params": [
              {
                "name": "payload",
                "type": "string",
                "doc": "Serialized payload.",
                "is_mandatory": true
              }
            ],
            "return_type": "example.Document"
          }
        }
      ]
    },
    {
      "name": "example.Document",
      "doc": "Rendered document.",
      "field": [
        {
          "name": "title",
          "type": "string",
          "doc": "Document title."
        }
      ]
    },
    {
      "name": "example.Request",
      "field": [
        {
          "name": "metadata",
          "type": "example.Metadata"
        }
      ]
    },
    {
      "name": "example.Metadata",
      "field": [
        {
          "name": "name",
          "type": "string"
        }
      ]
    }
  ]
}
```

## Stub Patterns

Global value:

```json
{
  "name": "runtime",
  "type": "example.runtime",
  "doc": "Runtime helper namespace."
}
```

Global function:

```json
{
  "name": "make_document",
  "doc": "Creates a document.",
  "callable": {
    "params": [
      {
        "name": "title",
        "type": "string",
        "doc": "Document title.",
        "is_mandatory": true
      }
    ],
    "return_type": "example.Document"
  }
}
```

Optional parameter with a default:

```json
{
  "name": "indent",
  "type": "int",
  "default_value": "2",
  "doc": "Number of spaces."
}
```

Variadic parameters:

```json
[
  {
    "name": "items",
    "type": "list of string",
    "is_star_arg": true
  },
  {
    "name": "options",
    "type": "dict of string",
    "is_star_star_arg": true
  }
]
```

Namespace object with methods:

```json
{
  "name": "example.runtime",
  "field": [
    {
      "name": "decode",
      "callable": {
        "params": [
          {
            "name": "payload",
            "type": "string",
            "is_mandatory": true
          }
        ],
        "return_type": "example.Document"
      }
    }
  ]
}
```

Plain object type with fields:

```json
{
  "name": "example.Request",
  "field": [
    {
      "name": "metadata",
      "type": "example.Metadata"
    }
  ]
}
```

Nested field type:

```json
{
  "name": "example.Metadata",
  "field": [
    {
      "name": "name",
      "type": "string"
    }
  ]
}
```

## Type Comments In Source Files

Use PEP 484-style type comments when a runtime passes values into your entry
points and `starpls` cannot infer their types from local code:

```python
def render(req):
    # type: (example.Request) -> Unknown
    return req.metadata.name
```

Custom dotted type names in comments, such as `example.Request`, resolve to
matching names in the schema's `type` array.

## Type Names

Schema files use the same builtin type-string parser as Bazel builtins. Use
plain names for primitives and dotted custom types; use `list of ...` or
`dict of ...` for collections.

Common type strings:

```text
Unknown
None
bool
int
float
string
bytes
list of string
dict of example.Document
example.Request
```

Prefer `Unknown` when the runtime accepts or returns an unconstrained value.

## Quick Check

After adding the manifest and schema:

```sh
starpls check path/to/file.star
```

For editor use, restart the language server after schema changes. If using the
local development binary from this repository:

```sh
make install-starpls
```
