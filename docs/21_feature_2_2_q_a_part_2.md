## 1. Orbital Parameters & Motion

### When generating orbital parameters for bodies, what properties do we need?

#### Semi-major axis (distance from star)?

Yes

#### Orbital period (or should this be calculated from distance)?

Yes

#### Starting angle/phase (where on the orbit the body starts)?

Yes

#### Eccentricity (circular vs elliptical orbits), or keep all circular for now?

Yes

### Should orbital motion be time-based (updates with game ticks) or real-time animation?

Updates with game ticks, this can be very approximate.

## 2. Game Mode Context

### How do we distinguish between "new game setup mode" vs "playing game mode"? Is there a GameMode enum or flag in GameState? Or do we check if a colony has been established yet?

Once the player clicks launch the colony from the star system selection screen they should be taken to the star system journey log screen. After the ship completes its journey, they will click next and be taken to the star system screen. Once they've gone through that sequence of steps there is no returning to the star ship journey log or galaxy map.

### Should we create a Navigation Stack system to track screen history? Store as List<ScreenId> in GameState? Commands like PushScreen, PopScreen, NavigateToScreen?

Yes. Store it as a list in First-in Last-out order (stack) and have commands for push, pop, peek, and navigate, as well as events when the screen navigation takes place

## 3. System Overview Modal Panel

### What tabs should be included initially? Bodies List (with quick-select)? Messages/Notifications? System Properties? Other Tabs?

For now a list of bodies and a list of messages/notifications

### Should the toggle state (open/closed) be persisted in GameState?

Yes. Store the last state it was in.

### Should selecting a body from the list also highlight it on the map?

Yes.

## 4. Procedural Generation Trigger

### When first viewing a system, what needs to be generated?

#### Number and types of bodies (terrestrial, gas giant, ice, dwarf)?

Yes. Number of bodies should be known from the probe launch mechanic. Their types and orbital parameters need to be generated.

#### Orbital distances and periods?

Yes

#### Asteroid belts and Oort cloud positions?

Yes

#### Body properties (size, mass, atmosphere, resources)?

Just size and mass. For atmosphere and resources, they can be generated after the planet has been visited by a probe or the player's ship.

#### Should this happen via a Command like GenerateSystemDetails that produces a SystemDetailsGenerated event?

Yes commands and associated events

#### Should generation be deterministic (seeded by system ID)?

This is an interesting idea. The current system ID is a ULID and does not contain any useful data. However, a Seed value should be calculated and displayed composed of Numbers and Letters, case insensitive (displayed in / generated in all caps)

## 5. Camera State Persistence

### Should camera state be per-system (different zoom/pan for each system map)?

No.  if the player is not playing a game session, dont persist the camera state. 

#### What data structure: Dictionary<SystemId, CameraState> with CameraState { Pan, Zoom }?

Yes.

#### Should this be in a separate ViewState or part of main GameState?

Part of the GameState

## 6. Hotkeys

### What hotkey for "Reset Camera"? (Default to Home or R?)

Home key

### Should hotkeys be configurable or hardcoded for Session 2.2?

### Any other hotkeys needed (pause, speed controls, open system overview panel)?

Pause = Pause, Space
+ = faster game speed
- = slower game speed

WASD/direction arrows pan

Z/X zoom in and out

Esc go back/close open modal

## 7. Body Selection & Highlighting

### What visual feedback for selected body? Outline/glow around the icon? Change color or add selection ring?

Display a selection ring

#### Show info panel below/beside it?

No

### Can only one body be selected at a time?

Yes, for now. there may be special commands in the future requiring the selection of multiple bodies

### Should clicking empty space deselect the current body?

Yes

## 8. Asteroid Belts & Oort Cloud

### For the annulus shape representing belts/clouds:

#### Should they have visual particles/dots, or just a colored ring?

Just a colored ring

#### Should they rotate/animate?

No

#### Should they be clickable to show info?

Yes, just like a planet or the star

#### How many belts/clouds per system (0-2 belts, always 1 Oort cloud)?

At least an oort cloud, and possibly additional asteroid belts in-system where a planetary body is generated.

## 9. Star Rendering

### Should the star have:

#### A solid color circle? Gradient (brighter at center)? Glow/bloom effect (shader)? Animated surface (just for visual flair)?

For now just a solid color circle. In the future this will likely be a sprite with animated texture and additional effects.

#### What size on screen (fixed pixel size, or scales with zoom)?

This should scale by zoom, pick a good initial size for planets and stars to make them both visible at the greast zoom out level, and then prorportionally increase their size with zoom in.

## 10. Time & Speed Controls

### Should the Star System Map have time controls (pause, 1x, 2x, 5x, etc.)? Or does it inherit the current game speed from GameState?

Yes there should be a set of controls for pausing/resuming, increasing, an decreasing game speed.

### Should pausing the game freeze orbital motion?

Yes

### Should the speed be displayed in the header panel?

Yes