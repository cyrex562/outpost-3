# Plugin Host, Sandbox & SDK (HR-403)

> Status: **Implemented** (HR-403). Builds on the protocol from HR-402
> (`docs/design/2026-06-21-plugin-ipc-protocol.md`).
> Code: `src/harsh_realm/plugins/{host,sandbox,transport,sdk}.py` +
> `examples/example_plugin.py`.

HR-402 defined the wire contract. HR-403 is the runtime that uses it: a host that
spawns and supervises a plugin process, enforces hard timeouts, surfaces failures
as typed exceptions, and an author SDK for writing plugins. This lets
*untrusted or rapidly-iterated* Python hooks (`compute` / `effect` /
`procedure_step`) run out-of-process where a crash, hang, or runaway can be
contained — unlike the trusted in-process `packs/<id>/code/` hooks.

## Components

| Module | Role |
|---|---|
| `plugins.host.PluginHost` | Spawn/supervise one plugin; `invoke()` with per-call timeouts; route results/errors/logs; reap on shutdown. |
| `plugins.sandbox.SandboxConfig` | Scrubbed environment, working dir, POSIX rlimits. |
| `plugins.transport` | `LineTransport` (NDJSON bytes + process controls): subprocess, in-memory (tests), and plugin-side stdio. |
| `plugins.sdk.PluginApp` | Author-facing: register hooks, run the protocol loop. |
| `plugins.examples.example_plugin` | A runnable reference plugin. |

## Writing a plugin

```python
from harsh_realm.plugins.sdk import PluginApp

app = PluginApp(name="my-pack-plugin", version="1.0.0")

@app.compute("my-pack:compute.fancy")
def fancy(params):                       # params: dict[str, JsonValue]
    return params["a"] + params["b"]

@app.effect("my-pack:effect.bleed")
def bleed(params, context):              # context: InvocationContext | None
    return [{"intent": "change_resource", "entity_id": context.self_id,
             "resource": "hp", "delta": -params["amount"]}]

if __name__ == "__main__":
    app.run_stdio()
```

- Hooks may be `def` or `async def`. A one-arg hook receives `params`; a two-arg
  hook also receives the invocation `context`.
- `compute` / `procedure_step` return any JSON value; `effect` returns a **list of
  intent objects** (the host applies them — the plugin never writes to the world).
- Hook exceptions are caught and returned to the host as `error{hook_raised}`
  with a traceback in `data`. **Log via the protocol** (future SDK helper) or
  stderr — never `print()` to stdout, which carries protocol frames.

## Running a plugin from the host

```python
from harsh_realm.plugins import PluginHost, SandboxConfig
import sys

host = await PluginHost.spawn(
    [sys.executable, "-m", "harsh_realm.plugins.examples.example_plugin"],
    sandbox=SandboxConfig(cpu_seconds=5),   # POSIX rlimit; ignored on Windows
    default_timeout=10.0,
)
try:
    value = await host.invoke("example:compute.sum", "compute", {"numbers": [1, 2, 3]})
    intents = await host.invoke("example:effect.bleed", "effect", {"amount": 2},
                                context=InvocationContext(self_id="hero"))
finally:
    await host.shutdown()
```

`spawn()` performs the handshake and fails with `PluginStartError` if the plugin
doesn't become ready (bad version, crash on startup, or timeout). After it
returns, `host.capabilities` lists what the plugin advertised.

## Lifecycle & supervision

- **Handshake:** host sends `hello`; plugin replies `ready` (version +
  capabilities) or the host raises `PluginStartError`.
- **Invocation:** each `invoke()` gets a unique id and is awaited up to
  `timeout` (or `default_timeout`). Concurrent invokes are multiplexed by id.
- **Timeout:** a hung hook can't be interrupted in-band, so on timeout the host
  **kills the process** and raises `PluginTimeoutError`; subsequent calls raise
  `PluginCrashedError`. Restart is a caller policy (`spawn` again).
- **Crash:** if the process exits with work pending, those invokes raise
  `PluginCrashedError` (carrying a tail of the plugin's stderr).
- **Shutdown:** `host.shutdown()` sends `shutdown`, waits up to a grace period,
  then kills; it always joins the reader task and reaps the child. It is safe to
  call after a timeout/crash.

### Failure → exception map (all subclass `PluginError`)

| Situation | Exception |
|---|---|
| Spawn / handshake failure | `PluginStartError` |
| Plugin returned `error{...}` | `PluginInvocationError` (`.code`, `.data`) |
| Deadline exceeded (process killed) | `PluginTimeoutError` |
| Process exited unexpectedly | `PluginCrashedError` |
| Malformed frame on the wire | `PluginProtocolError` |

## Sandbox: what it does and does not do

`SandboxConfig` provides **process isolation and resource hints**, not a hard
security boundary:

- **Environment scrub** (default): only an allow-listed set of variables passes
  to the child (`build_env`), so host secrets aren't inherited. `extra_env` adds
  overrides; `inherit_env=True` opts out of the scrub.
- **Working directory** via `cwd`.
- **POSIX rlimits** (`cpu_seconds`, `memory_bytes`) applied with a `preexec_fn`.
  **Ignored on Windows** (no `resource`/`preexec_fn`).

It does **not** provide kernel confinement (namespaces, seccomp, Windows job
objects/restricted tokens). For genuinely hostile code, run the host's child
inside an OS-level sandbox (container, restricted user). Combined with the host's
hard timeouts and crash isolation, the current posture is sufficient for *buggy
or runaway* trusted-but-fallible plugins — the intended use.

## Testing

- `tests/plugins/test_sandbox.py` — env scrub / rlimits.
- `tests/plugins/test_runtime.py` — host ↔ SDK over an in-memory transport
  (handshake, compute/effect, error surfacing, wrong-kind, version mismatch,
  timeout, crash-on-EOF) — deterministic, no subprocess.
- `tests/plugins/test_subprocess.py` — the reference plugin as a real child
  process, including genuine timeout-kill and hard-crash recovery.

## Not in scope (future)

- Wiring out-of-process plugins into the live `ComputeRegistry` / procedure
  runner / trigger effect path (a `PluginComputeBackend` that satisfies the same
  call sites as in-process hooks). The protocol + host make this a thin adapter.
- A pack-manifest declaration for out-of-process plugin entry points.
- Structured `log` helper on the SDK and a host log sink.
- OS-level sandbox integration.
