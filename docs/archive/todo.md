# Project TODO List

Short-term backlog tuned to the new Godot-first workflow. Promote/demote items as milestones shift.

## Next Up – Godot Prototype
- [ ] Draw orbit gizmos or sprites so positions are visible in-scene.
- [ ] Build a basic HUD (turn counter, current date, selected body info).
- [ ] Expose turn advance to UI buttons + keyboard shortcuts.
- [ ] Add minimal unit tests for SolarSystemService (orbit step regression cases).

## Next Up – Rust Sandbox
- [ ] Keep Bevy project compiling against latest dependencies.
- [ ] Extract reusable simulation logic into libraries for future FFI use.
- [ ] Document differences between Rust and C# implementations as they appear.

## Research / Spikes
- [ ] Evaluate best approach for sharing data between Rust and Godot (GDNative vs REST vs file hand-off).
- [ ] Investigate tile/iso rendering solutions compatible with both engines.
- [ ] Explore narrative/event tooling (ink, yarn, custom) for long-form emergent stories.

## Technical Debt & Tooling
- [ ] Automate CSV validation and conversion to project-friendly formats.
- [ ] Establish logging conventions for both engines.
- [ ] Set up CI lint/build for Rust and C# projects.
- [ ] Plan serialization format for saves and cross-engine data transfer.

## Completed
- [x] Stand up Godot 4 C# project skeleton mirroring the Rust simulation.
- [x] Document port architecture and roadmap links.
