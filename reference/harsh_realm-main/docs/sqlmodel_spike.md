## SQLModel Spike

This document records a narrow evaluation of `SQLModel` after the relational
schema stabilization work for typed persistence.

## Scope

The spike intentionally stays narrow:

- no runtime integration
- no repository rewrites
- no schema migration changes
- no dependency adoption yet

The goal is to answer a smaller question: would `SQLModel` materially simplify
database model definitions for tables that are already normalized and stable?

## Prototype Targets

Two already-normalized tables were used as the comparison target:

1. `character_state`
2. `cell_settlements`

These are good spike candidates because they are active gameplay tables, they
already have stable repository usage, and they represent two common patterns in
this codebase:

- mostly scalar columns plus a few JSON-serialized fields
- scalar columns plus structured JSON sidecar fields used for nested content

## Current Approach

Today the persistence boundary uses:

- explicit SQLite schema in `db_schema.py`
- explicit repositories for queries and writes
- Pydantic models for application-facing typed state

For the two spike targets, the active implementation is centered around:

- [src/harsh_realm/models/entity_state.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/models/entity_state.py)
- [src/harsh_realm/models/cell.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/models/cell.py)
- [src/harsh_realm/gm/entity_state_repository.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/gm/entity_state_repository.py)
- [src/harsh_realm/gm/cell_repository.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/gm/cell_repository.py)

That gives the project:

- clear SQL ownership
- explicit serialization boundaries
- stable repository contracts
- no ORM session lifecycle to manage

## Prototype Sketch

The spike below is illustrative only. It is not wired into runtime code.

```python
from __future__ import annotations

from sqlmodel import Field, SQLModel


class CharacterStateRow(SQLModel, table=True):
    """Prototype SQLModel mapping for the character_state table."""

    __tablename__ = "character_state"

    entity_id: str = Field(primary_key=True)
    character_class: str
    level: int = 1
    xp: int = 0
    xp_next: int = 1500
    hp: int = 0
    max_hp: int = 0
    ac: int = 10
    attack_bonus: int = 0
    physical_save: int = 15
    evasion_save: int = 15
    mental_save: int = 15
    position_q: int = 0
    position_r: int = 0
    attributes_json: str = "{}"
    attr_mods_json: str = "{}"
    skills_json: str = "{}"
    save_bonuses_json: str = "{}"
    equipment_json: str = "[]"
    class_abilities_json: str = "{}"


class CellSettlementRow(SQLModel, table=True):
    """Prototype SQLModel mapping for the cell_settlements table."""

    __tablename__ = "cell_settlements"

    q: int = Field(primary_key=True)
    r: int = Field(primary_key=True)
    name: str
    size: str
    description: str = ""
    settlement_id: str
    starting: int = 0
    establishments_json: str = "[]"
    resident_npc_ids_json: str = "[]"
    town_cells_json: str = "[]"
```

## Comparison

### What SQLModel improves

- one place to describe row shape and primary keys
- built-in SQLAlchemy metadata if the project later wants generated schema tools
- easier to inspect a table definition as a Python class instead of a SQL string

### What SQLModel does not improve enough here

- it does not replace the existing Pydantic application models
- it does not remove the explicit JSON serialization still required for
  `*_json` columns
- it does not remove the need for repository methods, because the project still
  wants explicit persistence boundaries and game-specific query behavior
- it adds another modeling layer on top of existing Pydantic state models
- it adds SQLAlchemy/SQLModel engine and session concepts to a codebase that
  currently uses `aiosqlite` directly and predictably

## Complexity Comparison

### Current repository + Pydantic approach

Advantages:

- already implemented and tested
- no new dependency
- works cleanly with `aiosqlite`
- serialization boundaries are explicit in repository code
- keeps gameplay code decoupled from persistence row classes

Costs:

- table definitions live in SQL instead of Python classes
- some row-to-model mapping is verbose
- schema and row-shape definitions are split across modules

### SQLModel schema/row-definition approach

Advantages:

- row definitions become more discoverable
- primary-key and nullability intent is easier to read in Python
- may reduce some row-shape duplication for simple tables

Costs:

- introduces a new dependency and ORM-adjacent concepts
- still leaves JSON field encoding/decoding in place
- risks a three-layer model stack:
  schema SQL/ORM row model, repository mapping code, and Pydantic domain model
- does not materially simplify the more complex repositories in this codebase

## Decision

`SQLModel` is rejected for the current persistence architecture.

This is an explicit decision, not a defer-without-conclusion.

The reasons are:

1. The current repository + Pydantic approach already matches the project
   architecture and is now stable.
2. The main persistence complexity in Harsh Realm is aggregate mapping and JSON
   sidecar handling, not row-class boilerplate.
3. `SQLModel` would add another abstraction layer without removing the explicit
   repositories or the existing Pydantic models.
4. The project uses `aiosqlite` directly today; moving toward SQLModel would
   likely pull in SQLAlchemy engine/session patterns that do not currently solve
   a pressing problem.

## Adoption Outcome

The project should:

- keep explicit SQL schema definitions
- keep repository modules as the persistence boundary
- keep Pydantic models as the typed application model layer
- avoid adding `SQLModel` unless a future change requires SQLAlchemy metadata
  generation or a broader ORM-based query strategy

## Revisit Trigger

This decision should only be revisited if at least one of these becomes true:

- the project adopts SQLAlchemy for another compelling reason
- schema generation/reflection becomes a persistent maintenance problem
- multiple new normalized tables prove that row-class duplication is now a real
  cost center
- the repository layer is intentionally redesigned around SQLAlchemy sessions
