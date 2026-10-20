<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import Board from './Board.svelte'
  import EventLog from './EventLog.svelte'
  import Legend from './Legend.svelte'
  import type { LegendEntry } from './Legend.svelte'
  import Modal from './Modal.svelte'
  import Ornament from './Ornament.svelte'
  import PlayerPanel from './PlayerPanel.svelte'
  import RulesOverlay from './RulesOverlay.svelte'
  import TokenIcon from './TokenIcon.svelte'
  import { AiController } from '../ai/aiController'
  import {
    applyChoice,
    attackPreview,
    forcedOrder,
    initialState,
    legalChoices,
    moveOutcomes,
    pendingAttackers,
    slideOutcomes,
  } from '../engine/wasmEngine'
  import { destroyedSquares, lineName, squareName } from '../data/log'
  import { clearGame, saveGame } from '../data/session'
  import type { GameSetup, SavedGame } from '../data/session'
  import {
    advanceIds,
    idsAfter,
    initialIds,
    recordTurns,
    summarize,
    turnMarks,
  } from '../data/turns'
  import type { PieceIds, Turn } from '../data/turns'
  import {
    PIECE_NAMES,
    PLAYER_NAMES,
    TOKEN_NAMES,
    attackingToken,
    pendingTokens,
    pieceAt,
    tokenOf,
  } from '../data/types'
  import type {
    AttackPreview,
    Choice,
    GameState,
    MoveOutcome,
    SlideOutcome,
    Square,
    TokenKind,
  } from '../data/types'

  interface Props {
    setup: GameSetup
    /** A game to pick up where it was left, rather than a new one. */
    saved?: SavedGame | null
    onexit: () => void
  }

  let { setup, saved = null, onexit }: Props = $props()

  type Speed = 'slow' | 'normal' | 'fast'
  const SPEEDS: Speed[] = ['slow', 'normal', 'fast']
  /** Pauses between the machine's decisions within a turn, and between whole
      turns when two machines play, so a turn can be followed. */
  const PACE: Record<Speed, { step: number; gap: number }> = {
    slow: { step: 1000, gap: 1500 },
    normal: { step: 600, gap: 700 },
    fast: { step: 150, gap: 150 },
  }
  const FLASH_MS = 800

  interface Snapshot {
    game: GameState
    turns: Turn[]
    ids: PieceIds
  }

  const start = initialState()
  // A saved game is the starting point by design, not a value to keep in sync.
  // svelte-ignore state_referenced_locally
  const resumed = saved

  // Every position, turn record and id list is replaced, never edited, so they
  // are held raw: an undo snapshot can share them rather than copy them.
  let game: GameState = $state.raw(resumed?.game ?? start)
  let turns: Turn[] = $state.raw(resumed?.turns ?? [])
  let ids: PieceIds = $state.raw(resumed?.ids ?? initialIds(start))
  let history: Snapshot[] = $state.raw([])
  let epoch = $state(0)
  let destroyed: Square[] = $state.raw([])
  /** True from the moment a search starts until the machine's turn is on the
      board. */
  let thinking = $state(false)
  let failure: string | null = $state(null)
  let showRules = $state(false)
  let showOptions = $state(false)
  let confirmRestart = $state(false)
  let showThreats = $state(readPref('threats'))
  let showOutcomes = $state(readPref('outcomes'))
  /** A sentence about whatever is under the cursor on the board. */
  let hint: string | null = $state(null)
  let highlight: Square[] = $state.raw([])
  let legend: LegendEntry[] = $state.raw([])
  /** How many turns the review is showing, or null outside a review. */
  let review: number | null = $state(null)
  let paused = $state(false)
  let speed: Speed = $state('normal')
  let cardDismissed = $state(false)

  const ai = new AiController()
  /** Bumped whenever the position the machine is searching stops being
      current. */
  let generation = 0
  let stepOnce = false
  let flashTimer: ReturnType<typeof setTimeout> | null = null

  function readPref(name: string): boolean {
    try {
      return localStorage.getItem(`assedio:${name}`) !== 'off'
    } catch {
      return true
    }
  }

  function writePref(name: string, on: boolean) {
    try {
      localStorage.setItem(`assedio:${name}`, on ? 'on' : 'off')
    } catch {
      // A preference that does not stick is no reason to stop.
    }
  }

  let humanSeats = $derived(
    setup.seats.map((seat, index) => (seat === 'human' ? index : -1)).filter((i) => i >= 0),
  )
  let mode = $derived(
    humanSeats.length === 1 ? 'solo' : humanSeats.length === 2 ? 'hotseat' : 'watch',
  )
  let isHumanTurn = $derived(
    review === null &&
      game.outcome === null &&
      setup.seats[game.currentPlayer] === 'human' &&
      !thinking,
  )

  // Whose controls are live. One human seat keeps them that side's even while
  // the opponent thinks; hot-seat and watched games follow whoever is to move.
  let viewer = $derived(mode === 'solo' ? humanSeats[0] : game.currentPlayer)

  /** The side drawn nearest: yours against the machine, otherwise the rules
      sheet's view with Ivory along the bottom. Fixed for the whole game. */
  let orientation = $derived(mode === 'solo' ? humanSeats[0] : 0)

  /** While the activation order is open, the two ways the turn could go. The
      board offers the moves of one and the first target of the other, and
      whichever the player takes first decides the order. */
  let orderPlan = $derived.by(() => {
    if (!isHumanTurn || game.phase !== 'order') return null
    const armed = attackingToken(game, game.currentPlayer)
    if (!armed) return null
    const mover: TokenKind = armed === 'march' ? 'flank' : 'march'
    const position = game
    return {
      armed,
      mover,
      moveFirst: applyChoice(position, { type: 'order', first: mover }).state,
      attackFirst: applyChoice(position, { type: 'order', first: armed }).state,
    }
  })

  let choices: Choice[] = $derived.by(() => {
    if (!isHumanTurn) return []
    if (!orderPlan) return legalChoices(game)
    return [
      ...legalChoices(orderPlan.moveFirst),
      ...legalChoices(orderPlan.attackFirst).filter(
        (choice) => choice.type === 'attack' || choice.type === 'holdFire',
      ),
    ]
  })
  let moveState = $derived(orderPlan?.moveFirst ?? game)
  let mustPass = $derived(
    game.phase === 'activate' && choices.length === 1 && choices[0].type === 'pass',
  )

  /** The game on the board: the live one, or a reviewed turn. */
  let shown: GameState = $derived(
    review === null ? game : review === 0 ? start : (turns[review - 1].after ?? game),
  )
  let shownIds = $derived(review === null ? ids : idsAfter(start, turns, review))

  let attacks = $derived.by(() => {
    const result: Record<number, AttackPreview | null> = { 0: null, 1: null }
    if (review !== null || shown.outcome) return result
    for (const player of [0, 1]) {
      const kind = attackingToken(shown, player)
      result[player] = kind ? attackPreview(shown, player, kind) : null
    }
    return result
  })

  let outcomes: MoveOutcome[] = $derived(
    isHumanTurn && (game.phase === 'activate' || orderPlan)
      ? moveOutcomes(moveState)
      : [],
  )
  let slides: SlideOutcome[] = $derived(
    isHumanTurn && game.phase === 'slide' ? slideOutcomes(game) : [],
  )
  let attackers: Square[] = $derived.by(() => {
    if (orderPlan) return pendingAttackers(orderPlan.attackFirst)
    return game.phase === 'attack' && review === null
      ? pendingAttackers(game)
      : []
  })
  let moveToken: TokenKind | null = $derived(
    orderPlan
      ? orderPlan.mover
      : game.phase === 'activate'
        ? (pendingTokens(game)[0] ?? null)
        : null,
  )

  /** The turn whose marks stay on the board: the one before a person's own
      while they decide, and otherwise the latest, played out as it happens. */
  let marks = $derived.by(() => {
    if (review !== null) return review > 0 ? turnMarks(turns[review - 1]) : null
    const last = turns.at(-1)
    if (!last) return null
    if (isHumanTurn && !last.after) {
      const before = turns.at(-2)
      return before ? turnMarks(before) : null
    }
    return turnMarks(last)
  })

  let summary = $derived(summarize(turns))

  function snapshot(): Snapshot {
    return { game, turns, ids }
  }

  // The game is saved as it goes, so a reload or a trip to the menu loses
  // nothing.
  $effect(() => {
    saveGame({ setup, ...snapshot() })
  })

  $effect(() => writePref('threats', showThreats))
  $effect(() => writePref('outcomes', showOutcomes))

  function apply(choice: Choice) {
    hint = null
    const round = game.turn
    const result = applyChoice(game, choice)
    game = result.state
    turns = recordTurns(turns, result.events, round, result.state)
    ids = advanceIds(ids, result.events)
    const struck = destroyedSquares(result.events)
    if (struck.length > 0) {
      destroyed = struck
      if (flashTimer) clearTimeout(flashTimer)
      flashTimer = setTimeout(() => (destroyed = []), FLASH_MS)
    }
  }

  function handleChoice(choice: Choice) {
    if (!isHumanTurn) return
    history = [...history, snapshot()]
    if (orderPlan) {
      const attackFirst = choice.type === 'attack' || choice.type === 'holdFire'
      apply({ type: 'order', first: attackFirst ? orderPlan.armed : orderPlan.mover })
    }
    apply(choice)
    advance()
  }

  function attackFirst() {
    if (!orderPlan) return
    history = [...history, snapshot()]
    apply({ type: 'order', first: orderPlan.armed })
    advance()
  }

  function choose(type: 'pass' | 'holdFire') {
    const choice = choices.find((option) => option.type === type)
    if (choice) handleChoice(choice)
  }

  /** Play out every decision that has only one possible outcome, so the player
      is only ever asked something they could answer more than one way. Nothing
      taken here reaches the undo stack: undo steps back to a real decision. */
  function autoResolve() {
    // A turn is a handful of decisions and a hot-seat game hands straight over.
    // The bound is only here so that a bug cannot freeze the tab.
    for (let step = 0; step < 64; step++) {
      if (game.outcome || thinking) return
      if (setup.seats[game.currentPlayer] !== 'human') return

      const position = game
      const options = legalChoices(position)
      if (options.length === 1) {
        apply(options[0])
        continue
      }
      if (position.phase !== 'order') return
      // The engine settles the order whenever one of them gives up nothing;
      // only a fork the player could answer either way reaches them.
      const first = forcedOrder(position)
      if (!first) return
      apply({ type: 'order', first })
    }
  }

  function advance() {
    autoResolve()
    driveAi()
  }

  function driveAi() {
    if (game.outcome || thinking || review !== null) return
    if (setup.seats[game.currentPlayer] !== 'ai') return
    if (paused && !stepOnce) return
    stepOnce = false
    void runAi()
  }

  const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))

  async function runAi() {
    thinking = true
    const gen = generation
    try {
      const turn = await ai.takeTurn(game, setup.iterations)
      if (gen !== generation) return
      if (!turn.state.outcome && setup.seats[turn.state.currentPlayer] === 'ai') {
        ai.searchAhead(turn.state, setup.iterations)
      }
      for (const [index, choice] of turn.choices.entries()) {
        if (gen !== generation) return
        apply(choice)
        if (index < turn.choices.length - 1) await sleep(PACE[speed].step)
      }
    } catch (error) {
      if (gen === generation) failure = String(error)
    } finally {
      if (gen === generation) thinking = false
    }
    if (gen !== generation) return
    // Two machines playing each other leave a beat between turns.
    if (mode === 'watch') await sleep(PACE[speed].gap)
    if (gen === generation) advance()
  }

  /** Abandon whatever the machine is doing and put a position on the board. */
  function reset(next: Snapshot) {
    generation += 1
    ai.cancel()
    thinking = false
    destroyed = []
    hint = null
    highlight = []
    game = next.game
    turns = next.turns
    ids = next.ids
    epoch += 1
  }

  function undo() {
    const previous = history.at(-1)
    if (!previous) return
    history = history.slice(0, -1)
    reset(previous)
  }

  function restart() {
    confirmRestart = false
    showOptions = false
    history = []
    failure = null
    review = null
    cardDismissed = false
    reset({ game: start, turns: [], ids: initialIds(start) })
    advance()
  }

  function exit() {
    generation += 1
    ai.cancel()
    if (game.outcome) clearGame()
    onexit()
  }

  function togglePause() {
    paused = !paused
    if (!paused) advance()
  }

  function step() {
    stepOnce = true
    driveAi()
  }

  function startReview(count: number) {
    review = Math.max(0, Math.min(turns.length, count))
    epoch += 1
  }

  function moveReview(delta: number) {
    if (review === null) return
    review = Math.max(0, Math.min(turns.length, review + delta))
  }

  function onKey(event: KeyboardEvent) {
    if (event.key === 'Escape') showOptions = false
    if (review === null || showRules) return
    if (event.key === 'ArrowLeft') moveReview(-1)
    if (event.key === 'ArrowRight') moveReview(1)
  }

  function attackOwner(player: number): string {
    return mode === 'solo' && player === humanSeats[0] ? 'Your' : `${PLAYER_NAMES[player]}'s`
  }

  function seat(player: number): string {
    if (setup.seats[player] === 'ai') return `Machine · ${setup.difficulty}`
    return mode === 'solo' ? 'You' : 'Player'
  }

  /** Which of the sheet's three steps the turn is on. */
  let currentStep = $derived.by(() => {
    if (game.phase === 'slide') return 1
    if (game.phase === 'order') return 2
    return game.pendingLen >= 2 ? 2 : 3
  })

  let stepTokens = $derived.by((): (TokenKind | null)[] => {
    const sliding = slides[0]?.token ?? null
    if (game.phase === 'slide' || game.phase === 'order') return [sliding, null, null]
    const pending = pendingTokens(game)
    if (pending.length === 2) return [null, pending[0], pending[1]]
    const done: TokenKind | null = pending[0] ? (pending[0] === 'march' ? 'flank' : 'march') : null
    return [null, done, pending[0] ?? null]
  })

  let banner = $derived.by(() => {
    if (game.outcome) {
      return game.outcome.type === 'draw' ? 'A draw' : `${PLAYER_NAMES[game.outcome.player]} wins`
    }
    const mover = PLAYER_NAMES[game.currentPlayer]
    if (mode === 'solo') {
      return setup.seats[game.currentPlayer] === 'human' ? 'Your turn' : `${mover} is thinking…`
    }
    if (mode === 'watch') return paused && !thinking ? `Paused · ${mover} to move` : `${mover} to move`
    return `${mover} to move`
  })

  let prompt = $derived.by(() => {
    if (game.outcome) return ''
    if (!isHumanTurn) {
      if (mode === 'watch' && paused && !thinking) return 'Paused. Step through a turn, or resume.'
      return `${PLAYER_NAMES[game.currentPlayer]} is deliberating…`
    }
    if (game.phase === 'slide') {
      const kind = slides[0]?.token
      if (!kind) return ''
      return `Slide your ${TOKEN_NAMES[kind]} token to a new ${kind === 'march' ? 'row' : 'column'}.`
    }
    if (orderPlan) {
      const from = lineName(orderPlan.mover, tokenOf(game, game.currentPlayer, orderPlan.mover).line)
      const along = lineName(orderPlan.armed, tokenOf(game, game.currentPlayer, orderPlan.armed).line)
      return `${TOKEN_NAMES[orderPlan.mover]} a piece from ${from}, or Attack along ${along} first.`
    }
    const kind = pendingTokens(game)[0]
    if (!kind) return ''
    const where = lineName(kind, tokenOf(game, game.currentPlayer, kind).line)
    if (game.phase === 'attack') {
      const from = choices.find((c) => c.type === 'holdFire')
      const piece = from && from.type === 'holdFire' ? pieceAt(game, from.from) : null
      const at = from && from.type === 'holdFire' ? squareName(from.from) : ''
      return `Attack along ${where}: your ${piece ? PIECE_NAMES[piece.kind] : 'attacker'} on ${at} takes one target.`
    }
    if (mustPass) return `No piece on ${where} can move.`
    return kind === 'march'
      ? `March: move a piece from ${where} vertically or horizontally.`
      : `Flank: move a piece from ${where} diagonally.`
  })

  /** The standing sentence under the prompt: what this decision is really
      deciding. The board replaces it while something is under the cursor. */
  let guidance = $derived.by(() => {
    if (game.outcome || !isHumanTurn) return ''
    if (game.phase === 'slide') {
      return 'The line you choose is where this token attacks from next turn.'
    }
    if (orderPlan) {
      return 'Whichever you start first goes first. An Attack counts only the pieces on its line when it is made.'
    }
    if (game.phase === 'attack') {
      const left = attackers.length - 1
      return left > 0
        ? `${left} more ${left === 1 ? 'attacker' : 'attackers'} on this line after this one.`
        : 'The last attacker on this line.'
    }
    if (mustPass) return 'The activation passes.'
    const armed = attackingToken(game, game.currentPlayer)
    if (!armed) return 'Your Attack is done; this move ends the turn.'
    const where = lineName(armed, tokenOf(game, game.currentPlayer, armed).line)
    return `Your Attack along ${where} follows the moment the piece lands.`
  })

  let verdict = $derived.by(() => {
    if (!game.outcome) return null
    if (game.outcome.type === 'draw') return 'Neither side could force a decision.'
    const loser = 1 - game.outcome.player
    return game.castles[loser].every((standing) => !standing)
      ? `${PLAYER_NAMES[loser]} has lost every castle.`
      : `${PLAYER_NAMES[loser]} has lost both siege engines.`
  })

  let endTitle = $derived.by(() => {
    if (!game.outcome) return ''
    if (game.outcome.type === 'draw') return 'A Draw'
    if (mode === 'solo') return game.outcome.player === humanSeats[0] ? 'Victory' : 'Defeat'
    return `${PLAYER_NAMES[game.outcome.player]} Wins`
  })

  // Only the opening move needs a kick; every turn after is driven from
  // `handleChoice` or from `runAi` itself.
  onMount(advance)
  onDestroy(() => {
    generation += 1
    if (flashTimer) clearTimeout(flashTimer)
    ai.dispose()
  })
</script>

<svelte:window onkeydown={onKey} />

<div class="screen">
  <header class="bar">
    <div class="brand">
      <span class="wide"><Ornament side="left" /></span>
      <h1>Assedio</h1>
      <span class="wide"><Ornament side="right" /></span>
    </div>
    <p class="banner" class:over={game.outcome !== null} aria-live="polite">
      <span class={`disc side-${game.outcome?.type === 'winner' ? game.outcome.player : game.currentPlayer}`}
      ></span>
      {banner}
      {#if mode !== 'hotseat'}<span class="turn">· Turn {game.turn + 1}</span>{/if}
    </p>
    <div class="controls">
      {#if mode !== 'watch'}
        <button
          onclick={undo}
          disabled={history.length === 0 || review !== null}
          aria-label="Undo"
          title="Undo"
        >
          <span class="full">Undo</span><svg class="short" viewBox="0 0 20 20" aria-hidden="true"
            ><path d="M7 4 3 8l4 4M3.5 8H12a5 5 0 0 1 0 10H8" /></svg
          >
        </button>
      {/if}
      <button onclick={() => (showRules = true)} aria-label="Rules" title="Rules">
        <span class="full">Rules</span><span class="short" aria-hidden="true">?</span>
      </button>
      <div class="options">
        <button
          aria-expanded={showOptions}
          aria-haspopup="true"
          aria-label="Options"
          title="Options"
          onclick={() => (showOptions = !showOptions)}
        >
          <span class="full">Options</span><span class="short" aria-hidden="true">☰</span>
        </button>
        {#if showOptions}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div class="catcher" onclick={() => (showOptions = false)}></div>
          <div class="panel popover" role="menu" tabindex="-1">
            <label class="switch"><input type="checkbox" bind:checked={showThreats} /> Show threats</label>
            <label class="switch"><input type="checkbox" bind:checked={showOutcomes} /> Look ahead</label>
            <hr />
            <button onclick={() => ((confirmRestart = true), (showOptions = false))}>Restart…</button>
            <button onclick={exit}>Main menu</button>
          </div>
        {/if}
      </div>
    </div>
  </header>

  <div class="body">
    <main class="board-area">
      <Board
        game={shown}
        {choices}
        {moveState}
        {attacks}
        moveOutcomes={outcomes}
        slideOutcomes={slides}
        pendingAttackers={attackers}
        {moveToken}
        {viewer}
        {orientation}
        {showThreats}
        {showOutcomes}
        ids={shownIds}
        {marks}
        {highlight}
        {destroyed}
        {epoch}
        {attackOwner}
        onChoice={handleChoice}
        onhint={(text) => (hint = text)}
        onlegend={(entries) => (legend = entries)}
      />

      {#if game.outcome && review === null && !cardDismissed}
        <div class="end-card panel" role="dialog" aria-label="Game over">
          <h2 class="head">{endTitle}</h2>
          <div class="end-body">
            <p class="verdict">{verdict}</p>
            <dl class="stats">
              <dt>Turns</dt>
              <dd>{summary.rounds}</dd>
              {#each [0, 1] as player (player)}
                <dt>{PLAYER_NAMES[player]}</dt>
                <dd>
                  {summary.castles[player]} castle{summary.castles[player] === 1 ? '' : 's'} destroyed,
                  {summary.pieces[player]} piece{summary.pieces[player] === 1 ? '' : 's'} killed
                </dd>
              {/each}
            </dl>
            <div class="actions">
              <button class="primary" onclick={restart}>Play again</button>
              <button onclick={() => startReview(turns.length)}>Review the game</button>
              <button onclick={exit}>Menu</button>
            </div>
            <button class="link" onclick={() => (cardDismissed = true)}>Look at the board</button>
          </div>
        </div>
      {/if}
    </main>

    <section class="prompt panel">
      {#if review !== null}
        <p class="line">
          Reviewing {review === 0 ? 'the opening position' : `turn ${turns[review - 1].round + 1}, ${PLAYER_NAMES[turns[review - 1].player]}`}
        </p>
        <div class="actions">
          <button onclick={() => (review = 0)} disabled={review === 0} aria-label="First turn">⏮</button>
          <button onclick={() => moveReview(-1)} disabled={review === 0}>◀ Back</button>
          <span class="count" aria-label={`Turn ${review} of ${turns.length}`}>{review} / {turns.length}</span>
          <button onclick={() => moveReview(1)} disabled={review === turns.length}>Next ▶</button>
          <button onclick={() => (review = turns.length)} disabled={review === turns.length} aria-label="Last turn"
            >⏭</button
          >
          <button class="primary" onclick={() => ((review = null), (epoch += 1))}>Done</button>
        </div>
      {:else if game.outcome}
        <p class="line"><strong>{banner}.</strong> {verdict}</p>
        {#if cardDismissed}
          <div class="actions">
            <button class="primary" onclick={restart}>Play again</button>
            <button onclick={() => startReview(turns.length)}>Review the game</button>
            <button onclick={exit}>Menu</button>
          </div>
        {/if}
      {:else}
        {#if isHumanTurn}
          <ol class="steps" aria-label="Your turn">
            {#each ['Slide', 'Activate one', 'Activate the other'] as label, index (label)}
              {@const n = index + 1}
              {@const kind = stepTokens[index]}
              <li class:now={currentStep === n} class:done={currentStep > n}>
                <span class="diamond"><span>{n}</span></span>
                {label}
                {#if kind}
                  <span class="step-token">
                    <TokenIcon
                      {kind}
                      side={tokenOf(game, game.currentPlayer, kind).side}
                      owner={game.currentPlayer}
                    />
                  </span>
                {/if}
              </li>
            {/each}
          </ol>
        {/if}
        <p class="line" aria-live="polite">{prompt}</p>
        <p class="guidance" class:live={hint !== null}>{hint ?? guidance}</p>
        <div class="actions">
          {#if isHumanTurn && orderPlan}
            <button onclick={attackFirst}>Attack first</button>
            <button onclick={() => choose('pass')}>Take no action</button>
          {:else if isHumanTurn && game.phase === 'attack'}
            <button onclick={() => choose('holdFire')}>Hold fire</button>
          {:else if isHumanTurn && game.phase === 'activate'}
            <button class:primary={mustPass} onclick={() => choose('pass')}>
              {mustPass ? 'Pass' : 'Take no action'}
            </button>
          {:else if mode === 'watch'}
            <button class="primary" onclick={togglePause}>{paused ? 'Resume' : 'Pause'}</button>
            <button onclick={step} disabled={!paused || thinking}>Step a turn</button>
            <span class="speed" role="radiogroup" aria-label="Speed">
              {#each SPEEDS as option (option)}
                <button
                  role="radio"
                  aria-checked={speed === option}
                  class:chosen={speed === option}
                  onclick={() => (speed = option)}>{option}</button
                >
              {/each}
            </span>
          {/if}
        </div>
      {/if}
      {#if failure}
        <p class="failure">The machine could not move: {failure}</p>
      {/if}
      {#if review === null && !game.outcome}
        <Legend
          entries={legend}
          mine={attackOwner(viewer)}
          theirs={attackOwner(1 - viewer)}
        />
      {/if}
    </section>

    <aside class="side">
      <div class="panels">
        {#each [1 - orientation, orientation] as player (player)}
          <PlayerPanel
            game={shown}
            {player}
            seat={seat(player)}
            active={review === null && !game.outcome && game.currentPlayer === player}
            status={thinking && game.currentPlayer === player ? 'thinking…' : undefined}
          />
        {/each}
      </div>
      <EventLog
        {turns}
        onhighlight={(squares) => (highlight = squares)}
        onreview={game.outcome ? startReview : undefined}
        reviewing={review}
      />
    </aside>
  </div>
</div>

{#if showRules}
  <RulesOverlay onclose={() => (showRules = false)} />
{/if}

{#if confirmRestart}
  <Modal label="Restart the game" compact onclose={() => (confirmRestart = false)}>
    <h2 class="head">Restart?</h2>
    <div class="confirm">
      <p>This game will be abandoned and a new one started from the opening position.</p>
      <div class="actions">
        <button class="primary" onclick={restart}>Restart</button>
        <button data-autofocus onclick={() => (confirmRestart = false)}>Keep playing</button>
      </div>
    </div>
  </Modal>
{/if}

<style>
  .screen {
    --bar-h: 3.6rem;
    min-height: 100dvh;
    display: grid;
    grid-template-rows: auto 1fr;
  }

  .bar {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 0.8rem;
    padding: 0.5rem 1rem;
    min-height: var(--bar-h);
    border-bottom: 2px solid var(--copper);
    background: rgba(241, 233, 212, 0.75);
    position: relative;
    z-index: 10;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    font-size: 1.5rem;
  }

  h1 {
    font-size: 1.6rem;
    font-weight: 900;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .banner {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    font-family: var(--font-display);
    font-weight: 800;
    font-size: 1.02rem;
    background: var(--navy);
    color: var(--peach);
    padding: 0.3rem 0.95rem;
    border-radius: 999px;
    border: 1.5px solid var(--copper);
    white-space: nowrap;
  }

  .banner .turn {
    font-weight: 600;
    opacity: 0.8;
  }

  .disc {
    width: 0.8rem;
    height: 0.8rem;
    border-radius: 50%;
    flex: none;
  }

  .disc.side-0 {
    background: var(--ivory);
    box-shadow: 0 0 0 2px var(--ivory-rim);
  }

  .disc.side-1 {
    background: var(--umber);
    box-shadow: 0 0 0 2px var(--peach);
  }

  .controls {
    display: flex;
    justify-content: flex-end;
    gap: 0.4rem;
  }

  .options {
    position: relative;
  }

  .short {
    display: none;
  }

  .catcher {
    position: fixed;
    inset: 0;
    z-index: 29;
  }

  .popover {
    position: absolute;
    right: 0;
    top: calc(100% + 0.4rem);
    z-index: 30;
    display: grid;
    gap: 0.55rem;
    padding: 0.8rem 0.9rem;
    min-width: 13rem;
    box-shadow: var(--shadow-lift);
  }

  .popover hr {
    border: none;
    border-top: 1px solid var(--copper-soft);
  }

  /* The board takes whatever the window leaves it, in both directions: the
     sidebar on the left, the prompt beneath. */
  .body {
    display: grid;
    grid-template-columns: minmax(16rem, 21rem) minmax(0, 1fr);
    grid-template-rows: 1fr auto;
    grid-template-areas:
      'side board'
      'side prompt';
    gap: 0.6rem 1.2rem;
    padding: 0.8rem 1rem 0.9rem;
    height: calc(100dvh - var(--bar-h));
    min-height: 34rem;
  }

  .board-area {
    grid-area: board;
    position: relative;
    display: grid;
    place-items: center;
    min-height: 0;
    container-type: size;
    --avail-w: 100cqw;
    --avail-h: 100cqh;
  }

  .prompt {
    grid-area: prompt;
    justify-self: center;
    width: min(46rem, 100%);
    /* Tall enough for every phase's prompt, so the board above never resizes
       between one decision and the next. */
    min-height: 13rem;
    padding: 0.55rem 1rem 0.6rem;
    display: grid;
    align-content: start;
    justify-items: center;
    gap: 0.4rem;
    text-align: center;
  }

  .side {
    grid-area: side;
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    min-height: 0;
  }

  .panels {
    display: grid;
    gap: 0.7rem;
  }

  .side :global(.log) {
    flex: 1;
  }

  .steps {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.2rem 0.9rem;
    list-style: none;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 0.8rem;
    color: var(--ink-faint);
  }

  .steps li {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
  }

  .steps li.now {
    color: var(--ink);
  }

  .steps li.done {
    opacity: 0.55;
  }

  /* The sheet numbers its steps in peach diamonds on a copper edge. */
  .diamond {
    display: inline-grid;
    place-items: center;
    width: 1.15rem;
    height: 1.15rem;
    transform: rotate(45deg);
    border: 1.5px solid var(--copper);
    background: var(--parchment);
    flex: none;
  }

  .diamond span {
    transform: rotate(-45deg);
    font-size: 0.72rem;
    line-height: 1;
  }

  .now .diamond {
    background: var(--peach);
  }

  .step-token {
    width: 1.15rem;
    height: 1.15rem;
  }

  .line {
    font-size: 1.04rem;
    font-weight: 600;
  }

  /* Two lines are always kept for the hint, so the panel never jumps. */
  .guidance {
    font-size: 0.88rem;
    color: var(--ink-soft);
    font-style: italic;
    min-height: 2.6em;
    line-height: 1.3;
    max-width: 38rem;
  }

  .guidance.live {
    color: var(--ink);
    font-style: normal;
    font-weight: 600;
  }

  .actions {
    display: flex;
    gap: 0.45rem;
    flex-wrap: wrap;
    justify-content: center;
    align-items: center;
    min-height: 2.3rem;
  }

  .count {
    font-weight: 700;
    min-width: 4rem;
  }

  .speed {
    display: inline-flex;
    border: 1.5px solid var(--copper);
    border-radius: 8px;
    overflow: hidden;
  }

  .speed button {
    border: none;
    border-radius: 0;
    text-transform: capitalize;
    padding: 0.35rem 0.7rem;
  }

  .speed button + button {
    border-left: 1px solid var(--copper-soft);
  }

  .speed button.chosen {
    background: var(--navy);
    color: var(--peach);
  }

  .failure {
    color: var(--attack-deep);
    font-size: 0.85rem;
  }

  .end-card {
    position: absolute;
    z-index: 20;
    width: min(26rem, 92%);
    box-shadow: var(--shadow-lift);
    animation: rise 0.35s ease-out;
  }

  .end-card .head {
    font-size: 1.5rem;
  }

  .end-body {
    display: grid;
    gap: 0.8rem;
    justify-items: center;
    padding: 0.9rem 1.1rem 1rem;
    text-align: center;
  }

  .verdict {
    font-size: 1.05rem;
    font-weight: 600;
  }

  .stats {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.2rem 0.8rem;
    font-size: 0.9rem;
    text-align: left;
  }

  .stats dt {
    font-weight: 800;
  }

  .link {
    background: none;
    border: none;
    text-decoration: underline;
    font-weight: 600;
    font-size: 0.85rem;
    color: var(--ink-soft);
  }

  .link:hover:not(:disabled) {
    background: none;
    color: var(--ink);
  }

  .confirm {
    display: grid;
    gap: 1rem;
    padding: 1rem 1.2rem 1.2rem;
  }

  @keyframes rise {
    from {
      transform: translateY(14px);
      opacity: 0;
    }
  }

  /* A phone or a tablet held upright: the board first, the prompt under it,
     the panels and the dispatches below that. */
  @media (max-width: 900px) {
    .screen {
      --bar-h: 3.1rem;
    }

    .bar {
      grid-template-columns: auto 1fr auto;
      padding: 0.4rem 0.6rem;
      gap: 0.5rem;
    }

    .wide {
      display: none;
    }

    h1 {
      font-size: 1.15rem;
    }

    .banner {
      justify-self: center;
      font-size: 0.85rem;
      padding: 0.2rem 0.65rem;
    }

    .banner .turn {
      display: none;
    }

    .controls button {
      padding: 0.35rem 0.6rem;
      font-size: 0.85rem;
    }

    .body {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: auto auto auto;
      grid-template-areas: 'board' 'prompt' 'side';
      height: auto;
      min-height: 0;
      padding: 0.5rem 0.5rem 1rem;
      gap: 0.5rem;
    }

    .board-area {
      container-type: inline-size;
      --avail-h: calc(100svh - var(--bar-h) - 12.5rem);
    }

    .prompt {
      min-height: 8.6rem;
      padding: 0.45rem 0.6rem;
    }

    .steps {
      gap: 0.5rem;
      font-size: 0.72rem;
    }

    .panels {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 0.45rem;
    }

    .side :global(.log ol) {
      max-height: 16rem;
    }
  }

  @media (max-width: 480px) {
    .full {
      display: none;
    }

    .short {
      display: inline;
      font-size: 1.05rem;
      line-height: 1;
    }

    svg.short {
      width: 1.05rem;
      height: 1.05rem;
      vertical-align: -0.2rem;
      fill: none;
      stroke: currentColor;
      stroke-width: 2;
      stroke-linecap: round;
      stroke-linejoin: round;
    }

    .controls button {
      min-width: 2.3rem;
    }
  }

  /* A phone on its side: the board down the left at full height, everything
     else in a column beside it. */
  @media (max-height: 540px) and (orientation: landscape) {
    .screen {
      --bar-h: 2.8rem;
    }

    .bar {
      padding: 0.25rem 0.6rem;
    }

    .wide {
      display: none;
    }

    h1 {
      font-size: 1.1rem;
    }

    .banner .turn {
      display: none;
    }

    .body {
      grid-template-columns: auto minmax(0, 1fr);
      grid-template-rows: auto 1fr;
      grid-template-areas:
        'board prompt'
        'board side';
      height: calc(100dvh - var(--bar-h));
      min-height: 0;
      padding: 0.3rem 0.5rem;
      gap: 0.4rem 0.6rem;
    }

    .board-area {
      container-type: normal;
      --avail-w: 62vw;
      --avail-h: calc(100dvh - var(--bar-h) - 0.6rem);
    }

    .prompt {
      min-height: 0;
      width: 100%;
    }

    .steps {
      display: none;
    }

    .side {
      overflow-y: auto;
    }

    .panels {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 0.4rem;
    }
  }
</style>
