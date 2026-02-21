# Project Guidelines

- Never push directly on main, always create a feature branch and a relative pr
- never automatically push any sort of code wait for an explicit push command

## Feature implementation
Whenever you implement a feature, or modify, make sure to always keep logical, naming, and file parity, with the java counterpart in forge/forge-game/ or forge/forge-core.
More importalty, you must keep equal (identical structure of file, and interface with the files inside forge/forge-game/src/main/java/forge/game).
You cannot make up different file names.

## Feature Tracking

Whenever you implement, partially implement, or modify a feature in this repository, you **must** update `features.md` accordingly:

- Mark newly implemented features as **Implemented** and note the corresponding Rust file(s).
- Mark features with incomplete coverage as **Partial** with a brief note on what exists.
- If a feature's status changes (e.g. from Partial to Implemented), update the status and description.
- Keep the Summary Statistics section at the bottom of `features.md` accurate after any changes.
- whenever relevant update or create a new preset deck in src-tauri/src/game_manager.rs so that it will be possible to test the newly introduced mechanic 

## Project Structure

- **UI**: Tauri + Swift (macOS/iOS) app in the repo root and `src-tauri/`
- **Engine**: Rust crates under `forge-engine/` — the game engine being ported from Java (Forge)
- **Reference**: Original Java source lives in `forge/forge-game/src/main/java/forge/game/`
- **Feature map**: `features.md` tracks Java-to-Rust porting progress across all subsystems
