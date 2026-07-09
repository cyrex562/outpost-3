# Plugin IPC Protocol (HR-402)

> Status: **Defined** (HR-402). Runtime/sandbox/reference plugin: **HR-403**.
> Contract code: `src/harsh_realm/plugins/protocol.py` + `codec.py`.
> Relates to: `docs/design/rust-core-migration-plan.md` §4.2 (intent boundary),
> §7 R7 (plugin surface).

## 1. Purpose

Harsh Realm runs game logic in two tiers:

- **In-process hooks** — `packs/<id>/code/__init__.py` registers Python callables
  into the `ComputeRegistry` (see `src/harsh_realm/procedures/compute_registry.py`).
  These are *trusted*, fast and synchronous, but cannot be sandboxed, timed out,
  or hot-reloaded; a crash or infinite loop takes down the engine.

- **Out-of-process plugins** — *untrusted or rapidly-iterated* Python that
  supplies the same kinds of hooks, but runs in a child process the host can
  kill, time out, and reload. This document defines the protocol that child
  speaks; it is the durable contract referenced by migration-plan task **R7**.

The same three hook categories are reachable either way, invoked **by qualified
name** (e.g. `my-pack:compute.fancy_roll`):

| Kind | Today (in-proc) | Result semantics |
|---|---|---|
| `compute` | procedure `compute` step → `ComputeRegistry.invoke` | a `JsonValue` |
| `effect` | declarative trigger effect (lowered to intents) | a list of **intent** objects |
| `procedure_step` | a custom step kind beyond roll/compute/procedure/format | a `JsonValue` (the step's assigned output) |

The protocol is JSON-only — the same boundary the engine already uses for
intents — so a **Rust** host or the **Python/FastAPI** host can speak it
unchanged.

## 2. Transport & framing

- **Newline-delimited JSON (NDJSON).** Each message is one compact JSON object on
  its own line, terminated by `\n`. JSON escapes any interior newline in string
  values, so *one line is always exactly one message*.
- **Streams** (the host owns the child's stdio):
  - host → plugin: the plugin's **stdin**.
  - plugin → host: the plugin's **stdout**.
  - the plugin's **stderr** is out of band (crash output / unstructured logs);
    it is **not** part of the protocol. Structured diagnostics use `log` messages.
- **Encoding:** UTF-8.
- **Alternative considered:** length-prefixed framing (`<u32 len><bytes>`). Not
  adopted — these are small control messages and NDJSON is simpler to implement,
  debug (human-readable), and test. The framing is an implementation detail of
  `codec.py` and may change without touching message shapes.

## 3. Message types (tagged union)

Every message is an object with a `type` discriminator. Concrete shapes live in
`protocol.py` as frozen Pydantic models; the union is `ProtocolMessage`.

### Host → plugin

- **`hello`** — handshake opener.
  `{ "type":"hello", "protocol_versions":[1], "host":{"name","version"} }`
- **`invoke`** — run one hook.
  `{ "type":"invoke", "id":"r1", "qualified_name":"pack:fn", "kind":"compute",
     "params":{…}, "context":{…}|null, "deadline_ms":2000|null }`
- **`shutdown`** — finish in-flight work and exit.
  `{ "type":"shutdown", "reason":"" }`

### Plugin → host

- **`ready`** — handshake reply; commits to a version and lists hooks.
  `{ "type":"ready", "protocol_version":1, "plugin":{"name","version"},
     "capabilities":[{"qualified_name","kind","description"}] }`
- **`result`** — successful invocation (echoes the request `id`).
  `{ "type":"result", "id":"r1", "value":<kind-specific> }`
- **`error`** — failed invocation (`id` set) or connection-level error (`id` null).
  `{ "type":"error", "id":"r1"|null, "code":<ErrorCode>, "message":"…",
     "data":{…}|null }`
- **`log`** — out-of-band diagnostic; never a reply.
  `{ "type":"log", "level":"info", "message":"…", "id":"r1"|null, "fields":{…} }`

### Result `value` by kind

- `compute` / `procedure_step`: the computed `JsonValue` (the value the step
  assigns).
- `effect`: a JSON **list of intent objects**, each shaped like the intents the
  host already applies through its `IntentSink`
  (`src/harsh_realm/triggers/runner.py`), e.g.
  `{"intent":"change_resource","entity_id":"e1","resource":"hp","delta":-3}`.
  The plugin never writes to the world; it only describes what should happen, and
  the host applies (or rejects) the intents — preserving the engine's
  pure-core / host-applies-intents seam.

## 4. Lifecycle

```
host                                   plugin
 │  spawn child, wire stdio             │
 │ ───────────── hello ───────────────▶ │  negotiate_version(hello.protocol_versions)
 │ ◀───────────── ready ─────────────── │  (or error{version_unsupported} then exit)
 │                                      │
 │ ───────────── invoke r1 ───────────▶ │  look up capability, run hook
 │ ◀──── log (optional, id=r1) ──────── │
 │ ◀──────── result r1 / error r1 ───── │
 │            … many invokes …          │
 │ ───────────── shutdown ────────────▶ │  drain, flush, exit 0
 │  wait(); on timeout, kill (HR-403)   │
```

- **Handshake.** The host sends `hello` first. The plugin replies `ready` with
  the negotiated `protocol_version` and its capability list, or replies `error`
  with `version_unsupported` and exits. `negotiate_version()` picks the highest
  version both sides list.
- **Invocation.** Each `invoke` carries a unique `id`; the matching `result` or
  `error` echoes it. The host MAY have several invocations in flight; correlation
  is by `id`. A plugin MAY process invocations concurrently or serially.
- **Validation the plugin owns:** unknown `qualified_name` → `error
  unknown_capability`; `kind` disagreeing with the advertised capability →
  `wrong_kind`; bad params → `invalid_params`; hook raised → `hook_raised` (with a
  traceback in `data` when available).
- **Shutdown.** On `shutdown` the plugin finishes in-flight work, flushes, and
  exits 0. Hard kill on timeout is the host's job (**HR-403**).

## 5. Versioning

- `PROTOCOL_VERSION = 1`; `SUPPORTED_PROTOCOL_VERSIONS = (1,)`.
- Bump `PROTOCOL_VERSION` on any **breaking** change to message shapes. Additive,
  optional fields are backward compatible and do **not** require a bump.
- Both sides advertise the versions they support; the session uses the highest in
  common. No overlap ⇒ the plugin reports `version_unsupported` and exits.

## 6. Error taxonomy (`ErrorCode`)

`protocol_error`, `version_unsupported`, `unknown_capability`, `wrong_kind`,
`invalid_params`, `hook_raised`, `timeout`, `cancelled`, `internal`.

`message` is human-facing; `data` MAY carry structured detail (e.g. a `traceback`
string, the offending param name). `timeout`/`cancelled` are produced by the
**host** when it abandons an invocation (enforcement is HR-403); a well-behaved
plugin does not emit them.

## 7. Security & isolation (deferred to HR-403)

This document defines only the *contract*. The host runtime (HR-403) owns:
process spawning and supervision, hard timeouts and cancellation, resource
limits, restricting filesystem/network, capability allow-listing per world, and
recovering from a crashed plugin. Nothing in this protocol grants a plugin
ambient authority — it can only return values/intents that the host chooses to
apply.

## 8. Worked example (compute hook)

Re-expressing the existing in-process hook `xwn-core:disposition_from_chaos`
(`packs/xwn-core/code/__init__.py`) as an out-of-process invocation:

```
→ {"type":"invoke","id":"a7","qualified_name":"xwn-core:disposition_from_chaos","kind":"compute","params":{"chaos_factor":7}}
← {"type":"result","id":"a7","value":-1}
```

An effect hook returning intents:

```
→ {"type":"invoke","id":"b2","qualified_name":"my-pack:effect.bleed","kind":"effect","params":{"amount":3},"context":{"self_id":"e1","event_type":"combat.attack"}}
← {"type":"result","id":"b2","value":[{"intent":"change_resource","entity_id":"e1","resource":"hp","delta":-3}]}
```

## 9. What HR-402 ships

- `src/harsh_realm/plugins/protocol.py` — message models, `HookKind`, `ErrorCode`,
  `PROTOCOL_VERSION`, `negotiate_version()`.
- `src/harsh_realm/plugins/codec.py` — `encode_message`/`decode_message`/
  `iter_messages` (pure NDJSON, no process I/O).
- `harsh_realm.exceptions.PluginProtocolError`.
- Tests: `tests/plugins/test_protocol.py`, `tests/plugins/test_codec.py`, plus
  round-trip property tests in `tests/test_properties.py`.

HR-403 will add the process host (spawn/supervise/timeout), the sandbox, a
reference plugin, and author-facing docs.
