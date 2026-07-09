# Milestone 4.8: Bot Framework — Task Specification

> **Goal:** Build a goal-oriented automated playtesting bot that connects to the
> running game server via WebSocket, executes structured goal sequences, logs all
> commands and responses, and asserts outcomes. The bot lives in
> `src/harsh_realm/bot/` and its tests in `tests/bot/`. It will be used to
> validate M4.6 and M4.7 features as they land, and later to drive NPC agent
> behaviour.
> **Estimated time:** 3–4 days (AI-assisted)
> **Prerequisite:** M4.9 complete (sidebar and ChatLog events in place).
> Read CLAUDE.md, AGENTS.md before starting.

## Success Criteria

When this milestone is complete:

1. `pytest` passes with all tests green (excluding `tests/bot/` unless
   `--run-bot` flag is passed).
2. `GET /api/world/map` returns the full cell grid for a loaded world,
   including coordinates, terrain, passability, and feature type per cell.
3. The bot can connect to a running server, create a world, and create a
   character via command sequences.
4. The bot pathfinds to any reachable cell on the map using A* over the
   square grid, emitting movement commands and waiting for confirmation.
5. The bot executes the full first goal suite (see Task 4.8.5) without
   hanging or crashing.
6. Every bot run produces a structured JSON log of commands sent and
   responses received, with timestamps and pass/fail assertions.
7. Bot goal tests are marked `@pytest.mark.bot` and skipped by default.
   Running `pytest --run-bot` executes them against a live server.
8. All four test layers pass for non-bot code (world map endpoint and bot
   package unit tests run in the standard suite without a live server).

---

## Task 4.8.1: World Map API Endpoint

> **What:** Add `GET /api/world/map` returning the full passable cell grid.
> **Estimated time:** 2 hours

**File:** `src/harsh_realm/api/routes.py` (add endpoint)
**Model:** `src/harsh_realm/models/map.py` (new — MapCell and MapGrid Pydantic models)

**Endpoint:** `GET /api/world/map?world=<path>`

Uses the standard `?world=<path>` convention consistent with all other
`/api/admin/*` endpoints. If no world is loaded, returns 404.

**Response shape:**
```json
{
  "width": 20,
  "height": 20,
  "grid_type": "square",
  "cells": [
    {
      "q": 0,
      "r": 0,
      "terrain": "plains",
      "passable": true,
      "feature": "settlement",
      "feature_name": "Millhaven",
      "explored": false
    },
    ...
  ]
}
```

`passable` is derived from `terrain.yaml` passability flag (mountains and
water are impassable). `feature` is null if no feature on the cell.

**Pydantic models (`models/map.py`):**
```python
class MapCell(BaseModel):
    q: int
    r: int
    terrain: str
    passable: bool
    feature: str | None
    feature_name: str | None
    explored: bool

class MapGrid(BaseModel):
    width: int
    height: int
    grid_type: str
    cells: list[MapCell]
```

**Tests:**
- Unit: endpoint returns 200 with correct cell count (width × height)
- Unit: impassable terrain cells have `passable: false`
- Unit: settlement cells have `feature: "settlement"` and non-null `feature_name`
- Unit: endpoint returns 404 when no world loaded
- Property (Hypothesis): all returned cells have valid terrain values from
  the terrain registry; passable field is always bool

**Acceptance:** Endpoint returns complete grid. Bot can consume it to build
its internal map for pathfinding.

---

## Task 4.8.2: Bot Package Structure

> **What:** Create `src/harsh_realm/bot/` package with core data models and
> the BotRunner WebSocket client.
> **Estimated time:** 3 hours

**Package structure:**
```
src/harsh_realm/bot/
├── __init__.py
├── runner.py        # BotRunner — WS client, command/response loop
├── models.py        # BotState, Goal, BotAction, AssertionResult
├── logger.py        # Structured JSON log writer
└── assertions.py    # Assertion helpers
```

**`models.py`:**
```python
from dataclasses import dataclass, field
from typing import Callable

@dataclass
class BotState:
    world_path: str | None = None
    character_created: bool = False
    current_q: int = 0
    current_r: int = 0
    current_scene: str = "exploration"
    gold: int = 0
    hp: int = 0
    inventory: list[str] = field(default_factory=list)
    cells_visited: set[tuple[int, int]] = field(default_factory=set)
    last_response: str = ""
    combat_won: bool = False
    combat_fled: bool = False
    shop_purchased: bool = False
    npc_talked: bool = False

@dataclass
class BotAction:
    command: str                          # text command to send
    expected: str | None = None           # substring to assert in response
    update_state: Callable[[BotState, str], None] | None = None

@dataclass
class Goal:
    name: str
    preconditions: list[Callable[[BotState], bool]]
    actions: list[BotAction]
    success_condition: Callable[[BotState], bool]
    success_threshold: float = 1.0        # fraction of attempts required

@dataclass
class AssertionResult:
    goal_name: str
    passed: bool
    threshold: float
    actual_ratio: float
    notes: str = ""
```

**`runner.py` — BotRunner:**
```python
class BotRunner:
    def __init__(self, base_url: str, world_path: str):
        # base_url: e.g. "ws://localhost:8000"
        ...

    async def connect(self) -> None: ...
    async def send(self, command: str) -> str: ...
        # sends command, waits for next gm.narrate or gm.suggestions WS message
        # timeout: 10 seconds
        # returns response text
    async def run_goal(self, goal: Goal) -> AssertionResult: ...
    async def get_map(self) -> MapGrid: ...
        # calls GET /api/world/map via httpx
    def get_state(self) -> BotState: ...
    async def disconnect(self) -> None: ...
```

**`logger.py`:**

Each bot run writes to `logs/bot_runs/<timestamp>_<goal_name>.jsonl`.
Each line is one command/response pair:
```json
{
  "ts": "2026-03-29T14:22:01Z",
  "goal": "explore_entire_map",
  "command": "go north",
  "response": "You move north across the open ground...",
  "assertion": {"expected": null, "passed": null},
  "state_snapshot": {"q": 0, "r": 1, "scene": "exploration"}
}
```
Final line of each run is a summary:
```json
{"type": "summary", "goal": "explore_entire_map", "passed": true,
 "ratio": 1.0, "duration_seconds": 42.1}
```

**`assertions.py`:**
```python
def assert_contains(response: str, substring: str) -> bool: ...
def assert_exact(response: str, expected: str) -> bool: ...
def assert_threshold(results: list[bool], threshold: float) -> bool: ...
```

**Tests (standard pytest, no live server needed):**
- Unit: `BotState` default values are correct
- Unit: `assert_contains` matches substring case-insensitively
- Unit: `assert_threshold([], 0.9)` returns False
- Unit: `assert_threshold([True] * 9 + [False], 0.9)` returns True
- Unit: logger writes valid JSONL with correct fields
- Property: any sequence of `BotAction` objects can be serialised to the log
  format without error

**Acceptance:** Package importable. BotRunner instantiates without a live
server. Models are correct Pydantic/dataclass instances.

---

## Task 4.8.3: A* Pathfinder

> **What:** Implement A* pathfinding over the square grid using the map
> returned by the world map API.
> **Estimated time:** 2.5 hours

**File:** `src/harsh_realm/bot/pathfinder.py`

```python
class Pathfinder:
    def __init__(self, grid: MapGrid): ...

    def find_path(
        self,
        start: tuple[int, int],
        goal: tuple[int, int]
    ) -> list[tuple[int, int]] | None:
        """
        Returns list of (q, r) coords from start to goal (inclusive),
        or None if no path exists.
        Uses Chebyshev distance heuristic (king moves, 8 directions).
        Only traverses cells where passable=True.
        """

    def direction_to(
        self,
        current: tuple[int, int],
        next_cell: tuple[int, int]
    ) -> str:
        """
        Returns direction string ('north', 'south', 'east', 'west',
        'northeast', 'northwest', 'southeast', 'southwest') for the
        step from current to next_cell.
        """

    def all_reachable_cells(
        self,
        start: tuple[int, int]
    ) -> list[tuple[int, int]]:
        """
        BFS from start. Returns all passable cells reachable from start.
        Used to build the full exploration goal cell list.
        """
```

**Tests:**
- Unit: simple 3×3 grid — path from (0,0) to (2,2) found, length ≤ 3
- Unit: path blocked by impassable cell — detours correctly
- Unit: destination is impassable — returns None
- Unit: start == goal — returns single-element list
- Unit: `direction_to` returns correct direction for all 8 compass steps
- Property (Hypothesis): for any passable start and goal on a generated
  10×10 grid, `find_path` either returns a valid path (all cells passable,
  each step is adjacent) or None; never raises
- Property: `all_reachable_cells` result never contains impassable cells

**Acceptance:** Pathfinder finds shortest passable paths on square grids.
All property tests pass.

---

## Task 4.8.4: conftest & pytest Marker

> **What:** Add `@pytest.mark.bot` marker. Bot goal tests in `tests/bot/`
> are skipped unless `--run-bot` flag is passed. Standard suite is unaffected.
> **Estimated time:** 0.5 hours

**Files:**
- `pytest.ini` or `pyproject.toml` — register `bot` marker
- `conftest.py` (root) — add `--run-bot` flag and skip logic:

```python
def pytest_addoption(parser):
    parser.addoption(
        "--run-bot", action="store_true", default=False,
        help="Run bot integration tests against live server"
    )

def pytest_collection_modifyitems(config, items):
    if not config.getoption("--run-bot"):
        skip_bot = pytest.mark.skip(reason="Bot tests require --run-bot flag")
        for item in items:
            if "bot" in item.keywords:
                item.add_marker(skip_bot)
```

**Acceptance:** `pytest` runs without error and skips all `tests/bot/` tests.
`pytest --run-bot` includes them.

---

## Task 4.8.5: First Goal Suite

> **What:** Implement the first set of bot goals covering core game flows.
> These are integration tests requiring a live server.
> **Estimated time:** 4 hours

**File:** `tests/bot/test_first_suite.py`

All tests in this file are marked `@pytest.mark.bot`.

**Fixture:** `bot_runner` — starts the server as a subprocess, creates a
world via API, yields a connected `BotRunner`, tears down after.

```python
@pytest.fixture(scope="module")
async def bot_runner():
    # Start server: subprocess.Popen(["uvicorn", "harsh_realm.main:app", ...])
    # Create world: POST /api/worlds
    # Connect bot
    runner = BotRunner("ws://localhost:8001", world_path)
    await runner.connect()
    yield runner
    await runner.disconnect()
    # Kill server subprocess
```

**Goals to implement in `src/harsh_realm/bot/goals/first_suite.py`:**

**Goal 1 — Create Character**
```python
create_character_goal = Goal(
    name="create_character",
    preconditions=[],
    actions=[
        BotAction("new", expected="What is your character's name"),
        BotAction("TestBot", expected="Choose your class"),
        BotAction("warrior", expected="Roll attributes"),
        BotAction("roll", expected="Assign"),
        BotAction("assign str 14", expected=None),
        BotAction("assign dex 12", expected=None),
        BotAction("assign con 13", expected=None),
        BotAction("assign int 10", expected=None),
        BotAction("assign wis 11", expected=None),
        BotAction("assign cha 9", expected=None),
        BotAction("confirm", expected="Choose your kit"),
        BotAction("heavy_fighter", expected="You are"),
        # update_state sets character_created=True
    ],
    success_condition=lambda s: s.character_created,
)
```

**Goal 2 — Explore Entire Map**
```python
# Pathfinds to every reachable cell in BFS order.
# At each cell: sends 'search' and 'look'. Handles encounters by fleeing.
# success_condition: cells_visited / total_reachable >= 0.95
explore_map_goal = Goal(
    name="explore_entire_map",
    preconditions=[lambda s: s.character_created],
    actions=[],  # dynamically built from pathfinder
    success_condition=lambda s: (
        len(s.cells_visited) / s.total_reachable >= 0.95
    ),
    success_threshold=0.95,
)
```

Note: explore_map_goal builds its action list dynamically in `BotRunner.run_goal()`
by calling `pathfinder.all_reachable_cells()` and constructing movement + search
+ look sequences. Encounters detected by response containing "combat" keywords
trigger flee sequence.

**Goal 3 — Complete a Combat**
```python
# Finds a combat encounter (or triggers one by exploring hostile territory).
# Fights until enemy defeated. Does not flee.
complete_combat_goal = Goal(
    name="complete_combat",
    preconditions=[lambda s: s.character_created],
    actions=[
        # BotRunner handles combat loop: send 'attack' until
        # response contains 'defeated' or 'XP' or character dies
    ],
    success_condition=lambda s: s.combat_won,
)
```

**Goal 4 — Flee a Combat**
```python
flee_combat_goal = Goal(
    name="flee_combat",
    preconditions=[lambda s: s.character_created],
    actions=[
        BotAction("flee", expected=None),
    ],
    success_condition=lambda s: s.combat_fled,
)
```

**Goal 5 — Enter Town and Buy Item**
```python
enter_town_buy_goal = Goal(
    name="enter_town_buy_item",
    preconditions=[lambda s: s.character_created],
    actions=[
        # Pathfind to nearest settlement cell
        # BotAction("shop", expected="Available items"),
        # BotAction("buy rations", expected="Purchased"),
    ],
    success_condition=lambda s: s.shop_purchased,
)
```

**Goal 6 — Talk to NPC**
```python
talk_npc_goal = Goal(
    name="talk_to_npc",
    preconditions=[lambda s: s.character_created],
    actions=[
        # BotAction("look", expected=None),  # surfaces NPC names
        # BotAction("talk <first_npc_from_response>", expected="sociable"),
        # BotAction("ask about anything", expected=None),
        # BotAction("leave", expected=None),
    ],
    success_condition=lambda s: s.npc_talked,
)
```

**Test functions:**
```python
@pytest.mark.bot
async def test_create_character(bot_runner): ...

@pytest.mark.bot
async def test_explore_map(bot_runner): ...

@pytest.mark.bot
async def test_complete_combat(bot_runner): ...

@pytest.mark.bot
async def test_flee_combat(bot_runner): ...

@pytest.mark.bot
async def test_enter_town_buy_item(bot_runner): ...

@pytest.mark.bot
async def test_talk_to_npc(bot_runner): ...
```

Each test calls `bot_runner.run_goal(goal)` and asserts
`result.passed == True`.

**Log output:** Each test produces a JSONL log at
`logs/bot_runs/<timestamp>_<goal_name>.jsonl`. Logs are not committed
to git (add `logs/` to `.gitignore` if not already present).

**Acceptance:** All 6 goal tests pass when run with `pytest --run-bot`
against a freshly created world. Logs are produced for each run.

---

## Dependency Order

```
4.8.1 (map API endpoint) — no deps, build first
  ↓
4.8.3 (pathfinder) — needs MapGrid model from 4.8.1
  ↑
4.8.2 (bot package) — needs MapGrid model; can parallel with 4.8.3
  ↓
4.8.4 (pytest marker) — no deps, trivial
  ↓
4.8.5 (first goal suite) — needs 4.8.1 + 4.8.2 + 4.8.3 + 4.8.4
```

Recommended order:
1. 4.8.1 (map endpoint + MapGrid models)
2. 4.8.2 + 4.8.3 in parallel (bot package + pathfinder both need MapGrid)
3. 4.8.4 (marker, ~30 min)
4. 4.8.5 (goal suite — last, requires everything)

---

## Notes for the Coding Agent

- Read CLAUDE.md and AGENTS.md before starting.
- The bot is NOT a test mock. It connects to a live running server and
  sends real commands. Do not use FastAPI TestClient here.
- `BotRunner.send()` must handle the case where the server sends multiple
  WebSocket messages for a single command (the game currently emits
  gm.narrate + gm.suggestions per action). Collect all messages until
  a `gm.suggestions` event is received, then return the concatenated
  narration text as the response.
- The bot's pathfinder uses the map API, not DB queries. It has no
  knowledge of world state beyond what the API and game text provide.
- Combat detection in the explore goal: check response text for the
  strings "initiative", "attacks you", "combat" — if any present,
  the bot is in combat and should execute flee sequence.
- NPC name extraction in Goal 6: parse the `look` response for the
  "Present:" line added in M4.9 Task 4.9.6. Split on comma, use first
  name before the parenthesis.
- `logs/` directory must be created if it doesn't exist. Add
  `logs/` to `.gitignore`.
- After completing all tasks, update CLAUDE.md:
  - Mark Milestone 4.8 complete with date
  - Record test count
  - Note that bot tests require `--run-bot` flag
