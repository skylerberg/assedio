import AiWorkerModule from './aiWorker?worker'
import type { AiRequest, AiResponse, AiTurn } from './aiWorker'
import type { GameState } from '../data/types'

export interface Difficulty {
  name: string
  iterations: number
  blurb: string
}

/// Iterations are per decision, and a turn is a handful of decisions.
export const DIFFICULTIES: Difficulty[] = [
  { name: 'Squire', iterations: 4_000, blurb: 'Sees the next exchange, little more.' },
  { name: 'Knight', iterations: 30_000, blurb: 'Plays the siege race honestly.' },
  { name: 'Marshal', iterations: 120_000, blurb: 'Punishes a loose siege engine.' },
  { name: 'Warlord', iterations: 500_000, blurb: 'Takes a moment, and takes the game.' },
]

/// One worker for the whole session. Each request supersedes the last, so a
/// reply whose id has gone stale — the player started a new game mid-search —
/// is dropped rather than applied to a position it was never about.
export class AiController {
  private worker: Worker
  private nextId = 1
  private pending: {
    id: number
    resolve: (turn: AiTurn) => void
    reject: (error: Error) => void
  } | null = null
  /** A search begun before the board reached its position. */
  private ahead: { key: string; turn: Promise<AiTurn> } | null = null

  constructor() {
    this.worker = this.spawn()
  }

  private spawn(): Worker {
    const worker = new AiWorkerModule()
    worker.onmessage = (event: MessageEvent<AiResponse>) => {
      const waiting = this.pending
      if (!waiting || waiting.id !== event.data.id) return
      this.pending = null
      if (event.data.type === 'error') {
        waiting.reject(new Error(event.data.message))
      } else {
        const { choices, state, events } = event.data
        waiting.resolve({ choices, state, events })
      }
    }
    return worker
  }

  /** The machine's turn from this position: the search begun ahead for it if
      there is one, otherwise a fresh one. */
  takeTurn(state: GameState, iterations: number): Promise<AiTurn> {
    const ahead = this.ahead
    if (ahead && ahead.key === key(state, iterations)) {
      this.ahead = null
      return ahead.turn
    }
    if (ahead) this.cancel()
    return this.search(state, iterations)
  }

  /** Start searching a position the board has yet to reach. When two machines
      play, the next one's position is known the moment the last one's search
      returns, so it can think while that turn is still being shown. */
  searchAhead(state: GameState, iterations: number): void {
    const turn = this.search(state, iterations)
    // A failure is the turn's to report when it is taken, not before.
    turn.catch(() => {})
    this.ahead = { key: key(state, iterations), turn }
  }

  private search(state: GameState, iterations: number): Promise<AiTurn> {
    const id = this.nextId++
    return new Promise((resolve, reject) => {
      this.pending = { id, resolve, reject }
      const request: AiRequest = { id, state, iterations }
      this.worker.postMessage(JSON.parse(JSON.stringify(request)))
    })
  }

  /** Abandon any search in flight, and any begun ahead. The worker cannot be
      interrupted mid-search, and the next request would queue behind it, so a
      busy one is replaced. */
  cancel(): void {
    if (this.pending) {
      this.worker.terminate()
      this.worker = this.spawn()
    }
    this.pending = null
    this.ahead = null
  }

  dispose(): void {
    this.worker.terminate()
  }
}

function key(state: GameState, iterations: number): string {
  return `${iterations}:${JSON.stringify(state)}`
}
