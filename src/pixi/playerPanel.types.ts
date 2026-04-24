/**
 * Shared interface for player panel variants (column layout, square layout, etc.)
 * Player panel variants (e.g. PlayerSquarePanel) implement this.
 */

import type { Container } from "pixi.js";
import type { Theme } from "@/hooks/useTheme";

export interface PlayerPanelState {
  playerName: string;
  playerId: string;
  life: number;
  handCount: number;
  poison: number;
  energyCounters: number;
  commanderDamage: number;
  manaPool: Record<string, number>;
  libraryCount: number;
  graveyardCount: number;
  exileCount: number;
  commandZoneCount: number;
  currentStep: string;
  isActiveTurn: boolean;
  isPriorityPlayer: boolean;
  isTargetable: boolean;
  hasPlayableInGraveyard: boolean;
  hasPlayableInExile: boolean;
  enabledPhases: Set<string>;
  isInteractive: boolean;
  /** Which player seat colour to use: "self" | "opponent1" | "opponent2" | "opponent3" */
  playerSeat: "self" | "opponent1" | "opponent2" | "opponent3";
}

export interface PlayerPanelCallbacks {
  onOpenGraveyard?: () => void;
  onOpenExile?: () => void;
  onOpenCommandZone?: () => void;
  onTargetPlayer?: () => void;
  onTogglePhase?: (phaseId: string) => void;
}

export interface PlayerPanel {
  readonly container: Container;
  setTheme(theme: Theme): void;
  setCallbacks(cb: PlayerPanelCallbacks): void;
  setPosition(x: number, y: number): void;
  setHeight(h: number): void;
  update(state: PlayerPanelState): void;
  tick(): void;
  destroy(): void;
}
