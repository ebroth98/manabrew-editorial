import { useEffect } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useGameStore } from '@/stores/useGameStore';
import { normalizeGameLogPayload } from '@/types/gameLog';
import { normalizeSnapshotPayload } from '@/types/gameSnapshot';
import { applyPrompt } from '@/stores/gameStore.constants';
import type { AgentPrompt } from '@/stores/gameStore.types';

/**
 * Hook that sets up Tauri event listeners for game state updates.
 * Automatically cleans up on unmount.
 */
export function useGameEventListeners() {
  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];

    (async () => {
      try {
        const unlisten1 = await listen<AgentPrompt>('game:prompt', (event) => {
          const prompt = event.payload;
          const gameView = useGameStore.getState().gameView;
          if (gameView?.gameOver) return;
          if (prompt && prompt.gameView) {
            applyPrompt(prompt, 'Event', useGameStore.setState, useGameStore.getState);
          }
        });
        unlisteners.push(unlisten1);

        const unlisten2 = await listen<unknown>('game:log', (event) => {
          const entry = normalizeGameLogPayload(event.payload);
          useGameStore.setState((state) => ({
            gameLog: [...state.gameLog.slice(-199), entry],
          }));
        });
        unlisteners.push(unlisten2);

        const unlistenSnapshot = await listen<unknown>('game:snapshot', (event) => {
          const snapshot = normalizeSnapshotPayload(event.payload);
          if (!snapshot.gameView) return;
          useGameStore.setState((state) => ({
            snapshots: [...state.snapshots.filter((s) => s.checkpointId !== snapshot.checkpointId).slice(-199), snapshot],
          }));
        });
        unlisteners.push(unlistenSnapshot);

        // Remote prompt listener: receives prompts relayed via the server for non-host players
        const unlisten3 = await listen<{ kind: string; forPlayer: string; prompt: AgentPrompt }>('game:remote_prompt', (event) => {
          const { forPlayer, prompt } = event.payload;
          const { myPlayerSlot } = useGameStore.getState();
          if (forPlayer === myPlayerSlot) {
            // This prompt is for us — render it fully.
            applyPrompt(prompt, 'Remote', useGameStore.setState, useGameStore.getState);
          } else {
            // Keep shared turn/priority in sync even when the prompt is for another player.
            // Do not apply full foreign-perspective view (would leak/flip local actionability).
            const current = useGameStore.getState().gameView;
            if (current && prompt?.gameView) {
              const iHavePriority = prompt.gameView.priorityPlayerId === myPlayerSlot;
              useGameStore.setState({
                gameView: {
                  ...current,
                  turn: prompt.gameView.turn,
                  step: prompt.gameView.step,
                  activePlayerId: prompt.gameView.activePlayerId,
                  priorityPlayerId: prompt.gameView.priorityPlayerId,
                  gameOver: prompt.gameView.gameOver,
                  winnerId: prompt.gameView.winnerId,
                },
                // Never keep a stale actionable prompt when priority is not ours.
                currentPrompt: iHavePriority ? useGameStore.getState().currentPrompt : null,
                isWaitingForResponse: iHavePriority ? useGameStore.getState().isWaitingForResponse : false,
                debugInfo: `Remote sync: ${prompt.type}`,
              });
            }
          }
        });
        unlisteners.push(unlisten3);

        const unlisten4 = await listen<{ reason: string; message: string }>('game:forced_end', (event) => {
          const message = event.payload?.message ?? 'Forced game exit';
          useGameStore.setState({
            isGameActive: false,
            gameView: null,
            currentPrompt: null,
            deferredQueue: [],
            isFlashing: false,
            isWaitingForResponse: false,
            isMultiplayer: false,
            isHost: false,
            myPlayerSlot: null,
            snapshots: [],
            debugInfo: `Game ended: ${message}`,
          });
        });
        unlisteners.push(unlisten4);
      } catch (e) {
        console.error('[hook] Failed to setup listeners:', e);
      }
    })();

    return () => {
      unlisteners.forEach((fn) => fn());
    };
  }, []);
}
