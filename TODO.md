# Outpost 3 — Post-M1 Recommendations

Codebase review completed after Milestone 1 (Colony Ship Arc — Observe Only).
All 6 sessions complete, 190 tests passing, full arc functional.

---

## Backend — Error Handling & Robustness

- [ ] **Replace bare `except Exception` blocks with specific handling**
  - `engine.py`: 5 instances — system tick errors, batch handlers, event/state listeners
  - `main.py`: 2 instances — WebSocket broadcast, snapshot loop
  - `narrative/__init__.py`: 1 instance — custom renderer failures return None silently
  - Add logging for suppressed exceptions; return fallback text for narrative failures

- [ ] **Clamp resource values at zero**
  - `Resources` component allows negative values (food, fuel, water, etc.)
  - Systems consume without flooring — can produce nonsensical negative displays
  - Add clamping in `Resources` or at consumption sites

- [ ] **Add reproducible playthrough support (`--seed` option)**
  - All `random` calls are currently unseeded
  - Pass a seed through to all systems for deterministic runs
  - Enables: debugging, regression testing, "share this story" feature

## Backend — Architecture

- [ ] **Centralize phase transitions into a state machine**
  - Currently scattered: SearchSystem→TRANSIT, TransitSystem→SURVEY, SurveySystem→FOUNDING/SEARCH
  - A `PhaseManager` or `MilestoneSystem` could own all transitions
  - Easier to debug, log, and extend with new phases

- [ ] **Extract behavior system thresholds to a config dict**
  - Magic numbers in `behavior.py`: food 30%, hull 70%, morale 40%, engine 50%, etc.
  - Single `THRESHOLDS` dict at module level for easy tuning
  - Same for consumption rates (food 0.004/person/day, water 0.003/person/day)

## Backend — Testing

- [ ] **Add end-to-end integration test (LOADOUT → FOUNDING)**
  - No test currently runs the full arc in one go
  - Would catch phase-transition edge cases and state consistency bugs
  - Run with a fixed seed for reproducibility

- [ ] **Add pytest to requirements (or create requirements-dev.txt)**
  - pytest is needed to run tests but not listed as a dependency

- [ ] **Add memory/performance test for long simulations**
  - Verify no leaks over 1000+ game-days at high speed
  - Check event log buffer doesn't grow unbounded

## Frontend — Reliability

- [ ] **Add exponential backoff to WebSocket reconnect**
  - `useWebSocket.js` uses fixed 2-second retry
  - Should escalate: 2s, 4s, 8s, 16s... with cap
  - Prevents hammering backend when it's down

- [ ] **Add WebSocket heartbeat/ping-pong**
  - Long-lived connections can silently die (NAT/proxy timeout)
  - Periodic ping detects dead connections faster than waiting for send failure

- [ ] **Virtualize the event log**
  - Currently renders all events in DOM (up to 1000)
  - At high counts, scrolling and rendering degrade
  - Virtual scroller renders only visible items

## Frontend — UX

- [ ] **Add loading/connecting state to panels**
  - Before first WebSocket message, panels show blank/zero data
  - Show "Connecting..." or skeleton state until data arrives

- [ ] **Surface REST command failures to the user**
  - Failed pause/resume/speed commands only log to console
  - Add toast notification or status indicator for errors

## Infrastructure

- [ ] **Use production build in frontend Dockerfile**
  - Currently runs `npm run dev` (Vite dev server)
  - Should use `npm run build` + nginx for production

- [ ] **Add CI pipeline (GitHub Actions)**
  - Run `pytest` on push/PR
  - Run frontend build check
  - Catches regressions automatically

## Future Prep (Nice to Have)

- [ ] **Implement save/load persistence**
  - `World.to_dict()` / `from_dict()` exist but nothing writes to disk
  - Simple JSON file save would enable resume and story sharing

- [ ] **Add command acknowledgment protocol**
  - REST commands return success but frontend doesn't verify state changed
  - Could cause UI desync on network issues

- [ ] **Profile snapshot serialization for M2+ entity growth**
  - Currently fine for ~20 entities
  - `build_snapshot()` iterates all components every push — watch at scale
