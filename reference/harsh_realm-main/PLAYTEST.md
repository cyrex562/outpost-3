# Harsh Realm — Quick Start Guide

Welcome to the playtest! Harsh Realm is now available as a standalone desktop application. Follow the instructions below to get started and help shape the future of the game.

## 🚀 Launching the App

There are two ways to run the application during development and playtesting:

### 1. Run via Developer CLI (Recommended for Testing)
This method runs the application from the source code.
1.  Ensure you have your Python virtual environment active.
2.  Run the following command:
    ```bash
    hrctl desktop
    ```
    *This will start the backend in the background and open the native game window.*

### 2. Run the Standalone Executable
If you have already built the standalone version:
1.  Navigate to the `dist/` directory.
2.  Run `HarshRealm.exe`.
    *Note: If you haven't built it yet, you can do so by running `hrctl build-desktop`.*

---

## 🕹️ Gameplay Quick Start

1.  **Create/Load a World**: On launch, use the World Manager to create a new procedural world or load an existing one.
2.  **Character Creation**: Once a world is loaded, you will be prompted to create your character. Roll your attributes and assign your skills.
3.  **Exploration**:
    *   Use **8-directional movement** (e.g., `n`, `s`, `ne`, `sw`) to travel across the hex map.
    *   Use `look` to see your current surroundings and weather.
    *   Use `search` to look for hidden features or items in a cell.
4.  **Interaction**:
    *   `enter`: Enter settlements, dungeons, or lairs.
    *   `talk`: Speak with NPCs in settlements.
    *   `shop`: Visit local merchants.
5.  **Status**: Type `status` or check the sidebar for your health, XP, and current atmospheric conditions.

---

## 🛠️ Feedback & Playtesting

As you play, please consider the following areas for feedback:
- **Atmosphere**: Does the new weather and narration system feel immersive?
- **UI/UX**: Is the Map Legend helpful? Are the window layouts clear?
- **Progression**: Does the flow from exploration to combat/town feel natural?
- **Data/Content**: What types of locations, items, or creatures would you like to see next?

### Future Direction
After this playtest, we will focus on:
- Improving tools for adding new data, content, and rules.
- Expanding the variety of procedural encounters.
- Refining the expert-system GM's narration capabilities.

---

**Happy adventuring!**
