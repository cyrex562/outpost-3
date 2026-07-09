# Saving Throw Infrastructure Audit

> Produced for M4.6 Task 4.6.1. Approach: EXTEND (not replace).

## What Exists

- **Character model**: `physical_save`, `evasion_save`, `mental_save` (all `int = 15`)
- **Recalculator**: `_calc_saves(level)` → `15 - (level // 2)`, same formula for all three
- **Advancement**: Even levels improve all three saves by 1
- **classes.yaml**: Each class has `saving_throws: {physical: 15, evasion: 15, mental: 15}`
- **Resolution**: None. No `resolve_save()` function exists. Saves are stored but never rolled against.
- **Tests**: 4 tests verify calculation and advancement. All pass.

## Extension Plan

Add `luck_save` as a fourth type. Create `engine/saves.py` with `resolve_save()`.

| File | Change |
|---|---|
| `models/character.py` | Add `luck_save: int = 15`, `save_bonuses: dict[str, int] = {}` |
| `engine/character_recalc.py` | Extend `_calc_saves()` and `RecalcResult` to include `luck_save` |
| `engine/advancement.py` | Extend even-level improvement to include `luck_save` |
| `data/classes.yaml` | Add `luck: 15` to all three classes |
| `engine/saves.py` | **NEW** — `resolve_save(character, save_type, difficulty_modifier)` → `SaveResult` |
| Tests | Add luck save tests to recalc + advancement; new `test_saves.py` |
