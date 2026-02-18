assume the api the server exposes is compatible with our client (we will create a middleware for it to adapt to our needs)

use this stack:
React
Vite
Tailwind
Shadcn/UI
Tanstack Query
Zustand

add as many details as possible to the plan.md file since we will use this plan to iterate with prompts.

The plan should be structured in phases and each phase should have a list of tasks.

The output should be only the plan.md file content.

```markdown
# Implementation Plan: Web-Based XMage Client

## Project Overview
**Goal:** Reimplement the XMage client as a modern web application using React.
**Target:** Feature parity with the existing Java Swing client, connected to standard XMage servers via a custom middleware adapter.
**Stack:**
- **Framework:** React + Vite
- **Styling:** Tailwind CSS
- **Components:** Shadcn/UI
- **State Management:** Zustand
- **Data Fetching/Async:** Tanstack Query
- **API Interaction:** WebSocket (primary) & REST (secondary via Middleware)

---

## Phase 1: Project Initialization & Architecture

**Objective:** Set up the development environment, directory structure, and foundational architecture for state and API communication.

1.  **Scaffold Project**
    - Initialize Vite project with React (TypeScript).
    - Install and configure Tailwind CSS.
    - Initialize Shadcn/UI and install core components (Button, Input, Card, Dialog, DropdownMenu, ScrollArea, Separator, Toast, Skeleton).
    - Configure path aliases (e.g., `@/components`, `@/stores`, `@/lib`).

2.  **Define Architecture & Types**
    - Create TypeScript definitions mirroring XMage core objects (`Card`, `Deck`, `Player`, `Table`, `GameView`, `ClientCallback`).
    - **Task:** Create a shared type library for API responses expected from the Middleware.

3.  **Network Layer Setup**
    - **Task:** Implement a WebSocket client singleton (using native WS or Socket.io client depending on middleware implementation) to handle persistent connection.
    - **Task:** Configure Tanstack Query `QueryClient` for REST endpoints (e.g., Card Database searching, Server Status).
    - **Task:** Create a "Middleware Mock" service to simulate server events for UI development without a running backend.

4.  **Global UI Layout**
    - Create `AppShell` layout (Sidebar/Navigation, Main Content Area, Status Bar).
    - Implement Client-side Routing (React Router or Tanstack Router) for:
        - `/login`
        - `/lobby`
        - `/deck-editor`
        - `/game/:gameId`
        - `/draft/:draftId`

---

## Phase 2: Authentication & Server Connection

**Objective:** Allow users to connect to the middleware, select a server, and authenticate.

1.  **State Management (Zustand)**
    - Create `useConnectionStore`: Handle socket status (Connecting, Connected, Disconnected, Reconnecting).
    - Create `useAuthStore`: Handle user session, username, preferences, and server selection.

2.  **Login Screen UI**
    - Design a modern Login form using Shadcn `Card` and `Form`.
    - Features: Server address input, Username, Password, "Register" toggle, Flag/Icon selection.

3.  **Connection Logic**
    - Implement handshake logic with the Middleware.
    - Handle authentication errors and display via Shadcn `Toast`.
    - Persist last used server/username in `localStorage`.

---

## Phase 3: The Lobby (Community & Matchmaking)

**Objective:** Replicate the functionality of the XMage Tabs (Tables, Chat, Users).

1.  **Lobby Layout**
    - Create a multi-pane layout:
        - Left: Available Tables (Matches).
        - Right: Connected Users.
        - Bottom/Overlay: System & Lobby Chat.

2.  **Chat System**
    - **Task:** Create `ChatComponent` with support for different message types (System, User, Whisper).
    - **Task:** Implement auto-scroll and "new message" indicators.
    - **Task:** Integrate user color coding based on XMage logic.

3.  **Table Management**
    - **Task:** Create `TableList` component using Tanstack Table (optional) or mapped Shadcn Cards.
    - **Task:** Implement filtering (Format, Rated/Unrated, Deck Type).
    - **Task:** Create "New Table" Dialog:
        - Inputs: Name, Password, Match Time, Format (Standard, Modern, Commander, etc.), Number of Wins.
        - Skill level selector.

4.  **User List**
    - Display users with status icons (In Game, Drafting, Lobby).
    - Implement context menu on users (Whisper, Watch Game, Profile).

---

## Phase 4: Deck Editor

**Objective:** A full-featured deck builder with search and import/export capabilities.

1.  **Card Database Service**
    - **Task:** Implement Tanstack Query hooks to search cards via the Middleware (which queries the XMage DB).
    - **Task:** Handle pagination and caching of card data.

2.  **Editor UI**
    - **Layout:** Split screen. Top: Card Search/Pool. Bottom: Deck (Main/Sideboard).
    - **Card Rendering:** Create `CardComponent` that displays the image (lazy loaded from Scryfall/Middleware) with a text-fallback tooltip for accessibility/loading states.

3.  **Deck Management Logic**
    - Create `useDeckStore` to manage current deck state.
    - **Task:** Implement Drag-and-Drop (using `dnd-kit`) to move cards between Pool, Main, and Sideboard.
    - **Task:** Implement Stats View (Mana curve charts, Color distribution).

4.  **I/O Operations**
    - **Task:** Implement `.dck` file parsing (XMage format) for import.
    - **Task:** Implement clipboard export/import (Arena format compatibility).
    - **Task:** Save/Load decks from the server.

---

## Phase 5: Game Interface (The Battlefield)

**Objective:** The core visual representation of Magic: The Gathering. This is the most complex phase.

1.  **Game Layout Engine**
    - Design a responsive grid system for the battlefield.
    - **Areas:**
        - Opponent Hand (Face down) & Library.
        - Opponent Battlefield.
        - **The Stack** (Crucial UI element).
        - Player Battlefield.
        - Player Hand.
        - Sidebar: Life totals, Turn phase indicator, Mana pool, Graveyard/Exile/Command Zone toggles.

2.  **Zone Components**
    - **Hand:** Fan layout for cards.
    - **Battlefield:** Grouping by permanent type (Lands back, Creatures front) or free-form.
    - **Stack:** Vertical list of spells/abilities waiting to resolve.

3.  **Game State Synchronization**
    - Create `useGameStore`.
    - **Task:** Handle `GAME_UPDATE` payloads from WebSocket. This payload contains the entire visible state of the game.
    - **Task:** Diffing logic to animate changes (e.g., Card moves from Hand to Stack).

4.  **Phase & Turn Indicator**
    - Visual representation of WUBRG mana pool.
    - Interactive Phase bar (Untap, Upkeep, Draw, Main1, Combat, Main2, End).
    - Priority indicator (Whose turn is it?).

---

## Phase 6: Game Interaction & Logic

**Objective:** Handling user inputs, priority passing, and complex game actions.

1.  **Feedback Loop**
    - **Task:** Middleware `sendAction` function (Cast spell, Activate ability, Pass priority).

2.  **Dialogs & Choices**
    - Implement specific UI for XMage callbacks:
        - `askYesNo` (Simple Dialog).
        - `chooseMode` (List selection).
        - `choosePile` (Fact or Fiction).
        - `chooseColor`.
        - `pickTarget` (Mouse interaction with battlefield cards).

3.  **Card Interactions**
    - **Task:** Context Menu on cards (Activate ability, Attack, Block).
    - **Task:** Targeting arrows (SVG overlay drawing lines between source and target).
    - **Task:** Combat assignment UI (ordering blockers).

4.  **Mana Payment**
    - UI for manual mana tapping vs. auto-payment suggestions provided by the engine.

---

## Phase 7: Drafting & Limited

**Objective:** Interface for Draft and Sealed formats.

1.  **Draft UI**
    - **Layout:** Center pack view (clickable cards), Bottom picked cards view.
    - **Task:** Timer visualizer (Draft picks are timed).

2.  **Deck Construction (Limited)**
    - Reuse Deck Editor components but restricted to the "Pool" of drafted/opened cards.
    - "Submit Deck" action.

---

## Phase 8: Polish & Settings

**Objective:** Quality of life features.

1.  **User Preferences**
    - Create a settings modal.
    - **Options:**
        - Auto-yield (F2/F4 functionality).
        - Auto-pass priority settings.
        - Card size scaling.
        - Sound volume.

2.  **Asset Management**
    - **Task:** Logic to handle card images (using high-res sources or proxies).
    - **Task:** Preloading assets for smoother gameplay.

3.  **Game Log**
    - A dedicated scrollable area logging all game actions (Spell casting, damage dealing) with hover-to-preview card functionality.

4.  **Keyboard Shortcuts**
    - Map keys to actions (Space to Pass, F2 to Yield, Ctrl+Z to Undo mana).

## Directory Structure

```text
src/
├── api/            # WebSocket and REST adapters
├── assets/         # Static images/icons
├── components/
│   ├── ui/         # Shadcn generic components
│   ├── lobby/      # Lobby specific components
│   ├── editor/     # Deck editor specific components
│   ├── game/       # Battlefield components (Card, Zone, Hand)
│   └── shared/     # Shared domain components (ManaSymbol, CardImage)
├── hooks/          # Custom React hooks
├── lib/            # Utilities (formatting, validation)
├── stores/         # Zustand state stores
├── types/          # TypeScript interfaces (Game, Card, Action)
└── views/          # Page components (Login, Lobby, Game)
```
```
