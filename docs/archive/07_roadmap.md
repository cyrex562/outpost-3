# Outpost Colony Simulation - Development Roadmap

## Overview

This roadmap breaks the game into **small, incremental sessions** of 45-90 minutes each. Each session delivers a complete, testable feature.

**Current Status:** ✅ Completed through Session 1.2

---

## ✅ Phase 0: Bootstrap & Foundation (Sessions 0.1-0.2)

### Session 0.1: Project Structure ⭐ COMPLETE
Create directory structure, initialize git, setup basic files.

### Session 0.2: Godot Project Setup ⭐ COMPLETE
Create Godot project, configure settings, create Main scene, test empty project runs.

---

## ✅ Phase 1: Core Domain & Time System (Sessions 1.1-1.4)

### Session 1.1: Domain Types & First UI ⭐ COMPLETE
Create immutable GameState, StarSystem, basic commands/events, StateStore, first working UI with time display.

### Session 1.2: Probe System ⭐ COMPLETE
Launch probes, display probes in flight with countdown, probe arrival logic, system discovery, procedural generation.

### Session 1.3: Event Log & History (60 min) 📍 NEXT
Create persistent event log, display recent events in scrollable list, event filtering by type, export event history to file.

### Session 1.4: Save/Load System (90 min)
Serialize GameState to JSON, save game to file, load game from file, quick save/load hotkeys, save file management UI.

---

## 🔄 Phase 2: Star System Exploration (Sessions 2.1-2.6)

### Session 2.1: System Selection & Details (60 min)
Click discovered system to select it, display system details modal/panel, show bodies list with basic info, "View System" button navigation.

### Session 2.2: Star System Map Scene (75 min)
Create orbital view scene, visual representation of star with orbits, clickable body icons, zoom/pan controls, back to main menu button.

### Session 2.3: Body Details Modal (60 min)
Click body to show detailed modal, display atmosphere, gravity, resources, hazards, "Explore Body" action button.

### Session 2.4: Body Exploration Missions (75 min)
Launch exploration mission to body, mission duration and progress, reveal body properties on completion, mark body as explored in state.

### Session 2.5: Resource Deposits (60 min)
Generate resource deposits per body, display resource richness (iron, water, uranium, etc.), resource overlay on body view.

### Session 2.6: Hazard System (45 min)
Generate hazards (radiation, storms, seismic), display hazard severity, hazards affect mission success rate, hazard warnings in UI.

---

## 🏗️ Phase 3: Colony Landing & Establishment (Sessions 3.1-3.6)

### Session 3.1: Ship & Cargo System (75 min)
Create Ship entity with cargo manifest, colonist roster, define cargo items (supplies, equipment, vehicles, robots), display ship status UI.

### Session 3.2: Colony Site Selection (60 min)
Select body for colonization, choose landing sector from surface map, display sector suitability metrics, confirm landing site.

### Session 3.3: Landing Sequence (90 min)
Landing risk calculation based on conditions, landing animation/sequence, cargo deployment with drift mechanics, update state with landed colony.

### Session 3.4: Surface Sector Map (90 min)
Create hex/square grid surface map, display terrain types per sector, elevation visualization, basic camera controls (pan/zoom).

### Session 3.5: Initial Base Structures (75 min)
Place initial habitat module, life support module, power generator, display structures on sector map, basic structure properties.

### Session 3.6: Colony Overview UI (60 min)
Create colony dashboard, display population count, resource stockpiles, power/oxygen status, morale indicator.

---

## ⚡ Phase 4: Power & Life Support (Sessions 4.1-4.4)

### Session 4.1: Power Generation (60 min)
Solar panel, nuclear reactor, RTG building types, power production rates, power storage (batteries), display power grid status.

### Session 4.2: Power Distribution (75 min)
Connect buildings to power network, power consumption per building, brownout/blackout mechanics when insufficient, visual power grid overlay.

### Session 4.3: Life Support System (60 min)
Oxygen generation (plants, electrolysis), CO2 scrubbing, temperature control, water recycling, display life support status per sector.

### Session 4.4: Environmental Hazards (60 min)
Buildings degrade without power, colonists die without oxygen/heat, repair mechanics, emergency power systems, warning alerts.

---

## 👥 Phase 5: Population & Labor (Sessions 5.1-5.5)

### Session 5.1: Colonist Entities (60 min)
Individual colonist records (name, role, skills), health, morale, fatigue stats, colonist detail view UI, display roster.

### Session 5.2: Labor Assignment (75 min)
Assign colonists to buildings/tasks, skill matching (engineer, scientist, laborer), efficiency modifiers based on skill, unassigned colonist pool.

### Session 5.3: Morale System (60 min)
Calculate morale from conditions (food, space, safety), morale affects productivity, events triggered by low morale (strikes, leaving), morale boosting actions.

### Session 5.4: Population Growth (45 min)
Birth mechanics over time, death from age/hazards/starvation, immigration when colony attractive, population growth UI visualization.

### Session 5.5: Skills & Training (60 min)
Colonists gain experience in roles, skill levels (novice → expert), training programs, skill tree visualization.

---

## 🏭 Phase 6: Resource Extraction & Production (Sessions 6.1-6.6)

### Session 6.1: Resource Types (45 min)
Define core resources (ore, metal, water, food, oxygen, fuel), resource stockpile entity, display resource inventory UI, basic resource flow.

### Session 6.2: Mining Operations (75 min)
Build mine on resource deposit, extraction rate based on richness/tech, assign miners, deplete deposits over time, ore stockpile.

### Session 6.3: Refinery & Processing (60 min)
Refinery building converts ore → metal, processing time and ratios, input/output storage, display processing queue UI.

### Session 6.4: Manufacturing (75 min)
Factory building produces goods, crafting recipes (metal + power → components), production queue management, display production chain.

### Session 6.5: Storage & Logistics (60 min)
Warehouse building for bulk storage, storage capacity limits, automatic resource routing, manual transfer commands, storage UI.

### Session 6.6: Production Chain Visualization (90 min)
Flow diagram UI showing resource paths, identify bottlenecks visually, production/consumption rates, optimize chain recommendations.

---

## 🤖 Phase 7: Vehicles & Automation (Sessions 7.1-7.6)

### Session 7.1: Vehicle Types (60 min)
Define vehicle types (rover, excavator, hauler, surveyor), vehicle stats (speed, capacity, fuel), vehicle entity and state, display vehicle roster.

### Session 7.2: Vehicle Commands (75 min)
Manual vehicle control (move to sector), task assignment (mine, haul, survey), fuel consumption, vehicle maintenance needs, vehicle status UI.

### Session 7.3: Road Network (60 min)
Build roads between sectors, roads increase vehicle speed, road maintenance costs, road overlay on sector map.

### Session 7.4: Automated Mining (75 min)
Script vehicle mining loops (mine → haul → return), automation templates, vehicle AI executes tasks, stop/start automation controls.

### Session 7.5: Hauler Routes (60 min)
Define pickup/delivery routes, automatic cargo transfer, multi-stop routes, route efficiency calculations, route configuration UI.

### Session 7.6: Drone Operations (60 min)
Drone entity (no roads needed), aerial survey missions, cargo delivery drops, drone station building, drone management UI.

---

## 🔬 Phase 8: Research & Technology (Sessions 8.1-8.5)

### Session 8.1: Tech Tree Structure (75 min)
Define tech tree nodes, prerequisites and dependencies, tech categories (physics, engineering, biology), tech costs (time, resources), tech tree visualization UI.

### Session 8.2: Research Lab (60 min)
Build research lab building, assign scientists to projects, research point generation rate, display active research UI.

### Session 8.3: Unlockable Technologies (90 min)
Unlock new buildings via research, unlock efficiency upgrades, unlock automation features, display unlocked tech notifications.

### Session 8.4: Engineering Projects (75 min)
Large one-time projects (orbital station, megastructure), resource and time requirements, project milestones, project management UI.

### Session 8.5: Research Priorities (45 min)
Queue multiple research projects, auto-start next in queue, research funding slider, pause/resume research.

---

## 🏛️ Phase 9: Governance & Policy (Sessions 9.1-9.4)

### Session 9.1: Policy System (60 min)
Define policy categories (economy, labor, ethics), enact policies with cooldowns, policy effects on morale/efficiency, policy UI panel.

### Session 9.2: Resource Rationing (60 min)
Ration food/water/oxygen in shortage, priority tiers (critical, normal, low), rationing affects morale, rationing configuration UI.

### Session 9.3: Taxation & Economy (75 min)
Abstract currency system, tax colonists for revenue, spend on infrastructure/services, budget tracking UI, deficit/surplus effects.

### Session 9.4: Colony Specialization (60 min)
Unlock colony types (mining, research, agricultural), specialization bonuses, specialization affects available policies, specialization selection UI.

---

## 🌍 Phase 10: Multi-Colony Management (Sessions 10.1-10.5)

### Session 10.1: Second Colony Foundation (60 min)
Launch new colony ship to different body, independent colony state, multiple colony tracking, switch between colonies UI.

### Session 10.2: Inter-Colony Trade (90 min)
Define trade routes between colonies, cargo ships transport goods, travel time calculations, trade route configuration, trade balance tracking.

### Session 10.3: Resource Sharing (60 min)
Pool resources across colonies, automatic balancing, emergency aid shipments, central resource dashboard UI.

### Session 10.4: System Overview (75 min)
System-wide dashboard, all colonies at a glance, resource totals, alerts across colonies, quick navigation between colonies.

### Session 10.5: Orbital Infrastructure (75 min)
Build orbital stations, space-based manufacturing, fuel depots in orbit, orbital asset management UI.

---

## 🛰️ Phase 11: Advanced Exploration (Sessions 11.1-11.4)

### Session 11.1: Improved Probes (60 min)
Upgrade probe types (fast, detailed, deep-space), probe costs resources, probe reliability/failure chance, probe configuration UI.

### Session 11.2: Manned Expeditions (75 min)
Send colonists on surface expeditions, expedition risks and rewards, discover POIs (alien artifacts, wrecks), expedition planning UI.

### Session 11.3: Anomalies & Events (90 min)
Generate anomalies per system, investigate anomalies (missions), anomaly outcomes (tech, resources, story), anomaly log UI.

### Session 11.4: Sector Deep Scan (60 min)
Detailed sector scanning mission, reveal hidden resources, find subsurface caves, scan data visualization UI.

---

## 🎯 Phase 12: Random Events & Narrative (Sessions 12.1-12.4)

### Session 12.1: Event System Framework (75 min)
Event trigger conditions (time, state), event templates with choices, event outcomes modify state, event modal UI with options.

### Session 12.2: Voyage Events (60 min)
Ship journey events (system failure, social conflict), choice affects arrival conditions, event log during travel, voyage event library.

### Session 12.3: Colony Events (75 min)
Colony crisis events (fire, disease, mutiny), random positive events (discovery, birth, volunteer), event frequency based on conditions, event notification system.

### Session 12.4: Story Beats (90 min)
Scripted story events at milestones, character-driven events, branching narrative choices, story progress tracking, story event UI.

---

## 📊 Phase 13: Analytics & Visualization (Sessions 13.1-13.4)

### Session 13.1: Statistics Tracking (60 min)
Track metrics over time (population, production, resources), time-series data structure, aggregate statistics calculations, export data to CSV.

### Session 13.2: Charts & Graphs (90 min)
Line charts for resource trends, bar charts for production, pie charts for resource distribution, interactive chart UI with zoom/pan.

### Session 13.3: Performance Reports (60 min)
Colony efficiency scores, productivity reports per sector, resource waste analysis, bottleneck identification, report generation UI.

### Session 13.4: Achievements (45 min)
Define achievement conditions, track achievement progress, unlock achievements, achievement showcase UI, achievement notifications.

---

## 🎮 Phase 14: UI/UX Polish (Sessions 14.1-14.5)

### Session 14.1: Tutorial System (90 min)
Step-by-step guided tutorial, tooltip system, help codex with searchable entries, tutorial sequence for new players.

### Session 14.2: Keyboard Shortcuts (45 min)
Define hotkey mappings, configurable key bindings, hotkey reference screen, implement common shortcuts (save, pause, zoom).

### Session 14.3: Camera & Navigation (60 min)
Smooth camera transitions, bookmark favorite locations, mini-map navigation, camera presets (colony, system, overview).

### Session 14.4: Visual Feedback (75 min)
Animations for state changes, particle effects for events, sound effects for actions, progress indicators for long operations.

### Session 14.5: Accessibility (60 min)
Colorblind mode, font size options, high contrast mode, screen reader support basics, accessibility settings panel.

---

## 🔧 Phase 15: Advanced Features (Sessions 15.1-15.5)

### Session 15.1: Automation Scripts (90 min)
YAML-like scripting language, if/then/else logic, trigger-action patterns, script editor UI, script library/templates.

### Session 15.2: Production Templates (60 min)
Save production chain configs, load templates across colonies, import/export templates, template library UI.

### Session 15.3: Scenario Mode (75 min)
Predefined starting conditions, scenario objectives, scenario victory conditions, scenario selection menu, custom scenario creator.

### Session 15.4: Sandbox Mode (45 min)
Unlimited resources toggle, instant construction, no fail conditions, sandbox settings panel, sandbox save distinction.

### Session 15.5: Mod Support Foundation (90 min)
JSON data pack loading, mod folder structure, override base content, mod priority system, mod manager UI.

---

## 🚀 Phase 16: Endgame & Meta (Sessions 16.1-16.3)

### Session 16.1: Interstellar Ship Construction (90 min)
Build new colony ship in orbit, massive resource requirements, construction progress tracking, launch new mission from colony.

### Session 16.2: Victory Conditions (60 min)
Multiple win conditions (population, tech, expansion), victory progress tracking, victory screen with stats, new game+ mode.

### Session 16.3: Persistent Meta Progression (75 min)
Unlock bonuses across playthroughs, legacy achievements, persistent tech unlocks, meta progression UI.

---

## 🐛 Phase 17: Testing & Optimization (Sessions 17.1-17.4)

### Session 17.1: Performance Profiling (75 min)
Identify bottlenecks, optimize update loops, reduce memory allocations, frame rate monitoring, performance settings.

### Session 17.2: Balance Pass (90 min)
Tune resource rates, adjust tech costs, balance difficulty curve, playtest feedback integration, balance configuration file.

### Session 17.3: Bug Fixing Sprint (variable)
Address reported issues, fix crash bugs, resolve logic errors, improve error handling, test edge cases.

### Session 17.4: Polish Pass (90 min)
Fix visual glitches, improve text clarity, add missing tooltips, consistent styling, final QA pass.

---

## 📦 Phase 18: Release Preparation (Sessions 18.1-18.3)

### Session 18.1: Export Configuration (60 min)
Configure Windows export, configure Linux export, configure Web (WASM) export, test exported builds, export presets.

### Session 18.2: Documentation (75 min)
Write player manual, create quick start guide, document controls, FAQ section, in-game help integration.

### Session 18.3: Release Build (45 min)
Version numbering, build scripts, package distribution files, upload to itch.io/Steam, launch checklist.

---

## 📈 Future Expansions (Post-Launch)

### Expansion A: Diplomacy & Factions
- AI-controlled rival colonies
- Trade negotiations
- Territorial disputes
- Alliance system

### Expansion B: Advanced Combat
- Defense structures
- Military units
- Space battles
- Invasion mechanics

### Expansion C: Biological Systems
- Terraforming mechanics
- Ecosystem simulation
- Alien life forms
- Agricultural complexity

### Expansion D: Advanced Economy
- Market system
- Currency fluctuations
- Import/export economy
- Economic policies

---

## Session Time Estimates

**Total Sessions:** ~170 sessions  
**Average Session Length:** 60 minutes  
**Total Development Time:** ~170 hours  
**At 1 hour/day:** ~6 months to feature-complete  
**At 3 hours/week:** ~14 months to feature-complete

---

## Milestone Summary

| Milestone | Sessions | Description |
|-----------|----------|-------------|
| **MVP** | 0.1 - 4.4 | Playable core loop: launch, explore, land, survive |
| **Alpha** | 5.1 - 9.4 | Full colony management with all core systems |
| **Beta** | 10.1 - 13.4 | Multi-colony, polish, analytics |
| **Release Candidate** | 14.1 - 17.4 | UI polish, testing, optimization |
| **v1.0 Launch** | 18.1 - 18.3 | Release preparation and distribution |

---

## Current Focus

**✅ Completed:** Sessions 0.1, 0.2, 1.1, 1.2  
**📍 Next Session:** 1.3 - Event Log & History  
**🎯 Next Milestone:** MVP (Core Loop) - Target completion: Session 4.4

---

## Notes

- **Flexibility:** Session order can be adjusted based on interest
- **Session Splitting:** Any 90-min session can be split into two 45-min sessions
- **Session Merging:** Related 45-min sessions can be combined if momentum is good
- **Skippable Sessions:** Some polish/optional features can be deferred to post-launch

**Strategy:** Focus on vertical slices (complete features) over horizontal layers to maintain motivation and have playable builds frequently.