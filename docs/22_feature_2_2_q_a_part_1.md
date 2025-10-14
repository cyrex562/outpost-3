## 1. Scene Entry Point & Navigation

### Should this scene be accessible from the System Details Modal (after "View System" button is clicked)?

Yes, but only if a probe has been launched, arrived at the system, and basic information about the bodies present is known.

### Should there also be direct access from the Galaxy Map when a discovered system is selected?

No.

### When clicking "Back", should it return to the Galaxy Map or to the previous screen?

The previous screen needs to be tracked, so that when playing a game session it would go to the previous screen viewed or do nothing if this is the last screen.
when creating a game, back should just go back to the star system selection screen.

## 2. Visual Representation

### Should the star be rendered as a sprite/circle at the center with a glow effect?

start with a large circle or sprite that is the correct spectral type color for the star.

### Should orbits be drawn as simple circles/ellipses, or do they need to show orbital paths with dots/lines?

Draw them as simple circles at the correct distance from the star based on the current zoom level.

### Should celestial bodies (planets/moons) be: Static icons at fixed positions on their orbits? Animated to show orbital motion?

Celestial bodies should move around the star based on their orbital parameters (generated when the system is generated) and the current game speed.

#### Sized proportionally to their actual size, or stylized for visibility?

Stylize them for visibility.

## 3. Clickable Body Interaction

### When clicking a body icon, should it:

#### Open the Celestial Body Details Modal (from 02_screens.md)? Just highlight/select it and show info in a side panel? Both (select + show panel, then button to open full modal)?

single click should just highlight/select the body. double click should open a modal with details. a button should be available to open the details modal for a selected body as well.

### Should there be visual feedback for hover states (tooltip with name/type)?

Yes. Hovering over a body should show basic information like type, distance from star, name.

## 4. Camera Controls

### What zoom levels should be supported (min/max)?

zoomed out to the extents of the star system including its Oort cloud equivalent (very small). Zoomed in to show individual bodies in their orbits distinguishable from each other.

### Should zoom be centered on mouse position or on screen center?

Zooming should always center on the mouse position

### Should there be a "Reset Camera" button to return to default view?

Yes a button on the UI as well as a hotkey

### Should pan be drag-with-mouse, or also support keyboard arrows/WASD?

Both. Right-click + drag to pan or WASD or direction keys or whatever is mapped

## 5. Data Integration

### Should this scene read from a selected system in GameState (e.g., SelectedSystemId)?

Yes. On first access, any additional information that needs to be generated for the system should be generated. This information should be persisted in the GameState for use in other scenes.

### If no system is selected, should it show an error or return to Galaxy Map?

The button to view the system should only be visible from the system details modal.

### Should it display system properties (name, spectral class) in a header/title area?

In the top header panel display the system's name, the current game time, and current game speed

## 6. Bodies Display

### Should it show ALL bodies in the system, or only discovered/explored ones?

Show all bodies. For asteroids and clouds, show an annulus shape that is the approx width of the belt/cloud at its correct orbit

### For bodies with moons, should moons be visible as smaller icons on sub-orbits?

No

### Should there be a legend or filter to show/hide certain body types?

No.

## 7. UI Layout

### Should there be a persistent UI panel showing:

#### System name and star properties?

Yes. a top panel with information

### List of bodies with quick-select buttons?

No. not persistent, but something a player can choose to display on the right side of the screen. by toggling it open. A system overview modal panel with tabs for different sets of information.

### Messages/notifications panel?

A tab that is a part of teh system overview modal panel

### Should the "Back to Main Menu" button actually go to Main Menu, or to Galaxy Map?

See the answer to this question above. It depends on the current game mode. If the player is playing the game itself, the back button needs to take them to the previous screen in a stack of previous screens. If there isnt a previous screen it should do nothing. If the player is in the new game settings, then it should take them back to the Galaxy map.

## 8. State & Commands

### Do we need new Commands like SelectCelestialBody or ViewSystemMap?

Yes. there are likely a number of Commands that need to be created.

### Should viewing this scene trigger any events (e.g., SystemMapViewed)?

Yes. there are likely a number of Events that need to be created

### Should camera position/zoom be saved in GameState for persistence?

Yes, when navigating to a menu/new scene, returning to the System map should show the same pan location and zoom level.

## 9. Architecture Alignment

### Should this be a Presenter pattern (like StarMapPresenter) that subscribes to projections?

Yes

### What projection(s) should it use? (e.g., SelectedSystemProjection, CelestialBodiesProjection)?

Unknown

### Should the scene have a separate .tscn file in godot-project/Scenes/StarSystem/?

Yes there should be a StarSystemMap.tscn file as well as any additional component definitions needed.

## 10. Testing Scope

### Should we write xUnit tests for any new domain logic (projections, calculators)?

No. Write GDUnit4 tests for this code.

### Should we include manual testing steps for UI interaction?

No. 

### Do we need test data (a mock system with multiple planets/moons)?

Planets and moons will be procedurally generated when a system is accessed for the first time.