// The game in progress, kept in this browser so that a reload or a trip to the
// menu does not throw it away.

import { BOARD_COLS, BOARD_ROWS } from './types'
import type { GameState } from './types'
import type { PieceIds, Turn } from './turns'

export interface GameSetup {
  seats: ('human' | 'ai')[]
  iterations: number
  /** The machine's strength, by name; meaningless when no seat is a machine. */
  difficulty: string
}

export interface SavedGame {
  setup: GameSetup
  game: GameState
  turns: Turn[]
  ids: PieceIds
}

const KEY = 'assedio:game'
/** Bumped whenever the shape changes, so an old save is dropped, not misread. */
const VERSION = 2

/** Browser storage can be full, disabled or private; a save is a convenience,
    so failing to make one must never interrupt the game. */
function storage(): Storage | null {
  try {
    return globalThis.localStorage ?? null
  } catch {
    return null
  }
}

export function saveGame(saved: SavedGame): void {
  try {
    storage()?.setItem(KEY, JSON.stringify({ version: VERSION, ...saved }))
  } catch {
    // Out of quota or blocked: the game goes on unsaved.
  }
}

export function loadGame(): SavedGame | null {
  try {
    const raw = storage()?.getItem(KEY)
    if (!raw) return null
    return parseSave(JSON.parse(raw))
  } catch {
    return null
  }
}

export function clearGame(): void {
  try {
    storage()?.removeItem(KEY)
  } catch {
    // Nothing to clear if storage is unavailable.
  }
}

export function parseSave(value: unknown): SavedGame | null {
  if (typeof value !== 'object' || value === null) return null
  const save = value as Record<string, unknown>
  if (save.version !== VERSION) return null
  const game = save.game as GameState | undefined
  const setup = save.setup as GameSetup | undefined
  if (
    !game ||
    !Array.isArray(game.board) ||
    game.board.length !== BOARD_ROWS ||
    !setup ||
    !Array.isArray(setup.seats) ||
    !Array.isArray(save.turns) ||
    !Array.isArray(save.ids) ||
    save.ids.length !== BOARD_ROWS * BOARD_COLS
  ) {
    return null
  }
  return { setup, game, turns: save.turns as Turn[], ids: save.ids as PieceIds }
}
