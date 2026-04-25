The role of this file is to describe common mistakes and
confusion points that agents might encounter as they work in
this project. If you ever encounter something in the project
that surprises you, please alert the developer working with you
and indicate that this is the case in the AgentMD file to help
prevent future agents from having the same issue.

# Project Guidelines

- Never push directly on main, always create a feature branch and a relative pr
- never automatically push any sort of code wait for an explicit push command
- always reference docs/STYLE_GUIDELINES.md for any kind of ui work
- Do not write unit tests unless explicitely asked to do so
- When opening a PR, follow the structure in `.github/pull_request_template.md`: every PR body must include the **Summary**, **Why**, **Test plan**, and **Build artifacts** sections in that order. Check the `Build macOS .dmg` and/or `Build Windows .exe` boxes independently based on which installers the PR should produce on merge to main; leave unchecked otherwise to save CI time.
- All commits MUST follow [Conventional Commits](https://www.conventionalcommits.org/) — enforced by the `commit-msg` git hook (`commitlint` + `@commitlint/config-conventional`). Format: `<type>(<scope>)?: <subject>`. Allowed types: `feat`, `fix`, `chore`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `revert`. Subject lowercase, no trailing period, ≤ 72 chars. Examples: `feat(engine): wire mana cost parser`, `fix(ui): clamp deck list height on small screens`, `chore: bump prettier`. Breaking changes use `!` after type/scope or a `BREAKING CHANGE:` footer. Do not bypass with `--no-verify`.

## Feature implementation

Whenever you implement a feature, or modify, make sure to always keep logical, naming, and file parity, with the java counterpart in forge/forge-game/ or forge/forge-core.
More importalty, you must keep equal (identical structure of file, and interface with the files inside forge/forge-game/src/main/java/forge/game).
You cannot make up different file names.
If you're working on a moudle, make sure to use the scan-feature-parity skill to track progress.

## File structure and naming precision

When implementing or modifying code, you MUST follow the existing file structure and symbol naming with precision. Do not guess or invent file paths, function names, struct names, or module organization. If you are unsure about the correct name, path, or structure, use the `scan-feature-parity` skill to verify before writing code. Getting file names or symbol names wrong causes merge conflicts and breaks parity with the Java reference.

## Agent Workflow

For any non-trivial development task (new features, significant refactoring, multi-file changes), you MUST load the `crew-orchestrator` skill first and follow its full workflow:

1. **DISCOVER** — Load `crew-context-scout` to understand project patterns
2. **PLAN** — Load `crew-task-planner` to break the task into subtasks
3. **USER APPROVAL** — Present the plan and wait for approval before coding
4. **EXECUTE** — Load `crew-coder` for each subtask
5. **REVIEW** — Load `crew-reviewer` to check for security and quality issues
6. **TEST** — Load `crew-test-engineer` to generate and run tests

Do NOT skip straight to `crew-coder`. The orchestrator ensures quality through planning, review, and testing stages. Only skip the orchestrator for trivial tasks (one-liner fixes, simple questions, file renames).

## Project Structure

- **UI**: Tauri + Swift (macOS/iOS) app in the repo root and `src-tauri/`
- **Engine**: Rust crates under `forge-engine/` — the game engine being ported from Java (Forge)
- **Reference**: Original Java source lives in `forge/forge-game/src/main/java/forge/game/`
- **Feature map**: `features.md` tracks Java-to-Rust porting progress across all subsystems
