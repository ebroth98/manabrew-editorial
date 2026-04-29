# Multiplayer Draft — implementation plan

Phase 1–4 shipped a single-player Limited mode (Sealed / Booster Draft / Winston / Gauntlet) where every non-human seat is an AI. This doc lays out what to add to support **multiple human seats in the same pod**, sharing one authoritative draft state through `forge-server`.

The recommended scope for the first pass is **Booster Draft only** — Sealed and Winston don't need pod synchronization (Sealed has no shared state at all; Winston is 2-player and worth doing as a follow-up).

## Current single-player architecture (recap)

```
React UI ──► useLimitedStore ──► getPlatform().invoke()
                                  ├─ Tauri:   limited_commands.rs ──► LimitedManager (HashMap of BoosterDraft)
                                  └─ Web:     game-engine.worker.ts ──► limited_api.rs (thread_local WasmLimitedState)
```

Both backends own their own `BoosterDraft` instance. The pod is hard-coded as `1 human (seat 0) + (pod_size − 1) AI`. Human picks come in via `submit_human_pick`; AI picks resolve inline during `tick()`.

## Target multiplayer architecture

```
React UI ──► useLimitedStore ──► getPlatform().invoke()
                                  ├─ "limited_join_pod"   ─┐
                                  ├─ "limited_pick_card"  ─┤
                                  ├─ "limited_get_state"  ─┘
                                  │
                                  └─ For pods with remote seats: relayed via
                                     forge-server WebSocket (host runs the draft)
```

Authoritative model: one **host** (any client in the pod) owns the `BoosterDraft` instance. Other humans connect via `forge-server` and submit picks through the existing room-relay protocol. Server broadcasts the new `DraftStateDto` to every seat after each pick is resolved.

## Engine changes — `forge-limited`

### `BoosterDraft` accepts multiple human seats

Today:

```rust
pub fn new(pod_size: usize, rounds: u32, ...) -> Self {
    let mut seats = vec![LimitedPlayer::new(0, "You", true, HumanLimitedAgent::new())];
    seats.extend(BoosterDraftAI::build_ai_seats(pod_size - 1, ...));
}
```

Change to:

```rust
pub struct PodSeatConfig {
    pub name: String,
    pub kind: SeatKind,
}
pub enum SeatKind { Human, Ai }

pub fn new(seats: Vec<PodSeatConfig>, rounds: u32, template, pool, ranker, color_of) -> Self
```

Each `Human` seat gets its own `HumanLimitedAgent`. The `submit_human_pick` API needs a `seat_index` parameter so the host knows which seat just picked:

```rust
pub fn submit_human_pick(&mut self, seat: usize, card: PaperCard) -> Result<(), String>
```

Update `BoosterDraft::tick()` to pause on **any** human seat that hasn't submitted yet, not just seat 0. Existing `downcast_human{,_ref}` helpers need to walk all seats and downcast generically — replace the unsafe pointer cast with a real `Any` downcast (`as_any` method on the `LimitedAgent` trait).

### Pack-pass direction stays the same

`PassDirection::Left` / `::Right` already alternates per round; works identically for any number of seats.

## Server changes — `forge-server`

### New session type

Today `forge-server` only knows about game rooms. Add a `DraftPod`:

```rust
struct DraftPod {
    pod_id: String,
    seats: Vec<DraftPodSeat>,        // one per slot
    draft: BoosterDraft,             // authoritative engine state
    host_user_id: UserId,
}

struct DraftPodSeat {
    seat_index: usize,
    occupant: SeatOccupant,          // Human(UserId) | Ai | Empty
    last_seen: Instant,              // for disconnect handling
}
```

Hosted on the `forge-server` process the pod was created on (no cross-server pod migration in the first pass).

### Wire protocol

Extend `ClientMessage` (`forge-server/src/protocol.rs`) with draft variants:

```rust
enum ClientMessage {
    // existing ...
    DraftCreatePod   { config: BoosterDraftSetupDto, seats: Vec<SeatKind> },
    DraftJoinPod     { pod_id: String, seat_index: Option<usize> },
    DraftLeavePod    { pod_id: String },
    DraftSubmitPick  { pod_id: String, card_name: String },
    DraftListPods,
}

enum ServerMessage {
    DraftPodCreated   { pod_id: String, state: DraftStateForSeatDto },
    DraftPodJoined    { pod_id: String, your_seat: usize },
    DraftStateUpdate  { pod_id: String, state: DraftStateForSeatDto },
    DraftPodClosed    { pod_id: String, reason: String },
}
```

`DraftStateForSeatDto` is `DraftStateDto` rendered from a specific seat's perspective — `currentPack` is the pack in front of _that_ seat, `pickedPile` is _that_ seat's pile. Other seats appear in `seatSummaries` with picks-made counts only.

### Server-side flow

1. Pod created via `DraftCreatePod` → server allocates a `DraftPod`, fills empty seats with AI per the config, broadcasts `DraftPodCreated` to the creator.
2. Other clients send `DraftJoinPod` (with optional preferred seat), server assigns the next empty Human seat, broadcasts state to everyone.
3. When all Human seats are joined → server calls `draft.start_round()`, sends `DraftStateUpdate` to each connected seat with their personalized view.
4. A client sends `DraftSubmitPick { card_name }` → server calls `draft.submit_human_pick(their_seat_index, card)`, then drains the AI tick loop, then broadcasts `DraftStateUpdate` to every seat.
5. When `draft.is_complete()` → final `DraftStateUpdate` with `isComplete: true`. Each seat now has its drafted pile; building the deck happens client-side with the existing `LimitedDeckBuilder`.

### Disconnects

If a Human seat hasn't pinged in N seconds (reuse the existing room presence timeout), demote that seat to AI for the rest of the draft so the pod doesn't stall. Broadcast a `DraftStateUpdate` with the new occupant kind.

## Tauri / WASM changes

### LimitedManager / WasmLimitedState

Two modes:

- **Local pod** (current behavior): own a `BoosterDraft` directly, route every command through it.
- **Remote pod**: hold a cached `DraftStateForSeatDto` updated via WebSocket events, forward `submit_pick` over the existing `IServerApi.broadcastState` / room-message channel.

Add a new field:

```rust
pub struct ActiveDraft {
    Local(BoosterDraft),
    Remote { pod_id: String, latest_state: DraftStateForSeatDto },
}
```

Pick command:

```rust
fn submit_human_pick(session_id, card_name):
    match self.drafts.get_mut(session_id):
        Local(d)  => d.submit_human_pick(0, card)  // local always seat 0
        Remote(_) => /* ServerClient sends DraftSubmitPick */
```

### Frontend

`useLimitedStore` grows a new action:

```ts
joinDraftPod: (podId: string) => Promise<DraftState>;
```

Wired to a new `IPlatformApi.invoke("limited_join_pod", ...)`. The lobby UI (a new `<DraftLobby>` view) lists active pods from `DraftListPods` and lets the user create one with a `BoosterDraftSetup`.

Existing `Draft.tsx` view stays mostly as-is — it already renders `DraftState`. The only addition: an indicator showing "your seat" in the pod summary (the seat label is already in `DraftSeatDto`).

## Lobby UI

A new `/limited/lobby` route that shows:

- "Create pod" form: pod size (2–8), set picker (reuses the existing one), # of human seats vs AI.
- Active public pods: pod id, host name, seats filled (e.g. `3/8`), set, "Join" button.
- Reuses the existing room list pattern from `Lobby.tsx` for layout.

When a user clicks Join → `joinDraftPod(podId)` → on success navigate to `/draft/{sessionId}`. The existing draft view picks up the cached state and renders the same UI.

## Open questions / future work

- **Sealed multiplayer**: not needed — Sealed has no shared state. Players can each open their own pool from the same edition and start a multiplayer game with their built decks via the existing flow.
- **Winston multiplayer**: 2 humans, 3 piles. Easier than Booster Draft because there's no pack-passing, just pile take/pass turns. Same architecture — server owns `WinstonDraft`, clients submit `take` / `pass` actions.
- **Draft chat** in the lobby + in-pod (reuse the existing room message protocol).
- **Replay / spectator mode**: stream `DraftStateUpdate` events to non-seat connections (read-only). Punt to a later phase.
- **Persistence**: in-memory pods only for v1. If a pod loses every Human seat, the pod is destroyed. Saving in-progress drafts to disk is a v2 concern.

## Suggested implementation order

1. `forge-limited`: refactor `BoosterDraft::new` to take a seat config; multi-human `submit_human_pick(seat, card)`; tests for 2-human, 3-human, 8-human pods.
2. `forge-server`: `DraftPod` session type + protocol messages + the create/join/pick/state flow. Unit tests at the protocol level.
3. Tauri + WASM: route the new `limited_join_pod` / remote-mode pick through the existing `ServerClient`. Reuse the local-mode `Draft.tsx` UI verbatim — only the data source changes.
4. New `<DraftLobby>` view + the create/list/join flow.
5. Disconnect → AI demotion.
6. Apply the same pattern to `WinstonDraft` once the Booster Draft path is proven.
