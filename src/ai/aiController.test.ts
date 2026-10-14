import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { AiRequest, AiTurn } from './aiWorker'
import type { GameState } from '../data/types'

const fake = vi.hoisted(() => {
  class FakeWorker {
    onmessage: ((event: { data: unknown }) => void) | null = null
    sent: AiRequest[] = []
    terminated = false

    constructor() {
      workers.push(this)
    }

    postMessage(request: AiRequest) {
      this.sent.push(request)
    }

    terminate() {
      this.terminated = true
    }

    reply(request: AiRequest, turn: AiTurn) {
      this.onmessage?.({ data: { type: 'success', id: request.id, ...turn } })
    }
  }
  const workers: FakeWorker[] = []
  return { FakeWorker, workers }
})

vi.mock('./aiWorker?worker', () => ({ default: fake.FakeWorker }))

const { AiController } = await import('./aiController')

const position = (turn: number) => ({ turn }) as GameState
const result = (turn: number): AiTurn => ({ choices: [], state: position(turn), events: [] })

describe('AiController', () => {
  beforeEach(() => {
    fake.workers.length = 0
  })

  it('takes a turn searched ahead without searching it again', async () => {
    const ai = new AiController()
    const worker = fake.workers[0]
    ai.searchAhead(position(1), 100)
    const turn = ai.takeTurn(position(1), 100)

    expect(worker.sent).toHaveLength(1)
    worker.reply(worker.sent[0], result(2))
    await expect(turn).resolves.toEqual(result(2))
  })

  it('searches afresh, on a fresh worker, when the board reached another position', async () => {
    const ai = new AiController()
    ai.searchAhead(position(1), 100)
    const turn = ai.takeTurn(position(7), 100)

    const [first, second] = fake.workers
    expect(first.terminated).toBe(true)
    expect(second.sent.map((request) => request.state)).toEqual([position(7)])
    second.reply(second.sent[0], result(8))
    await expect(turn).resolves.toEqual(result(8))
  })

  it('does not reuse a search made at another strength', () => {
    const ai = new AiController()
    ai.searchAhead(position(1), 100)
    ai.takeTurn(position(1), 200)

    expect(fake.workers.at(-1)?.sent.map((request) => request.iterations)).toEqual([200])
  })

  it('replaces a busy worker on cancel, so the next search need not wait', () => {
    const ai = new AiController()
    ai.searchAhead(position(1), 100)
    ai.cancel()
    ai.takeTurn(position(1), 100)

    expect(fake.workers).toHaveLength(2)
    expect(fake.workers[0].terminated).toBe(true)
    expect(fake.workers[1].sent).toHaveLength(1)
  })

  it('keeps an idle worker on cancel', () => {
    const ai = new AiController()
    ai.cancel()
    expect(fake.workers).toHaveLength(1)
    expect(fake.workers[0].terminated).toBe(false)
  })
})
