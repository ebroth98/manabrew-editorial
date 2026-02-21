# Project Guidelines

Never push directly on main, always create a feature branch and a relative pr

## Feature Tracking

Whenever you implement, partially implement, or modify a feature in this repository, you **must** update `features.md` accordingly:

- Mark newly implemented features as **Implemented** and note the corresponding Rust file(s).
- Mark features with incomplete coverage as **Partial** with a brief note on what exists.
- If a feature's status changes (e.g. from Partial to Implemented), update the status and description.
- Keep the Summary Statistics section at the bottom of `features.md` accurate after any changes.

## Project Structure

- **UI**: Tauri + Swift (macOS/iOS) app in the repo root and `src-tauri/`
- **Engine**: Rust crates under `forge-engine/` — the game engine being ported from Java (Forge)
- **Reference**: Original Java source lives in `forge/forge-game/src/main/java/forge/game/`
- **Feature map**: `features.md` tracks Java-to-Rust porting progress across all subsystems
