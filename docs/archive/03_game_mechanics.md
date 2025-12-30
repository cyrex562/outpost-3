# Game Mechanics

| **Category**                      | **Mechanic**                           | **Description / Player Interaction**                                                                       | **Systems / Screens Involved**                                                |
| --------------------------------- | -------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| **Exploration & Discovery**       | **Probe Launch & Data Resolution**     | Launch probes to discover and scan star systems, planets, and sectors; investment determines detail level. | Star System Selection Map, Celestial Body Details Modal, Surface Scan Results |
|                                   | **System & Planetary Mapping**         | Gradual revelation of star system data and planetary surfaces through probes, satellites, and expeditions. | Star System Map, Orbital View, Surface Map                                    |
|                                   | **Terrain & Sector Exploration**       | Robotic or manned expeditions reveal terrain, hazards, and resources; influences colony site selection.    | Surface Map (All Sectors), Sector Details                                     |
| **Travel & Events**               | **Ship Voyage Simulation**             | Background time passage with random/narrative events affecting arrival conditions (morale, integrity).     | Ship Journey Log                                                              |
|                                   | **Event-Driven Decision System**       | Dynamic narrative events triggered by game state, player decisions, and random factors.                    | Event/Encounter Modals, Event Log                                             |
| **Colonization & Infrastructure** | **Landing Operations**                 | Risk-based landing sequence influenced by ship type, cargo, and sector conditions.                         | Orbital View, Surface Map                                                     |
|                                   | **Construction & Expansion**           | Deploy and upgrade structures, roads, tunnels, and utilities using robotic and human labor.                | Sector Map, Building Details Modal                                            |
|                                   | **Power & Life Support Networks**      | Manage generation, storage, and distribution of essential systems (power, air, heat, water).               | Sector Map, Colony Overview                                                   |
| **Resource & Economy**            | **Resource Extraction**                | Mine, harvest, and refine materials from terrain; limited by energy, logistics, and labor.                 | Sector Map, Production Chain Config                                           |
|                                   | **Manufacturing Chains**               | Convert raw materials to products and goods; visualize and optimize through flow diagrams.                 | Production Chain Config Screen                                                |
|                                   | **Trade & Logistics**                  | Manage inter-settlement transport routes and balance supply-demand across colonies.                        | Trade/Logistics Overview                                                      |
| **Automation & Control**          | **Task Scripting**                     | Create YAML-like automation scripts for routine tasks or systems (if/then/repeat logic).                   | Automation Config Modal                                                       |
|                                   | **Vehicle & Drone Automation**         | Assign roles, automation profiles, or manual control; handle maintenance and upgrades.                     | Vehicle Overview, Vehicle Details                                             |
|                                   | **Building Automation**                | Toggle automated production cycles or shutdown/mothball under resource pressure.                           | Building Details                                                              |
| **Population & Morale**           | **Crew Assignment & Labor Allocation** | Assign colonists to jobs or roles; manage fatigue, specialization, and morale impacts.                     | Settlement Details, Colony Overview                                           |
|                                   | **Social Dynamics / Events**           | Population morale and cultural development influenced by events, governance, and environment.              | Policy Screen, Event Modals                                                   |
| **Research & Progression**        | **Research Projects**                  | Allocate labs and resources to research or engineering tasks to unlock new tech.                           | Research Screen                                                               |
|                                   | **Tech Tree Progression**              | Unlock improvements in automation, structures, vehicles, and efficiency.                                   | Research Screen, Colony Overview                                              |
| **Governance & Policy**           | **Policy Management**                  | Enact rules for economy, rationing, ethics, or research priorities; affects morale and efficiency.         | Policy & Governance Screen                                                    |
|                                   | **Colonial Governance Evolution**      | Unlock new governance forms or specializations (corporate, democratic, AI-led, etc.).                      | Policy & Governance, Colony Overview                                          |
| **System Expansion**              | **Multi-Colony Management**            | Establish additional outposts; manage resource flows and coordination.                                     | System Overview, Trade/Logistics                                              |
|                                   | **Orbital Construction**               | Build and upgrade stations and satellites for support and expansion.                                       | Orbital View, Colony Overview                                                 |
| **Analytics & Meta Systems**      | **Event Log & Codex**                  | Persistent record of player actions, colony history, and contextual help.                                  | Event Log, Help/Codex                                                         |
|                                   | **Statistics & Achievements**          | Performance tracking and long-term progression goals.                                                      | Analytics Screen, Achievements Screen                                         |
|                                   | **Save/Load & Replayability**          | Full state serialization with quick save/load and mod compatibility.                                       | Pause Menu, Load/Save Screens                                                 |

```
flowchart TD
    A[Exploration] --> B[Discovery Data]
    B --> C[Mission Planning]
    C --> D[Ship Voyage]
    D --> E[Arrival & Scanning]
    E --> F[Landing]
    F --> G[Construction & Infrastructure]
    G --> H[Resource Extraction]
    H --> I[Production & Logistics]
    I --> J[Automation]
    J --> K[Colony Stability]
    K --> L[Research & Policy]
    L --> M[System Expansion]
    M --> N[Analytics & Achievements]
    N --> A

```


This document defines the **core systems** and **gameplay mechanics** derived from the game concept and screen architecture.  
Each system represents a simulation layer of interacting mechanics within the colony simulation.  
Mechanics are annotated with the **simulation layer** they operate in (e.g., *Ship*, *System*, *Planet*, *Sector*, *Colony*, *Global*).

> Future Work: Insert detailed rules, formulas, and algorithms for each subsystem under the indicated annotations.

## Exploration System

Mechanics related to discovery, mapping, and reconnaissance of space and planetary environments.

### Probe Launch & Data Resolution  
**Layer:** System / Ship  
- Player builds and launches probes to nearby stars or planetary bodies.  
- Data quality depends on probe type, mission duration, and investment.  
> Add formula for probe data fidelity based on mission duration, equipment quality, and communication delay.

### System & Planetary Mapping  
**Layer:** System / Planet  
- Reveals planetary or moon data gradually as probes and ships gather telemetry.  
- Map layers include topography, temperature, atmosphere, and resource overlays.  
> Define map resolution scaling formula based on data completeness and instrument quality.

### Terrain & Sector Exploration  
**Layer:** Planet / Sector  
- Robotic or crewed expeditions explore individual sectors, uncovering resources and hazards.  
> Add rule for exploration risk, time cost, and probability of discovery events.

## Voyage & Event System

Handles interstellar travel, shipboard simulation, and narrative events.

### Ship Voyage Simulation  
**Layer:** Ship  
- Simulates time passage over years/centuries with random and deterministic events.  
> Create function to simulate ship integrity and population health decay over time.

### Event-Driven Decision System  
**Layer:** Global / All Layers  
- Narrative or emergent events triggered by conditions, random chance, or player actions.  
> Define event trigger schema: (Condition, Trigger Type, Probability, Consequence).  
> Create event resolution formula for morale, damage, or discovery outcomes.

## Colonization System

Covers landing operations, initial setup, and base establishment.

### Landing Operations  
**Layer:** Planet / Sector  
- Manages descent, landing accuracy, and cargo delivery success.  
> Add formula for landing success probability based on sector weather, gravity, and ship type.

### Construction & Expansion  
**Layer:** Sector / Colony  
- Uses available workforce and robots to deploy modular structures and infrastructure.  
> Define construction time and cost equations for structure classes and workforce efficiency.

### Power & Life Support Networks  
**Layer:** Colony  
- Manages interconnected systems for energy, oxygen, temperature, and water.  
> Add simulation model for network flow and redundancy failures.

## Resource & Economy System

Handles extraction, manufacturing, and logistics between colonies.

### Resource Extraction  
**Layer:** Sector / Colony  
- Mines and refineries extract and process natural resources.  
> Add resource yield formula based on deposit richness, equipment tier, and efficiency modifiers.

### Manufacturing Chains  
**Layer:** Colony  
- Converts raw materials to processed goods through production chains.  
> Define production node efficiency equations and failure conditions.  
> Specify production bottleneck logic for limited inputs.

### Trade & Logistics  
**Layer:** System / Colony  
- Configures inter-settlement trade routes and transport efficiency.  
> Add formula for travel time, transport capacity, and fuel/resource costs per trip.  
> Define algorithm for automatic route optimization.

## Automation & Control System

Supports scripted behaviors, repeatable tasks, and autonomous agents.

### Task Scripting  
**Layer:** Global / Colony  
- YAML-like automation scripts define trigger-action workflows.  
> Create a grammar definition for automation scripts (Trigger, Condition, Action).  
> Add simulation for CPU or command latency constraints (optional realism mechanic).

### Vehicle & Drone Automation  
**Layer:** Sector / Colony  
- Assigns vehicles automation templates; includes maintenance, upgrade, and task queues.  
> Define vehicle wear-and-tear function and maintenance frequency model.

### Building Automation  
**Layer:** Colony  
- Allows toggling of automated building operation and production pause/resume.  
> Define automation threshold for resource availability or workforce shortages.

## Population & Morale System

Simulates human factors, labor allocation, and social dynamics.

### Crew Assignment & Labor Allocation  
**Layer:** Colony  
- Assigns colonists to tasks, balancing fatigue, skill, and morale.  
> Add productivity formula based on skill level, morale, and environmental conditions.

### Social Dynamics / Events  
**Layer:** Colony / Global  
- Manages morale, unrest, and psychological effects of isolation.  
> Define morale decay/recovery rates and thresholds for mutiny, depression, or celebration events.

## Research & Progression System

Controls technology advancement and unlockable gameplay tiers.

### Research Projects  
**Layer:** Colony / Global  
- Allocate labs and scientists to unlock technologies or projects.  
> Add research point accumulation formula and breakthrough chance function.

### Tech Tree Progression  
**Layer:** Global  
- Sequential or branching technology paths modify efficiency and unlock features.  
> Define dependency graph for tech nodes; specify cost escalation per tier.

## Governance & Policy System

Implements player-driven control over social, ethical, and economic factors.

### Policy Management  
**Layer:** Colony / Global  
- Enacts and adjusts rules for economy, rationing, and ethics.  
> Create influence model showing policy effects on morale, output, and stability.  
> Add cooldown mechanics for policy changes.

### Colonial Governance Evolution  
**Layer:** Global  
- Unlocks new government models (democracy, corporate, AI-led, etc.).  
> Define governance tree with requirements and effects.

## System Expansion System

Simulates growth beyond the initial colony.

### Multi-Colony Management  
**Layer:** System / Global  
- Establishes additional outposts and manages coordination between them.  
> Add scaling model for administration overhead and communication lag.

### Orbital Construction  
**Layer:** System  
- Builds and maintains orbital infrastructure for transport, power, or research.  
> Define orbital construction resource and power requirements.

## Analytics & Meta System

Manages history, player progress, and performance visualization.

### Event Log & Codex  
**Layer:** Global  
- Tracks player choices, events, and narrative outcomes.  
> Define data retention and log categorization model.

### Statistics & Achievements  
**Layer:** Global  
- Records performance data and unlocks achievements.  
> Add scoring algorithm for milestones and conditions.

### Save/Load & Replayability  
**Layer:** Global  
- Manages state serialization for persistence and mod compatibility.  
> Define schema for save-state versioning and compression.

# Cross-System Interactions

Many systems influence each other across layers:

- **Exploration** feeds data into **Colonization** and **Resource** systems.  
- **Research** enhances **Automation**, **Economy**, and **Governance**.  
- **Population** morale affects **Productivity**, which feeds into **Economy** outcomes.  
- **Events** can occur in any system and modify global state.

> Add dependency graph or interaction matrix between systems once mechanics formulas are defined.

# Future Extensions

- Diplomacy and Trade Networks (inter-colony or interstellar).  
- Environmental Simulation (climate, radiation, contamination).  
- AI Faction or Rival Colonies (late-game layer).  
- Persistent Galactic Metagame (multi-system expansion).

> Placeholder for future design phases.

 ## System Dependency Matrix

Rows **influence** columns. A filled cell means the row system produces state/data/resources that the column system **consumes**.

**Legend**
- **D** = data/state
- **R** = resources/throughput
- **E** = events/triggers
- **P** = policy/control parameters
- **H** = human factors (morale, labor)
- **T** = tech unlocks/modifiers
- **L** = logistics/routes/topology
- **S** = save/telemetry/meta

| ⬇️ Produces \\ Consumes ➡️ | Exploration | Voyage & Events | Colonization | Resource & Economy | Automation & Control | Population & Morale | Research & Progression | Governance & Policy | System Expansion | Analytics & Meta |
|---|---|---|---|---|---|---|---|---|---|---|
| **Exploration** | — | E,D | D | D | D | E | D | D | D | S |
| **Voyage & Events** | D | — | E,D | E | E | E | E | E | E | S |
| **Colonization** | D | D | — | R,D | D | H,D | D | D | L,D | S |
| **Resource & Economy** | — | — | R | — | R,D | H (via scarcity) | D (labs res.) | D (budget inputs) | R,L | S |
| **Automation & Control** | — | — | D | R,D | — | H (workload) | D (throughput) | D (execution stats) | L (auto routes) | S |
| **Population & Morale** | — | E | H | H (labor) | H (uptime) | — | H (research speed) | H (legitimacy) | H (admin cost) | S |
| **Research & Progression** | T (sensors) | T (mitigations) | T (buildings) | T (processes) | T (scripts/AI) | T (amenities) | — | T (governance) | T (infrastructure) | S |
| **Governance & Policy** | P (funding) | P (risk posture) | P (zoning) | P (tax/ration) | P (automation caps) | P (rights/quarters) | P (priorities) | — | P (charters) | S |
| **System Expansion** | D (new targets) | D | L,D | R,L | L (global plans) | H (migration) | D (projects) | P (federalization) | — | S |
| **Analytics & Meta** | S | S | S | S | S | S | S | S | S | — |

---

## Inputs / Outputs by System (Quick Reference)

| System | Key **Outputs** | Primary **Inputs** |
|---|---|---|
| **Exploration** | Maps, scan fidelity (**D**), anomalies (**E**) | Probe tech (**T**), policies (**P**), events (**E**) |
| **Voyage & Events** | Incident stream (**E**), arrival modifiers (**D**) | Risk policy (**P**), ship condition, crew (**H**) |
| **Colonization** | Buildable sites & facilities (**D**), workforce allocation (**H**) | Maps (**D**), policies (**P**), tech (**T**) |
| **Resource & Economy** | Materials/energy throughput (**R**), inventories (**D**) | Workforce (**H**), utilities, policies (**P**) |
| **Automation & Control** | Task plans, execution stats (**D**), auto-routes (**L**) | Scripts, thresholds (**P**), capacities (**R**) |
| **Population & Morale** | Productivity modifier (**H**), social events (**E**) | Living conditions, workload, policies (**P**) |
| **Research & Progression** | Unlocks/modifiers (**T**), blueprints (**D**) | Labs, inputs (**R**), priorities (**P**), morale (**H**) |
| **Governance & Policy** | Constraints, budgets, priorities (**P**) | Analytics, stability (**H**), economy (**R**), events (**E**) |
| **System Expansion** | New nodes/routes (**L**), inter-settlement plans (**D**) | Surplus (**R**), authority (**P**), tech (**T**) |
| **Analytics & Meta** | Telemetry, KPIs, save snapshots (**S**) | All systems’ state/streams |

> When you formalize formulas and rules, use this matrix to validate **dataflow** (row → column) and prevent circular dependencies without buffers or delays.