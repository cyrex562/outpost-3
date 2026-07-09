# Oracle Rules Reference
> For use by coding agents implementing Milestone 4 oracle system.
> Sources: Mythic Game Master Emulator by Tom Pigeon; Mythic Adventure Crafter by Tom Pigeon.
> The M2 oracle.py is a placeholder — Task 4.7 replaces it entirely.

## Mythic GME Overview

The Mythic GME provides tools for solo play: answering yes/no questions with probabilistic outcomes, checking whether scenes go as expected, generating random events when they don't. It runs alongside the main game mechanics.

Three core components:
1. **Fate Chart** — answers yes/no questions
2. **Scene Checks** — determines if scenes play out as expected
3. **Random Events** — generates emergent plot complications

One controlling variable ties it together: **Chaos Factor** (1–9, starts at 5).

---

## Chaos Factor

Chaos Factor (CF) represents how out-of-control the situation is.

- **High CF (7–9):** Things are chaotic. NPCs act unpredictably. Scenes rarely go as planned. Bad things happen more often.
- **Low CF (1–3):** Things are under control. The player is on top of the situation. Scenes mostly proceed as expected.

**Adjusting CF:**
- Player wins / achieves their goal this scene: CF decreases by 1 (min 1)
- Player loses / situation goes wrong: CF increases by 1 (max 9)

Store in `gm_state` table, key `oracle_chaos_factor`, default 5.

---

## Fate Chart

The Fate Chart resolves yes/no questions. Input: a likelihood and the current CF. Output: Exceptional Yes, Yes, No, or Exceptional No.

### Likelihoods (9 levels)

```
1: Impossible
2: No Way
3: Very Unlikely
4: Unlikely
5: 50/50
6: Likely
7: Very Likely
8: Sure Thing
9: Has To Be
```

### How to Read the Chart

Roll d100. Compare against the threshold for (likelihood × CF):
- Roll ≤ `exceptional_yes` threshold: **Exceptional Yes** (yes, and something extra/unexpected)
- Roll ≤ `yes` threshold: **Yes**
- Roll ≥ `exceptional_no` threshold: **Exceptional No** (no, and something extra goes wrong)
- Otherwise: **No**

Encode the full 9×9 matrix verbatim from the Mythic GME rulebook into `data/tables/oracle/fate_chart.yaml`:

```yaml
# fate_chart.yaml
# 9 likelihoods × 9 chaos factors
# Each cell: yes_threshold (roll ≤ this = Yes), exceptional_yes, exceptional_no
impossible:
  chaos_1: {yes_threshold: 4, exceptional_yes: 0, exceptional_no: 96}
  chaos_2: {yes_threshold: 7, exceptional_yes: 0, exceptional_no: 96}
  # ... all 9 chaos values
no_way:
  chaos_1: {yes_threshold: 5, exceptional_yes: 0, exceptional_no: 93}
  # ...
# ... all 9 likelihoods
```

### Fate Check Procedure

```python
def fate_check(likelihood: str, chaos_factor: int) -> FateResult:
    cell = fate_chart[likelihood][f"chaos_{chaos_factor}"]
    roll = d100()
    if roll <= cell["exceptional_yes"]:
        return FateResult(result="Exceptional Yes", roll=roll, ...)
    elif roll <= cell["yes_threshold"]:
        return FateResult(result="Yes", roll=roll, ...)
    elif roll >= cell["exceptional_no"]:
        return FateResult(result="Exceptional No", roll=roll, ...)
    else:
        return FateResult(result="No", roll=roll, ...)
```

### Narration Format

Always show the mechanical details alongside the result:
```
Fate check: Is there a guard at the door? (Likely, Chaos 5)
Roll: 34 vs threshold 65 — Yes
```

Exceptional results add a twist:
```
Roll: 8 vs exceptional_yes threshold 10 — Exceptional Yes
Not only is there a guard, but [generate random event for context]
```

---

## Scene Checks

At the start of each new scene, the GM checks whether the scene plays out as the player expects.

### When to Fire

Scene checks trigger automatically on every scene transition:
- Player enters a new hex
- Player enters a social interaction
- Player begins a rest
- Combat ends
- Player enters a dungeon room
- Any explicit scene change in the GM Controller

### Procedure

```python
def scene_check(chaos_factor: int) -> SceneModification:
    roll = d10()
    if roll > chaos_factor:
        return SceneModification(type="normal")          # scene proceeds as expected
    elif roll % 2 == 1:  # odd
        return SceneModification(type="interrupt")       # random event fires, scene changes
    else:                # even
        return SceneModification(type="altered")         # scene proceeds but differently
```

**Normal:** Tell the player the scene begins as expected. No additional resolution.

**Interrupt:** The expected scene doesn't begin. A random event fires instead. Generate and narrate the event. The player now deals with this before their intended scene.

**Altered:** The scene begins but something is different. Generate a random event for context about *what* is different. The scene proceeds but the GM introduces a complication or twist.

### Probability by Chaos Factor

At CF 9, ~90% of scenes are interrupted or altered.
At CF 1, ~10% of scenes are interrupted or altered.

---

## Random Event Tables

Three tables of 100 entries each. Encode verbatim from Mythic GME rulebook:

### Event Focus (d100)
What type of event is this? Examples:
- Remote event (something happens elsewhere)
- NPC action
- Introduce a new NPC
- Move toward a thread
- Move away from a thread
- Close a thread
- PC negative (bad for player)
- PC positive (good for player)
- Ambiguous event
- etc.

### Event Action (d100)
What is happening? One-word action (appears, attacks, betrays, creates, deceives, etc.)

### Event Subject (d100)
What is it about? One-word subject (a plan, an enemy, happiness, a weapon, a path, etc.)

### Random Event Generation

```python
def random_event() -> RandomEvent:
    focus = roll_on("event_focus")
    action = roll_on("event_action")
    subject = roll_on("event_subject")
    return RandomEvent(
        focus=focus,
        action=action,
        subject=subject,
        description=f"{focus}: {action} {subject}"
    )
```

The GM interprets the result in context. Example: "NPC action: betray a plan" → the blacksmith is secretly reporting the player's activities to a faction. Narrate the scene accordingly.

---

## Thread and NPC Tracking

Mythic tracks two lists as live game state:

### Thread List

Threads are active story concerns. Types: story (plot events) or character (personal arcs).

```sql
CREATE TABLE threads (
    id       TEXT PRIMARY KEY,
    type     TEXT NOT NULL,    -- "story" | "character"
    title    TEXT NOT NULL,
    status   TEXT DEFAULT 'active',
    progress INTEGER DEFAULT 0,
    data     TEXT DEFAULT '{}'
);
```

Commands:
- `add thread <title>` → create story thread
- `add character thread <title>` → create character thread
- `resolve thread <id>` → mark resolved (inactive)
- `abandon thread <id>` → mark abandoned (inactive)
- `list threads` → show active threads with IDs and progress

When random event Focus references a thread (e.g., "Move toward a thread", "Close a thread"), increment that thread's progress counter.

### Oracle NPC List

NPCs relevant to the current story. Separate from the entity/NPC system — this list includes NPCs not yet encountered or without full stat blocks.

```sql
CREATE TABLE oracle_npcs (
    id        TEXT PRIMARY KEY,
    name      TEXT NOT NULL,
    status    TEXT DEFAULT 'active',
    notes     TEXT,
    entity_id TEXT REFERENCES entities(id)  -- link to entity if they exist in world
);
```

Commands:
- `add npc <name>` → add to oracle list
- `remove npc <id>` → remove from list
- `list npcs` → show active NPCs in oracle list

---

## Adventure Crafter

The Adventure Crafter builds structured stories on top of the core GME. It introduces plotlines, themed scenes, and systematic thread progression.

### Themes (5 types)

Each plotline has a theme that biases scene element generation:

| Theme | Description | Typical Elements |
|---|---|---|
| **Action** | Physical conflict, urgency, danger | Combat, chases, disasters |
| **Tension** | Pressure, difficult choices, mounting stakes | Deadlines, betrayals, moral dilemmas |
| **Mystery** | Secrets, discovery, investigation | Hidden motives, clues, revelations |
| **Social** | Relationships, politics, negotiation | Alliances, rivalries, NPC interactions |
| **Personal** | Character growth, backstory, internal conflict | Past catches up, beliefs tested |

### Plotlines

A plotline is an active story arc.

```sql
CREATE TABLE plotlines (
    id     TEXT PRIMARY KEY,
    title  TEXT NOT NULL,
    theme  TEXT NOT NULL,  -- "action"|"tension"|"mystery"|"social"|"personal"
    status TEXT DEFAULT 'active',
    scenes TEXT DEFAULT '[]',  -- JSON array of ACScene records
    data   TEXT DEFAULT '{}'
);
```

Commands:
- `create plotline <title> <theme>` → start a new plotline
- `list plotlines` → show active plotlines
- `advance plotline <id>` → generate next scene for the plotline
- `resolve plotline <id>` → mark plotline complete

### Adventure Crafter Tables

Three tables from the AC rulebook (encode verbatim):

**`ac_themes.yaml`:** 5 themes × weighted sub-tables. Each theme has its own 50-entry table of scene elements biased toward that theme's character. When advancing a plotline, roll on the active plotline's theme table.

**`ac_characters.yaml`:** Character element table. Generates character-based scene components.

**`ac_plots.yaml`:** Plot element table. Generates plot-based scene components.

### Scene Generation

When `advance plotline <id>` is called:

```python
def generate_scene(plotline: Plotline) -> ACScene:
    # Roll on the plotline's theme table for primary element
    primary = roll_on(f"ac_themes_{plotline.theme}")
    # Roll on character or plot table for secondary element
    secondary = roll_on("ac_characters" if d2() == 1 else "ac_plots")
    scene_number = len(plotline.scenes) + 1
    return ACScene(
        number=scene_number,
        primary_element=primary,
        secondary_element=secondary,
        theme=plotline.theme,
    )
```

Scene output format:
```
Advancing plotline "Find the Starship" (Action theme)...

Scene 3: A conflict erupts between competing interests.
Characters: Iron Pact commander, Mysterious stranger
Plot element: Hidden agenda revealed

Thread progression:
  "Find the Starship" (story): progress 3/5 — not yet resolved
  "Trust the stranger" (character): progress 3/3 — Resolution check: roll 8 vs 3 — not yet
```

### Thread Progression

Each time a scene is completed (player advances a plotline), threads connected to that plotline progress.

**Progression thresholds:**
- Character thread: check for resolution every 3 scenes
- Story thread: check for resolution every 5 scenes

**Resolution check:**
```python
def thread_resolution_check(thread: Thread) -> bool:
    roll = d10()
    return roll <= thread.progress  # higher progress = more likely to resolve
```

If resolution check succeeds: mark thread as resolved. Narrate the resolution.
If not: thread continues, progress continues accumulating.

Threads can also be manually resolved with `resolve thread <id>` at any time.

### Tracking Scene Completion

When `advance plotline <id>` is called:
1. Generate scene (as above)
2. Add scene to plotline.scenes list in DB
3. Check all active threads connected to this plotline
4. For threads at their check threshold: run resolution check
5. Narrate results

---

## Implementation Notes

- The M2 `oracle.py` is a stub. **Replace it entirely** in Task 4.7. Do not try to extend it.
- Store CF in `gm_state` table, key `oracle_chaos_factor`. Default 5.
- Fate chart must be encoded from the actual Mythic GME rulebook — do not estimate values.
- Scene checks must fire on EVERY scene transition in the GM Controller. Wire it in `gm/controller.py` as part of the transition logic, not as an optional call.
- Thread progress is incremented when random event Focus references a thread. The exact mapping (which Focus values trigger progress on which thread type) is in the Mythic GME rulebook — encode verbatim.
- Adventure Crafter theme tables have their own distinct probability distributions. The Action theme should generate action-flavored elements substantially more often than other themes. Test this with distribution checks (Task 4.8 tests).
