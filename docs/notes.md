# Notes and Ideas for Outpost 3

## Scenes

### Debug Panel

- the debug panel should be displayable from any scene using the `~` key or the equivalent binding.
- The debug panel should have a prompt for entering commands
- the debug panel should have a search and filter bar at the top
- the debug panel should be toggleable enabled/disabled from game settings (either file or menu)

### Star System Selection Scene

- add tooltips to hovering over stars with basic known stats about star: name, spectral class, number of bodies if known
- show a notification and possibly flash the target body when a probe arrives
- add a debugging command, valid only on this scene to toggle a system explored/unexplored
- events that happen to launched probes

### Ship Journey Log

- events that happen during the ship's journey

### Planet Map Scene

#### New Site Mode

- instead of sectors, player can click any point on the map to create a new site; maybe have a maximum number of sites per planet based on site size or something like that.

## Game Mechanics

- Colonists should have mini character sheets to track their abilities and experience. How would this scale in-game if we have billions of colonists? is there a better way to model this but maintain high fidelity in event generation and outcomes?
- procedurally generate planet map.
- procedurally generate the site
- site size can expand over time.
- sites can be linked with infrastructure and routes for trade like rails, highways, etc.
- big infra projects should be possible to improve planet-wide infrastructure like bridges, roads, rail, canals, dams, etc.
- research does more than science. Engineering teams take known science and apply it to building versions of systems and platforms tailored to the colonized environment. those are also "discoveries"

### Events

- event effects, event description/narrative, link to image/animation to display, choices and effects of choices with probabilities if random
- specific types of events should be loaded from disk in a YAML format to enable modding support and content creation support
- in logging display event logs should have specific colors for types of events, styling of text, nerdfont-like symbols and emoji
- events should have pre-requisites to make sure they occur under the right circumstances like what scene, what is going on, state of the game, etc. Ex: some events can happen to a ship that is a generation ship but may not make sense to happen to a sleeper ship and vice versa.
- Events should take into account things like the skills of colonists. 