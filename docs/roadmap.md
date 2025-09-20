# Project Roadmap

This roadmap is designed to stay expandable—each phase calls out deliverables plus placeholders for future discoveries.

## Phase 0 – Stabilize the Godot Port (Now)
- [ ] Verify CSV parsing against edge cases (missing data, alternative datasets).
- [ ] Render orbital positions on a 2D map (simple Node2D prototype).
- [ ] Add manual turn controls (UI buttons + keyboard shortcuts).
- [ ] Mirror Rust-side simulation deltas inside C# tests.
- [ ] Document data-loading expectations for modders.

## Phase 1 – Game Loop Foundations
- [ ] Layer basic UI panels (date/turn readout, body inspector, log feed).
- [ ] Introduce time controls (pause, step, auto-play speed settings).
- [ ] Implement settlement/resource data models in C#.
- [ ] Sync faction/population logic between Rust and C# services.
- [ ] Add save/load scaffolding (JSON serialization of services).

## Phase 2 – Simulation Depth
- [ ] Expand economic processes (production chains, upkeep, markets).
- [ ] Model population dynamics (growth, migration, morale).
- [ ] Implement faction agendas and diplomacy triggers.
- [ ] Create event pipeline for narrative beats.
- [ ] Stress-test performance with large solar-system datasets.

## Phase 3 – Strategic & UX Polish
- [ ] Build tactical overlays (trade routes, influence, alerts).
- [ ] Add scripting/mod hooks (C# or Rust plugin boundaries).
- [ ] Improve visual presentation (tile/iso renderer, effects).
- [ ] Integrate audio feedback loop (alerts, ambience).
- [ ] Conduct first external playtest and gather feedback.

## Ongoing Tracks
- [ ] Maintain parity between Rust and C# models (document divergences).
- [ ] Continuous test coverage for math-heavy systems.
- [ ] Tooling automation (CSV validation, asset import pipeline).
- [ ] Performance profiling after every major feature.
- [ ] Update documentation after each milestone (changelog + lessons).

Append new phases or expand bullet items as the design matures—treat each checkbox as a future issue stub.
