# content/checks/ — Balance Check Bundles

Each subdirectory is a self-contained **check bundle** that can be run with:

```
harness check content/checks/<bundle-name>
```

## Bundle Layout

```
content/checks/<bundle-name>/
├── pack.yaml          # Pack header (id, name, version)
├── commodities.yaml   # Commodity definitions
├── buildings.yaml     # Building definitions
├── recipes.yaml       # Recipe definitions
├── colony.yaml        # Colony layout (buildings + imports)
└── assertions.yaml    # Pass/fail assertions
```

Only `pack.yaml`, `colony.yaml`, and `assertions.yaml` are required.
Content files (`commodities.yaml`, `buildings.yaml`, `recipes.yaml`) are optional
if the pack defines no new types.

## assertions.yaml Schema

```yaml
# List of assertion objects
- commodity: iron_plate      # omit to assert the overall verdict
  expect: closed             # "closed" | "bottleneck" | "impossible"
  min_net: 0.5               # optional: minimum net rate (units/sol), only for expect: closed
  label: iron-plate-surplus  # optional: human-readable label for output
```

### `expect` values

| Value         | Meaning |
|---------------|---------|
| `closed`      | Net rate ≥ 0 (chain is sustainable) |
| `bottleneck`  | Net rate < 0 (deficit present) |
| `impossible`  | Commodity is consumed but never produced |

### Overall verdict assertion

Omit `commodity` to assert the overall `BalanceVerdict`:

```yaml
- expect: closed   # overall chain must close
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0    | All assertions passed |
| 1    | One or more assertions failed |
| 2    | Error (missing files, parse failure) |

## Adding a New Bundle

1. Create `content/checks/<hypothesis-name>/`
2. Add `pack.yaml` with a unique `id`
3. Add commodity/building/recipe YAML files as needed
4. Write `colony.yaml` describing the configuration under test
5. Write `assertions.yaml` with your hypothesis
6. Run `harness check content/checks/<hypothesis-name>` to validate

## Canonical Example

See `content/checks/bootstrap/` — a minimal iron-smelting chain that
demonstrates every field in `assertions.yaml`.

```
harness check content/checks/bootstrap        # human-readable
harness check content/checks/bootstrap --json # structured output
```
