<script lang="ts">
  import { cubicOut } from 'svelte/easing'
  import { fade } from 'svelte/transition'
  import Mark from './Mark.svelte'
  import PieceIcon from './PieceIcon.svelte'
  import TerrainIcon from './TerrainIcon.svelte'
  import TokenIcon from './TokenIcon.svelte'
  import type { LegendEntry, LegendKey } from './Legend.svelte'
  import {
    describeMoveOutcome,
    describeSlideOutcome,
    describeThreat,
    lineName,
    squareName,
  } from '../data/log'
  import { squareIndex } from '../data/turns'
  import type { PieceIds, TurnMarks } from '../data/turns'
  import { applyChoice, attackShots, pieceReach } from '../engine/wasmEngine'
  import {
    BOARD_COLS,
    BOARD_ROWS,
    NEUTRAL,
    PIECE_NAMES,
    PLAYER_NAMES,
    TOKEN_KINDS,
    TOKEN_NAMES,
    attackingToken,
    castleSlotAt,
    isHilltop,
    lineOf,
    pieceAt,
    sameSquare,
    squareKey,
    tokenOf,
  } from '../data/types'
  import type {
    AttackPreview,
    Choice,
    GameState,
    MoveOutcome,
    SlideOutcome,
    Square,
    Threat,
    TokenKind,
  } from '../data/types'

  interface Props {
    game: GameState
    /** The decisions the board can make for the viewer; empty when none is
        theirs to make. While the activation order is open this holds both the
        moves and the first attacker's targets, and whichever is taken first
        goes first. */
    choices: Choice[]
    /** The position the move choices are made in, which is not `game` while
        the order is still open. */
    moveState: GameState
    /** What each side's Attack-side token bears on. */
    attacks: Record<number, AttackPreview | null>
    moveOutcomes: MoveOutcome[]
    slideOutcomes: SlideOutcome[]
    /** Attackers of the Attack in progress still to choose, the current first. */
    pendingAttackers: Square[]
    /** The token the move choices belong to, which colours them. */
    moveToken: TokenKind | null
    /** The side the controls and the marks answer to. */
    viewer: number
    /** The side drawn along the bottom. */
    orientation: number
    showThreats: boolean
    showOutcomes: boolean
    /** Stable piece identities, so a move animates as the piece travelling. */
    ids: PieceIds
    /** Last turn's moves, kills and slide, left on the board until the next. */
    marks: TurnMarks | null
    /** Squares a hovered dispatch is about. */
    highlight: Square[]
    /** Squares the latest Attack emptied, flashed once. */
    destroyed: Square[]
    /** Bumped whenever the position is replaced wholesale — undo, review — so
        pieces swap without animating a journey they never made. */
    epoch: number
    /** "Your" or "Umber's": whose Attack a hint is about. */
    attackOwner: (player: number) => string
    onChoice: (choice: Choice) => void
    onhint: (hint: string | null) => void
    onlegend: (entries: LegendEntry[]) => void
  }

  let {
    game,
    choices,
    moveState,
    attacks,
    moveOutcomes,
    slideOutcomes,
    pendingAttackers,
    moveToken,
    viewer,
    orientation,
    showThreats,
    showOutcomes,
    ids,
    marks,
    highlight,
    destroyed,
    epoch,
    attackOwner,
    onChoice,
    onhint,
    onlegend,
  }: Props = $props()

  const TONES: Record<TokenKind, string> = { march: 'var(--march)', flank: 'var(--flank)' }
  const SIDE_TONES = ['var(--ivory-rim)', 'var(--umber)']

  let selected = $state<Square | null>(null)
  let hovered = $state<Square | null>(null)
  let hoveredSlide = $state<SlideOutcome | null>(null)
  /** A touch has picked this slide or destination; a second touch commits. */
  let pendingTap = $state<{ slide: number } | { to: Square } | null>(null)
  let lastPointer = 'mouse'
  let focusKey = $state(squareKey({ row: (BOARD_ROWS - 1) / 2, col: (BOARD_COLS - 1) / 2 }))
  let gridEl = $state<HTMLDivElement | null>(null)

  let near = $derived(orientation)
  let far = $derived(1 - orientation)

  /** Ivory's view is the rules sheet's: its back row along the bottom and
      column a on the left. Umber's is a half turn from it. Reordering the
      indices rather than rotating the grid keeps the art upright. */
  let rows = $derived(drawOrder(BOARD_ROWS, orientation === 0))
  let cols = $derived(drawOrder(BOARD_COLS, orientation === 1))

  function drawOrder(length: number, reversed: boolean): number[] {
    const line = Array.from({ length }, (_, i) => i)
    return reversed ? line.reverse() : line
  }

  function at(square: Square): string {
    return `--r:${rows.indexOf(square.row)};--c:${cols.indexOf(square.col)}`
  }

  let moveTone = $derived(moveToken ? TONES[moveToken] : 'var(--navy)')
  let moves = $derived(choices.filter((choice) => choice.type === 'move'))
  let movable = $derived(new Set(moves.map((choice) => squareKey(choice.from))))

  // A choice landed, so any half-finished selection is about a position that
  // no longer exists. When one piece is the only one that can move there is
  // nothing to pick: select it so its destinations are already on the board.
  $effect(() => {
    void game
    selected = movable.size === 1 ? moves[0].from : null
    hovered = null
    hoveredSlide = null
    pendingTap = null
  })

  let slideTargets = $derived(new Map(slideOutcomes.map((outcome) => [outcome.line, outcome])))
  let slidingToken = $derived(slideOutcomes[0]?.token ?? null)

  /** The one piece now choosing a target, while an Attack walks its line. */
  let attacker = $derived.by(() => {
    for (const choice of choices) {
      if (choice.type === 'attack' || choice.type === 'holdFire') return choice.from
    }
    return null
  })

  let targets = $derived(
    keys(choices.flatMap((choice) => (choice.type === 'attack' ? [choice.target] : []))),
  )

  let destinations = $derived(
    selected === null
      ? new Set<string>()
      : keys(moves.filter((choice) => sameSquare(choice.from, selected!)).map((c) => c.to)),
  )

  function threatened(threat: Threat): Square[] {
    return [...threat.threatenedPieces, ...threat.threatenedCastles]
  }

  function keys(squares: Square[]): Set<string> {
    return new Set(squares.map(squareKey))
  }

  let outcomeByDestination = $derived.by(() => {
    const map = new Map<string, MoveOutcome>()
    if (!selected) return map
    for (const outcome of moveOutcomes) {
      if (sameSquare(outcome.from, selected)) map.set(squareKey(outcome.to), outcome)
    }
    return map
  })

  /** Pieces holding a move that would bring something into the Attack. */
  let armingMovers = $derived(
    showOutcomes
      ? keys(moveOutcomes.filter((o) => threatened(o).length > 0).map((o) => o.from))
      : new Set<string>(),
  )

  function aims(key: string): boolean {
    const outcome = outcomeByDestination.get(key)
    return showOutcomes && outcome !== undefined && threatened(outcome).length > 0
  }

  /** The destination being considered: under the cursor, or picked by a tap. */
  let landing = $derived.by(() => {
    const considered = pendingTap && 'to' in pendingTap ? pendingTap.to : hovered
    // Only a move still on offer: a choice can land before the selection
    // that led to it is cleared, and the engine must never be asked to
    // preview a move that is no longer legal.
    return considered && destinations.has(squareKey(considered)) ? considered : null
  })

  /** What the destination being considered would bring into the Attack. */
  let inReach = $derived.by(() => {
    if (!showOutcomes || !landing) return new Set<string>()
    const outcome = outcomeByDestination.get(squareKey(landing))
    return outcome ? keys(threatened(outcome)) : new Set<string>()
  })

  /** The Attack Zone of the piece in question: where it would land, the piece
      picked, or the piece under the cursor. The engine draws it, so a siege
      engine's zone is the castles it can reach and nothing else. */
  let zone = $derived.by(() => {
    if (landing && selected) {
      const next = applyChoice(moveState, {
        type: 'move',
        from: selected,
        to: landing,
      })
      return keys(pieceReach(next.state, landing)?.zone ?? [])
    }
    const origin = hovered && pieceAt(game, hovered) ? hovered : selected
    if (!origin) return new Set<string>()
    return keys(pieceReach(game, origin)?.zone ?? [])
  })

  /** What the viewer's Attack can take: the armed token's reach, or the one a
      considered slide would arm. Left out while an attacker is choosing, where
      the targets say it better. */
  let ownMarks = $derived.by(() => {
    if (hoveredSlide && showOutcomes) return keys(threatened(hoveredSlide))
    const preview = attacker !== null ? null : attacks[viewer]
    return preview ? keys(threatened(preview)) : new Set<string>()
  })

  let slideZone = $derived(
    hoveredSlide && showOutcomes ? keys(hoveredSlide.zone) : new Set<string>(),
  )

  let enemyMarks = $derived.by(() => {
    const preview = showThreats ? attacks[1 - viewer] : null
    return preview ? keys(threatened(preview)) : new Set<string>()
  })

  let queuedAttackers = $derived(keys(pendingAttackers.slice(1)))
  let previewMovers = $derived(keys(hoveredSlide?.movers ?? []))
  let highlighted = $derived(keys(highlight))
  let destroyedKeys = $derived(keys(destroyed))
  let lastFrom = $derived(keys(marks?.moves.map((move) => move.from) ?? []))
  let killed = $derived(keys(marks?.kills.map((kill) => kill.square) ?? []))
  let squares = $derived(rows.flatMap((row) => cols.map((col) => ({ row, col }))))
  let lastTo = $derived(keys(marks?.moves.map((move) => move.to) ?? []))

  /** The two lines an Attack is armed on: whose decides the tint. */
  function band(square: Square): string {
    for (const [player, cls] of [
      [1 - viewer, 'band-threat'],
      [viewer, 'band-own'],
    ] as const) {
      const kind = attackingToken(game, player)
      if (kind && lineOf(kind, square) === tokenOf(game, player, kind).line) return cls
    }
    return ''
  }

  let pieces = $derived.by(() => {
    const list = []
    for (let row = 0; row < BOARD_ROWS; row++) {
      for (let col = 0; col < BOARD_COLS; col++) {
        const piece = game.board[row][col]
        if (!piece) continue
        const index = squareIndex({ row, col })
        list.push({ id: ids[index] || -1 - index, square: { row, col }, piece })
      }
    }
    return list
  })

  /** Where each token sits on its track. A side's Flank token rides the edge
      beside its own back row, its March token the edge to its right. */
  function tokenPlace(player: number, kind: TokenKind, line: number): string {
    const ownEdge = player === near
    if (kind === 'flank') {
      return `--x:calc(var(--track) + ${cols.indexOf(line) + 0.5} * var(--cell));--y:${
        ownEdge ? `calc(1.5 * var(--track) + ${BOARD_ROWS} * var(--cell))` : 'calc(var(--track) / 2)'
      }`
    }
    return `--y:calc(var(--track) + ${rows.indexOf(line) + 0.5} * var(--cell));--x:${
      ownEdge ? `calc(1.5 * var(--track) + ${BOARD_COLS} * var(--cell))` : 'calc(var(--track) / 2)'
    }`
  }

  let tokens = $derived(
    [0, 1].flatMap((player) =>
      TOKEN_KINDS.map((kind) => {
        const token = tokenOf(game, player, kind)
        return {
          key: `${player}-${kind}`,
          player,
          kind,
          side: token.side,
          place: tokenPlace(player, kind, token.line),
          sliding: player === viewer && kind === slidingToken,
        }
      }),
    ),
  )

  let slideGhost = $derived(
    marks?.slide ? tokenPlace(marks.player, marks.slide.token, marks.slide.from) : null,
  )

  /** The keys the board is drawing right now, for the legend under it. */
  let legend = $derived.by(() => {
    const entries: LegendEntry[] = []
    const add = (key: LegendKey, when: boolean, tone?: string) => {
      if (when) entries.push({ key, tone })
    }
    add('slide', slideTargets.size > 0, slidingToken ? TONES[slidingToken] : undefined)
    add('movable', movable.size > 0, moveTone)
    add('destination', destinations.size > 0, moveTone)
    add('aim', armingMovers.size > 0, moveTone)
    add('attacker', attacker !== null)
    add('target', targets.size > 0)
    add('reticle', ownMarks.size > 0)
    add('frame', enemyMarks.size > 0)
    add('zone', zone.size > 0 || slideZone.size > 0)
    add('last', lastTo.size > 0 || lastFrom.size > 0)
    add('kill', (marks?.kills.length ?? 0) > 0)
    return entries
  })

  $effect(() => onlegend(legend))

  function hintFor(square: Square): string | null {
    const key = squareKey(square)
    const outcome = outcomeByDestination.get(key)
    if (outcome) {
      const text = showOutcomes ? describeMoveOutcome(game, outcome) : null
      return lastPointer === 'touch' ? `${text ?? ''} Tap again to move.`.trim() : text
    }
    for (const player of [1 - viewer, viewer]) {
      const owned = player === viewer ? ownMarks : enemyMarks
      if (!owned.has(key) && !(player === viewer && targets.has(key))) continue
      const kind = attackingToken(game, player)
      if (!kind) continue
      const when = player === game.currentPlayer ? 'this turn' : 'next turn'
      const where = lineName(kind, tokenOf(game, player, kind).line)
      const text = describeThreat(
        game,
        square,
        attackShots(game, player, kind),
        `${attackOwner(player)} Attack along ${where} ${when}`,
      )
      if (text) return text
    }
    const piece = pieceAt(game, square)
    if (!piece || piece.owner === NEUTRAL) return null
    const who = `${PLAYER_NAMES[piece.owner]}'s ${PIECE_NAMES[piece.kind]} on ${squareName(square)}`
    return zone.size > 0
      ? `${who}: its Attack Zone is shaded.`
      : `${who}: no enemy castle in range.`
  }

  function enterTrack(line: number) {
    const outcome = slideTargets.get(line)
    if (!outcome) return
    hoveredSlide = outcome
    onhint(showOutcomes ? describeSlideOutcome(game, outcome) : null)
  }

  function leaveTrack() {
    if (pendingTap && 'slide' in pendingTap) return
    hoveredSlide = null
    onhint(null)
  }

  function clickTrack(line: number) {
    const outcome = slideTargets.get(line)
    if (!outcome) return
    const confirmed = pendingTap && 'slide' in pendingTap && pendingTap.slide === line
    if (lastPointer === 'touch' && !confirmed) {
      pendingTap = { slide: line }
      hoveredSlide = outcome
      const preview = showOutcomes ? describeSlideOutcome(game, outcome) : ''
      onhint(`${preview} Tap again to slide here.`.trim())
      return
    }
    pendingTap = null
    onChoice({ type: 'slide', line })
  }

  function enterSquare(square: Square) {
    hovered = square
    if (pendingTap) return
    onhint(hintFor(square))
  }

  function leaveSquare() {
    hovered = null
    if (pendingTap) return
    onhint(null)
  }

  function clickSquare(square: Square) {
    const key = squareKey(square)
    focusKey = key
    if (attacker && targets.has(key)) {
      onChoice({ type: 'attack', from: attacker, target: square })
      return
    }
    if (selected && destinations.has(key)) {
      const confirmed = pendingTap && 'to' in pendingTap && sameSquare(pendingTap.to, square)
      if (lastPointer === 'touch' && !confirmed) {
        pendingTap = { to: square }
        onhint(hintFor(square))
        return
      }
      onChoice({ type: 'move', from: selected, to: square })
      return
    }
    pendingTap = null
    if (movable.has(key)) {
      selected = selected && sameSquare(selected, square) ? null : square
      onhint(null)
      return
    }
    selected = null
    if (lastPointer === 'touch') onhint(hintFor(square))
  }

  const ARROWS: Record<string, [number, number]> = {
    ArrowUp: [-1, 0],
    ArrowDown: [1, 0],
    ArrowLeft: [0, -1],
    ArrowRight: [0, 1],
  }

  /** The board is one stop in the tab order; arrows walk it, as drawn. */
  function navigate(event: KeyboardEvent) {
    const step = ARROWS[event.key]
    if (!step) return
    event.preventDefault()
    const [row, col] = focusKey.split(',').map(Number)
    const r = Math.min(BOARD_ROWS - 1, Math.max(0, rows.indexOf(row) + step[0]))
    const c = Math.min(BOARD_COLS - 1, Math.max(0, cols.indexOf(col) + step[1]))
    focusKey = squareKey({ row: rows[r], col: cols[c] })
    gridEl?.querySelector<HTMLElement>(`[data-key="${focusKey}"]`)?.focus()
  }

  function describeSquare(square: Square): string {
    const key = squareKey(square)
    const parts = [squareName(square)]
    const piece = pieceAt(game, square)
    if (piece) {
      parts.push(
        piece.owner === NEUTRAL
          ? 'rock'
          : `${PLAYER_NAMES[piece.owner]} ${PIECE_NAMES[piece.kind]}`,
      )
    }
    const slot = castleSlotAt(square)
    if (slot && game.castles[slot.owner][slot.index]) {
      parts.push(`${PLAYER_NAMES[slot.owner]} castle`)
    } else if (isHilltop(game, square)) {
      parts.push('hilltop')
    }
    if (movable.has(key)) parts.push('can move')
    if (destinations.has(key)) parts.push('destination')
    if (targets.has(key)) parts.push('target')
    if (enemyMarks.has(key)) parts.push('under threat')
    return parts.join(', ')
  }

  function flip(_node: Element) {
    return {
      duration: 420,
      easing: cubicOut,
      css: (t: number) => `transform: perspective(200px) rotateY(${(1 - t) * 180}deg)`,
    }
  }
</script>

<svelte:window
  onpointerdown={(event) => (lastPointer = event.pointerType)}
  onkeydown={(event) => {
    if (event.key === 'Escape') {
      selected = null
      pendingTap = null
      hoveredSlide = null
      onhint(null)
    }
  }}
/>

<div class="board-frame" style={`--rows:${BOARD_ROWS};--cols:${BOARD_COLS}`}>
  <div class="grid" bind:this={gridEl} role="group" aria-label="Board">
    {#each [far, near] as player, edge (player)}
      {@const rowTrack = edge === 0 ? 1 : BOARD_ROWS + 2}
      {#each cols as col, index (col)}
        {@const live = player === viewer && slidingToken === 'flank' && slideTargets.has(col)}
        {#if live}
          <button
            class="track live"
            class:pending={pendingTap !== null && 'slide' in pendingTap && pendingTap.slide === col}
            style={`grid-row:${rowTrack};grid-column:${index + 2};--tone:${TONES.flank}`}
            onclick={() => clickTrack(col)}
            onmouseenter={() => enterTrack(col)}
            onmouseleave={leaveTrack}
            onfocus={() => enterTrack(col)}
            onblur={leaveTrack}
            aria-label={`Slide the ${TOKEN_NAMES.flank} token to ${lineName('flank', col)}`}
          >
            <span class="slot"></span>
          </button>
        {:else}
          <span class="track" style={`grid-row:${rowTrack};grid-column:${index + 2};--tone:${TONES.flank}`} aria-hidden="true"><span class="slot"></span></span>
        {/if}
      {/each}
    {/each}

    {#each [far, near] as player, edge (player)}
      {@const colTrack = edge === 0 ? 1 : BOARD_COLS + 2}
      {#each rows as row, index (row)}
        {@const live = player === viewer && slidingToken === 'march' && slideTargets.has(row)}
        {#if live}
          <button
            class="track live"
            class:pending={pendingTap !== null && 'slide' in pendingTap && pendingTap.slide === row}
            style={`grid-row:${index + 2};grid-column:${colTrack};--tone:${TONES.march}`}
            onclick={() => clickTrack(row)}
            onmouseenter={() => enterTrack(row)}
            onmouseleave={leaveTrack}
            onfocus={() => enterTrack(row)}
            onblur={leaveTrack}
            aria-label={`Slide the ${TOKEN_NAMES.march} token to ${lineName('march', row)}`}
          >
            <span class="slot"></span>
          </button>
        {:else}
          <span class="track" style={`grid-row:${index + 2};grid-column:${colTrack};--tone:${TONES.march}`} aria-hidden="true"><span class="slot"></span></span>
        {/if}
      {/each}
    {/each}

    {#each rows as row, r (row)}
      {#each cols as col, c (col)}
        {@const square = { row, col }}
        {@const key = squareKey(square)}
        {@const slot = castleSlotAt(square)}
        {@const standing = slot !== null && game.castles[slot.owner][slot.index]}
        <button
          class={`square ${band(square)}`}
          class:terrain={standing || isHilltop(game, square)}
          class:live={movable.has(key) || destinations.has(key) || targets.has(key)}
          style={`grid-row:${r + 2};grid-column:${c + 2}`}
          data-key={key}
          tabindex={key === focusKey ? 0 : -1}
          onkeydown={navigate}
          onclick={() => clickSquare(square)}
          onmouseenter={() => enterSquare(square)}
          onmouseleave={leaveSquare}
          onfocus={() => enterSquare(square)}
          onblur={leaveSquare}
          aria-label={describeSquare(square)}
        >
          {#if standing}
            <span class="castle"><TerrainIcon kind="castle" /></span>
          {:else if isHilltop(game, square)}
            <span class="hill"><TerrainIcon kind="hilltop" /></span>
          {/if}
          {#if c === 0}<span class="coord rank">{row + 1}</span>{/if}
          {#if r === BOARD_ROWS - 1}<span class="coord file">{'abcdefghi'[col]}</span>{/if}
        </button>
      {/each}
    {/each}
    <span class="frame-line" aria-hidden="true"></span>
  </div>

  <div class="layer" aria-hidden="true">
    {#each squares as square (squareKey(square))}
      {@const key = squareKey(square)}
      {@const line = hoveredSlide !== null && lineOf(hoveredSlide.token, square) === hoveredSlide.line}
      {#if line || lastTo.has(key) || lastFrom.has(key) || zone.has(key) || slideZone.has(key) || highlighted.has(key)}
        <span class="cell" style={at(square)}>
          {#if line}<Mark kind="line" tone={TONES[hoveredSlide?.token ?? 'march']} />{/if}
          {#if lastTo.has(key) || lastFrom.has(key)}<Mark kind="last" />{/if}
          {#if zone.has(key) || slideZone.has(key)}<Mark kind="zone" />{/if}
          {#if highlighted.has(key)}<Mark kind="highlight" />{/if}
        </span>
      {/if}
    {/each}
    {#each marks?.kills ?? [] as kill (squareKey(kill.square))}
      <span class="cell" style={at(kill.square)}>
        {#if kill.casualty.type === 'piece'}
          <span class="corpse"><PieceIcon kind={kill.casualty.piece.kind} owner={kill.casualty.piece.owner} /></span>
        {/if}
      </span>
    {/each}
  </div>

  {#key epoch}
    <div class="layer pieces" aria-hidden="true">
      {#each pieces as entry (entry.id)}
        <span class="piece-pos" style={at(entry.square)} out:fade={{ duration: 380 }}>
          <span class={`piece ${entry.piece.owner === NEUTRAL ? 'rock' : ''}`}>
            <PieceIcon kind={entry.piece.kind} owner={entry.piece.owner} />
          </span>
        </span>
      {/each}
    </div>
  {/key}

  <div class="layer" aria-hidden="true">
    {#each squares as square (squareKey(square))}
      {@const key = squareKey(square)}
      {@const firing = attacker !== null && sameSquare(attacker, square)}
      {@const picked = selected !== null && sameSquare(selected, square)}
      {@const tapped = pendingTap !== null && 'to' in pendingTap && sameSquare(pendingTap.to, square)}
      {#if firing || picked || tapped || lastFrom.has(key) || killed.has(key) || previewMovers.has(key) || movable.has(key) || destinations.has(key) || targets.has(key) || ownMarks.has(key) || enemyMarks.has(key) || inReach.has(key) || queuedAttackers.has(key) || destroyedKeys.has(key)}
        <span class="cell" style={at(square)}>
          {#if lastFrom.has(key) && !pieceAt(game, square)}
            <Mark kind="ghost" tone={SIDE_TONES[marks?.player ?? 0]} />
          {/if}
          {#if killed.has(key)}<Mark kind="kill" />{/if}
          {#if enemyMarks.has(key)}<Mark kind="frame" />{/if}
          {#if previewMovers.has(key)}<Mark kind="ring" tone={TONES[hoveredSlide?.token ?? 'march']} />{/if}
          {#if picked}
            <Mark kind="selected" tone={moveTone} />
          {:else if movable.has(key)}
            <Mark kind="ring" tone={moveTone} />
          {/if}
          {#if armingMovers.has(key) && !picked}<Mark kind="badge" tone={moveTone} />{/if}
          {#if destinations.has(key)}<Mark kind={aims(key) ? 'aim' : 'dot'} tone={moveTone} />{/if}
          {#if tapped}<Mark kind="pending" tone={moveTone} />{/if}
          {#if firing}<Mark kind="attacker" />{/if}
          {#if queuedAttackers.has(key)}<Mark kind="queued" />{/if}
          {#if targets.has(key)}
            <Mark kind="target" />
          {:else if ownMarks.has(key) || inReach.has(key)}
            <Mark kind="reticle" />
          {/if}
          {#if destroyedKeys.has(key)}<Mark kind="flash" />{/if}
        </span>
      {/if}
    {/each}
  </div>

  <div class="layer tokens" aria-hidden="true">
    {#if slideGhost}<span class="token-pos ghost" style={slideGhost}></span>{/if}
    {#each tokens as token (token.key)}
      <span class="token-pos" class:sliding={token.sliding} style={token.place}>
        {#if token.sliding}
          <svg class={`arrows ${token.kind}`} viewBox="0 0 100 20">
            <path d="M2 10 12 4v4h14v4H12v4ZM98 10 88 4v4H74v4h14v4Z" />
          </svg>
        {/if}
        {#key token.side}
          <span class="token" in:flip>
            <TokenIcon kind={token.kind} side={token.side} owner={token.player} />
          </span>
        {/key}
      </span>
    {/each}
  </div>
</div>

<style>
  .board-frame {
    --pad: 4px;
    /* The columns and two token tracks at 0.72 of a cell across; the rows
       and two tracks down. The cell is whichever of the space's two sides is
       tighter, divided by that many cells. The space comes from the parent as
       --avail-w and --avail-h, measured however its layout allows. */
    --cell: clamp(
      24px,
      min(
        calc((var(--avail-w, 100vw) - 2 * var(--pad)) / (var(--cols) + 1.44)),
        calc((var(--avail-h, 100vh) - 2 * var(--pad)) / (var(--rows) + 1.44))
      ),
      112px
    );
    --track: calc(var(--cell) * 0.72);
    --move-ms: 380ms;
    position: relative;
    padding: var(--pad);
    width: fit-content;
    touch-action: manipulation;
  }

  .grid {
    position: relative;
    display: grid;
    grid-template-columns: var(--track) repeat(var(--cols), var(--cell)) var(--track);
    grid-template-rows: var(--track) repeat(var(--rows), var(--cell)) var(--track);
  }

  .layer {
    position: absolute;
    inset: var(--pad);
    pointer-events: none;
  }

  .cell,
  .piece-pos {
    position: absolute;
    left: 0;
    top: 0;
    width: var(--cell);
    height: var(--cell);
    transform: translate(
      calc(var(--track) + var(--c) * var(--cell)),
      calc(var(--track) + var(--r) * var(--cell))
    );
  }

  .piece-pos {
    display: grid;
    place-items: center;
    transition: transform var(--move-ms) cubic-bezier(0.3, 0.7, 0.2, 1);
  }

  .piece {
    width: 84%;
    aspect-ratio: 1;
    border-radius: 50%;
    filter: drop-shadow(0 2px 2px rgba(43, 28, 10, 0.35));
  }

  .piece.rock {
    width: 78%;
    filter: none;
  }

  .corpse {
    position: absolute;
    inset: 12%;
    opacity: 0.32;
    filter: grayscale(0.6);
  }

  .track {
    display: grid;
    place-items: center;
    padding: 0;
    background: none;
    border: none;
    border-radius: 50%;
    cursor: default;
  }

  .track.live:hover {
    background: none;
  }

  /* The sheet marks every stop on a track with a small open ring. */
  .slot {
    width: max(6px, calc(var(--track) * 0.16));
    aspect-ratio: 1;
    border-radius: 50%;
    border: 1.5px solid var(--board-line);
    opacity: 0.7;
  }

  .track.live {
    cursor: pointer;
  }

  .track.live .slot {
    width: calc(var(--track) * 0.62);
    border: 2.5px dashed var(--tone);
    opacity: 1;
  }

  .track.live:hover .slot,
  .track.live:focus-visible .slot,
  .track.pending .slot {
    border-style: solid;
    background: color-mix(in srgb, var(--tone) 30%, transparent);
  }

  .square {
    position: relative;
    display: block;
    padding: 0;
    border: none;
    border-radius: 0;
    background: var(--cell);
    box-shadow: inset 0 0 0 0.75px var(--board-line);
    cursor: default;
  }

  .square:hover:not(:disabled) {
    background: var(--cell);
  }

  .square.terrain,
  .square.terrain:hover:not(:disabled) {
    background: var(--terrain);
  }

  .square.live {
    cursor: pointer;
  }

  .square:focus-visible {
    outline: 3px solid var(--march-deep);
    outline-offset: -3px;
    z-index: 1;
  }

  /* The lines an Attack is armed on: theirs reddened, yours shaded. */
  .square.band-threat {
    background-image: linear-gradient(rgba(227, 38, 43, 0.065), rgba(227, 38, 43, 0.065));
  }

  .square.band-own {
    background-image: linear-gradient(rgba(13, 23, 41, 0.06), rgba(13, 23, 41, 0.06));
  }

  .frame-line {
    grid-row: 2 / span var(--rows);
    grid-column: 2 / span var(--cols);
    border: 3px solid var(--board-line);
    pointer-events: none;
  }

  .castle,
  .hill {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    pointer-events: none;
  }

  .castle :global(svg) {
    width: 72%;
    height: 72%;
  }

  .hill :global(svg) {
    width: 76%;
    height: 76%;
  }

  .coord {
    position: absolute;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: max(9px, calc(var(--cell) * 0.17));
    line-height: 1;
    color: var(--board-line);
    opacity: 0.75;
    pointer-events: none;
    z-index: 4;
  }

  .coord.rank {
    top: 7%;
    left: 8%;
  }

  .coord.file {
    bottom: 6%;
    right: 9%;
  }

  .tokens {
    z-index: 5;
  }

  .token-pos {
    position: absolute;
    left: 0;
    top: 0;
    width: calc(var(--track) * 0.9);
    height: calc(var(--track) * 0.9);
    transform: translate(calc(var(--x) - 50%), calc(var(--y) - 50%));
    transition: transform 520ms cubic-bezier(0.3, 0.7, 0.2, 1);
  }

  .token-pos.ghost {
    border: 2px dashed var(--copper);
    border-radius: 50%;
    opacity: 0.7;
    transition: none;
  }

  .token {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    box-shadow: var(--shadow-soft);
  }

  /* The sheet draws a token that may slide with arrows along its edge. */
  .arrows {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 260%;
    height: 50%;
    transform: translate(-50%, -50%);
    fill: var(--copper-deep);
  }

  .arrows.march {
    transform: translate(-50%, -50%) rotate(90deg);
  }
</style>
