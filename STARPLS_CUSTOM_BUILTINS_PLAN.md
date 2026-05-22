# Starpls Custom Builtins Plan

This document captures a generic plan for adding custom builtin support to
`starpls`. The goal is to support Starlark runtimes that provide their own
predeclared globals, namespace objects, and runtime-specific value types.

## Goals

- Make a custom `starpls` binary work as a drop-in replacement for the current
  one: editors can keep launching `starpls` the same way.
- Keep the feature generic enough to propose upstream.
- Let projects describe runtime-specific Starlark APIs without editor-specific
  configuration.
- Support explicit Starlark type comments for runtime-specific values.
- Reuse the existing `starpls` builtin model instead of introducing a parallel
  type system.

## Non-goals

- Do not hardcode project-specific paths, runtime names, or file names in
  `starpls`.
- Do not teach `starpls` to parse host-language runtime implementations.
- Do not teach `starpls` to parse external schema languages directly in the
  first version.
- Do not infer application-specific entrypoint signatures automatically.
- Do not require editor-specific settings for projects that commit a local
  manifest.
- Do not implement live schema reload in the first version.

## Desired User Experience

A developer installs a `starpls` binary in the same place where the current
binary is used. When they open `.star` or `.bzl` files that belong to a custom
runtime, the language server discovers repo-local schema files and provides
completions, hover, signature help, and type checking for that runtime's
builtins.

Files use explicit type comments when they want parameter typing:

```python
def render(req):
    # type: (example.Request) -> Unknown
    return req.metadata.name
```

## Repository Contract

A repository provides a small manifest near the relevant Starlark files:

```json
{
  "version": 1,
  "schemas": [
    {
      "name": "example-runtime",
      "include": ["**/*.star"],
      "builtins": "example.starpls.json"
    }
  ]
}
```

The `builtins` file describes globals, namespace objects, functions, and
runtime-specific types:

```json
{
  "global": [
    {
      "name": "make_document",
      "doc": "Creates a mutable document builder.",
      "callable": {
        "params": [],
        "return_type": "example.DocumentBuilder"
      }
    },
    {
      "name": "runtime",
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
                "is_mandatory": true
              }
            ],
            "return_type": "Unknown"
          }
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

The schema shape intentionally stays close to Bazel's existing `Builtins`
model so `starpls` can deserialize it into the same internal representation.

See [Custom Builtins](docs/custom-builtins.md) for the practical manifest and
schema stub guide.

## Accepted MVP Decisions

- Manifest discovery uses the nearest ancestor `.starpls.json`, walking upward
  from the source file being opened, checked, or loaded.
- `include` globs are interpreted relative to the manifest directory.
- If multiple schemas in one manifest match a file, the first matching schema in
  manifest order wins.
- Custom schema identity is the canonical manifest path plus the schema name, so
  different manifests may reuse the same local schema name without colliding.
- The MVP requires an LSP restart after manifest or schema file changes; live
  reload and file watching are deferred.
- The MVP applies custom schemas only to `Dialect::Standard` files. Bazel `.bzl`
  files keep their normal Bazel builtin behavior in the first version.

## Starpls Plan

### 1. Add Custom Builtins JSON Loading

Status: done in `2da269f`.

Add a parser for a JSON representation close to `starpls_bazel::Builtins`.
This should be independent from LSP server startup so it can be unit-tested and
reused by `starpls check`.

Suggested behavior:

- Accept `global` and `type` arrays.
- Accept function callables with params and return types.
- Preserve docs for hover/signature help.
- Reuse the existing internal `Builtins` model rather than introducing a
  parallel type system.

Good first MR shape:

- Add `load_custom_builtins(path) -> anyhow::Result<Builtins>`.
- Add parser tests with a small generic example.

### 2. Add Manifest Discovery

Status: done in `72eb789`.

Add repo-local discovery for `.starpls.json`.

Suggested behavior:

- Search for the nearest `.starpls.json` by walking upward from the opened,
  checked, or loaded source file.
- Interpret `include` globs relative to the manifest directory.
- Resolve `builtins` paths relative to the manifest directory.
- Cache loaded schemas by canonical manifest path and schema name.
- Require an LSP restart after manifest/schema changes in the first version.

This mirrors the existing `starpls` style of discovering repo-local metadata
while making the mechanism generic.

### 3. Route Files To Custom Schemas

Status: done in `bc58381`.

Extend file metadata so a Starlark file can carry a custom schema.

One possible shape:

```rust
pub enum FileInfo {
    Bazel {
        api_context: APIContext,
        is_external: bool,
    },
    Custom {
        schema: String,
    },
}
```

Standard `.star` files should remain `Dialect::Standard`; they should not be
reclassified as Bazel `.bzl` files just to access custom builtins.

For `.bzl` files, custom schemas are deferred. The MVP is limited to Standard
files only, leaving Bazel files on their normal Bazel builtin path.

### 4. Store Custom Builtins Separately

Status: done in `bc58381`.

The existing builtin definitions are keyed by dialect. Add a separate custom
schema store keyed by canonical manifest path plus schema name.

Suggested API shape:

- `set_custom_builtin_defs(schema_id: String, builtins: Builtins)`
- `get_custom_builtin_defs(schema_id: &str) -> Option<BuiltinDefs>`

The resolver should check `FileInfo::Custom { schema }`, where `schema` is this
unique schema id, and expose the matching custom globals/types for that file.

### 5. Make Custom Builtins Visible In Standard Files

Status: done in `bc58381`.

Update name resolution so Standard-dialect files with a custom schema see that
schema's globals.

Expected result:

- `make_document()` resolves as a builtin function.
- `runtime.decode(...)` resolves through a builtin namespace object.
- Hover and signature help use docs and params from the custom schema.

This is the core upstream-worthy behavior: custom runtime environments can
describe their predeclared globals without editor-specific configuration.

### 6. Support Dotted Builtin Type Names

Status: done in `bc58381`.

Custom schemas should be able to name types like `example.Request`. Type
comments and schema return types should resolve the same name.

Example:

```python
def render(req):
    # type: (example.Request) -> Unknown
    return req.metadata.name
```

Implementation note:

- `TypeRef::Path(["example", "Request"])` should be able to resolve against a
  builtin type named `example.Request`.
- This is useful because custom Starlark runtimes often namespace their API
  types.

### 7. Make `starpls check` Use The Same Mechanism

Status: done in `bc58381`.

The CLI checker should load the same manifests and schemas as the LSP server.
Otherwise diagnostics in the editor and in `starpls check` will disagree.

### 8. Add Upstream-friendly Tests

Status: done. Core coverage landed in `bc58381`; the current working tree adds
the remaining IDE-facing completion and signature help coverage.

Use a small generic fixture such as `example.starpls.json`.

Coverage targets:

- [x] Manifest discovery and include matching.
- [x] Custom global visible in a `.star` file.
- [x] Namespace object completion, for example `runtime.` -> `decode`.
- [x] Function signature help for custom builtin functions.
- [x] Type comment with a dotted custom type.
- [x] Field completion through a custom type, for example `req.` -> `metadata`.
- [x] Negative case where a `.star` file outside `include` does not receive the
  custom schema.

## Proposed Upstream MR Split

1. Custom builtins JSON loader and parser tests.
2. `.starpls.json` discovery and include routing.
3. Custom builtins visible in Standard-dialect files.
4. Dotted custom builtin type resolution.
5. Shared loading path for LSP server and `starpls check`.

This keeps each MR understandable and avoids presenting any particular custom
runtime as part of the generic feature.

## Project Follow-up Plan

After the generic `starpls` support exists, each project can generate or
maintain its own schema.

Common inputs:

- Runtime API declarations from the host-language implementation.
- Existing API documentation metadata.
- External schemas such as protobuf, JSON schema, OpenAPI, or a hand-written
  runtime schema.
- A small hand-written or generated `.starpls.json` manifest for the relevant
  Starlark file tree.

Common generated outputs:

- `.starpls.json`
- `<runtime>.starpls.json`

Common type mapping strategy:

- string-like schema values -> `string`
- boolean schema values -> `bool`
- numeric schema values -> `int` or `float`
- enum-like schema values -> `string` or a custom enum-like type, depending on
  runtime behavior
- object/message/schema records -> generated custom types
- repeated/array fields -> `list[T]`
- map/object dictionary fields -> `dict[key, value]`

Starlark changes can be incremental. Entry points and helpers can receive
explicit type comments where they improve completion and type checking:

```python
def render(req):
    # type: (example.Request) -> Unknown
```

## Open Questions

- Should custom schemas support inheritance or imports in a later version?
- Should docs accept Markdown in `doc` fields exactly like Bazel builtin docs,
  or should schema docs be plain text initially?

## MVP Definition

The first useful end-to-end version is complete when:

- A `.starpls.json` manifest is discovered without editor configuration.
- A matched `.star` file receives custom globals from a schema JSON file.
- A namespace object and a builtin function resolve from the schema.
- A type comment using a dotted custom type resolves to a custom type.
- Completion works for at least one nested field chain under a typed parameter.
- The same fixture passes in both editor-oriented tests and `starpls check`.
