# Python Typing Policy

This document defines the project's strict typing policy for Python code.

The goal is to reduce runtime errors by making invalid states and incorrect
calls visible to static analysis before code is executed.

## Core Rule

Do not use bare `object` as a type to avoid writing a real type.

Do not use `Any` as a type to avoid writing a real type.

Both are allowed only at narrow, intentional boundaries where the code is
genuinely dynamic and the boundary is immediately narrowed, validated, or
adapted.

## Why

`object` and `Any` hide different classes of problems:

- `object` throws away useful structure and forces unchecked casts or
  attribute access later
- `Any` disables static analysis entirely and lets incorrect operations pass
  type checking

If the project is trying to become more reliable under Ruff, Pylance, Pyright,
and Mypy, both need to be treated as escape hatches, not defaults.

## Allowed Uses

These are acceptable only when documented and kept local:

1. JSON-serializable payload aliases
   Use the project's JSON aliases such as `JsonValue` and `JsonObject`, not
   `dict[str, object]`.

2. Third-party library boundaries
   If a library returns weakly typed data, keep `Any` at the boundary and
   immediately convert it into a typed model, protocol, or validated structure.

3. Framework state objects with no stable upstream type
   If a FastAPI `app.state` object or similar runtime object must be accessed,
   prefer a `Protocol` describing the fields actually used.

4. Validation hooks that truly accept arbitrary input
   Pydantic validators may accept `Any` only when they are normalizing unknown
   input into a typed result.

5. Generic container internals
   Extremely local implementation details may use `object` if they are part of a
   generic algorithm and never exposed as an application-facing type.

## Forbidden Uses

These patterns should be treated as violations:

- `dict[str, object]` where a payload model or `JsonObject` should exist
- `list[object]` for heterogeneous runtime data that should be a union or model
- function parameters typed as `object` just to accept "anything"
- return types typed as `object`
- `Any` for repositories, controllers, scenes, event buses, narrators, app
  state, or domain models when a `Protocol` or concrete type is possible
- `**kwargs: Any` for stable update shapes that should be explicit fields or
  typed patch models

## Preferred Replacements

Use the narrowest real type available.

Instead of `object` or `Any`, prefer:

- a Pydantic model for structured payloads
- `JsonValue` / `JsonObject` for JSON-shaped data
- a `Protocol` for service interfaces or framework-owned state
- a `TypeVar` or generic type parameter for reusable algorithms
- a concrete union such as `DungeonRoom | dict[str, JsonValue]` when a migration
  boundary is temporary
- `Path`, `str`, `URL`, `Callable[...]`, or other specific built-in/library
  types when the shape is known

## Review Standard

When adding or reviewing Python code:

1. Treat new `object` and `Any` annotations as suspect by default.
2. Ask whether the value has a stable structure that can be modeled.
3. Ask whether a `Protocol` would express the dependency more honestly.
4. Keep dynamic framework/library boundaries as small as possible.
5. If `Any` or `object` remains, document why it is unavoidable at that point.

## Rust Boundary

Rust should not be used as a substitute for weak Python typing.

Introduce Rust only when:

- there is a performance-critical or safety-critical subsystem that benefits
  from a narrower FFI/API boundary, and
- the Python side already has a typed contract for the boundary

The first response to weak typing in Python should be better Python types, not a
language switch.

## Static Gates

The current typed runtime allowlist is enforced through:

- `ruff check` on the curated runtime file set
- `mypy` in `--strict` mode through `pyproject.toml`
- `pyrightconfig.json` with `typeCheckingMode: "strict"` for the same file set

Use:

```bash
scripts/check_runtime_typing.sh
```

This gate is intentionally scoped to the runtime modules already tightened in
the current typing pass. As more files are cleaned up, they should be added to
the same allowlist instead of weakening the checker.
