<script lang="ts">
  import PieceIcon from './PieceIcon.svelte'
  import TerrainIcon from './TerrainIcon.svelte'
  import TokenIcon from './TokenIcon.svelte'
  import { describeMoveOutcome, describeSlideOutcome, lineName, squareName } from '../data/log'
  import {
    BOARD_COLS,
    BOARD_ROWS,
    NEUTRAL,
    PIECE_NAMES,
    PLAYER_NAMES,
    TOKEN_KINDS,
    TOKEN_NAMES,
    castleSlotAt,
    isHilltop,
    lineOf,
    pieceAt,
    sameSquare,
    squareKey,
    tokenOf,
    zoneSquares,
  } from '../data/types'
  import type {
    AttackPreview,
    Choice,
    GameState,
    MoveOutcome,
    Piece,
    SlideOutcome,
    Square,
    Threat,
    TokenKind,
  } from '../data/types'

  interface Props {
    game: GameState
    /** Empty whenever it is not the local player's move. */
    legalChoices: Choice[]
    /** What each side's Attack-side token currently bears on. */
    attacks: Record<number, AttackPreview | null>
    /** What each move available now puts in reach; empty off the move phase. */
    moveOutcomes: MoveOutcome[]
    /** What each slide available now sets up; empty off the slide phase. */
    slideOutcomes: SlideOutcome[]
    /** Attackers of the Attack in progress still to choose, the current one first. */
    pendingAttackers: Square[]
    /** The side the controls answer to: your seat in a solo game, and whoever
        is to move when two players share the screen or neither of them is
        human. Never absent — see `orientation` for the side that stays put. */
    viewer: number
    /** The side along the bottom edge, facing the other across the board. */
    orientation: number
    showThreats: boolean
    /** Whether to look a step ahead: what a move or a slide would lead to. */
    showOutcomes: boolean
    /** Squares emptied by the Attack just resolved, flashed and then dropped. */
    destroyed: Square[]
    onChoice: (choice: Choice) => void
    /** A sentence about whatever is under the cursor, for the prompt panel. */
    onhint: (hint: string | null) => void
  }

  let {
    game,
    legalChoices,
    attacks,
    moveOutcomes,
    slideOutcomes,
    pendingAttackers,
    viewer,
    orientation,
    showThreats,
    showOutcomes,
    destroyed,
    onChoice,
    onhint,
  }: Props = $props()

  let selected: Square | null = $state(null)
  let hovered: Square | null = $state(null)
  let hoveredSlide: SlideOutcome | null = $state(null)

  /** Whichever side is drawn nearest holds the bottom and right tracks, so the
      four of them go by the edge they sit on rather than by whose they are. */
  let near = $derived(orientation)
  let far = $derived(1 - orientation)

  /** Ivory's view is the rules sheet's: its back row along the bottom and
      column a on the left. Umber's is a half turn from it, so rows and columns
      both run backwards. Reordering the indices rather than rotating the grid
      keeps coordinates and piece art upright. */
  let rows = $derived(drawOrder(BOARD_ROWS, orientation === 0))
  let cols = $derived(drawOrder(BOARD_COLS, orientation === 1))

  function drawOrder(length: number, reversed: boolean): number[] {
    const line = Array.from({ length }, (_, i) => i)
    return reversed ? line.reverse() : line
  }

  let moves = $derived(legalChoices.filter((choice) => choice.type === 'move'))

  let movable = $derived(new Set(moves.map((choice) => squareKey(choice.from))))

  // A choice landed, so any half-finished selection is about a position that no
  // longer exists. When one piece is the only one that can move there is
  // nothing to pick: select it so its destinations are already on the board.
  $effect(() => {
    void game
    selected = movable.size === 1 ? moves[0].from : null
    hovered = null
    hoveredSlide = null
  })

  let slideTargets = $derived(
    new Map(slideOutcomes.map((outcome) => [`${outcome.token}:${outcome.line}`, outcome])),
  )

  /** The one piece now choosing a target, while an Attack walks its line. */
  let attacker = $derived.by(() => {
    for (const choice of legalChoices) {
      if (choice.type === 'attack' || choice.type === 'holdFire') return choice.from
    }
    return null
  })

  let targets = $derived(
    new Set(
      legalChoices
        .filter((choice) => choice.type === 'attack')
        .map((choice) => squareKey(choice.target)),
    ),
  )

  let destinations = $derived(
    selected === null
      ? new Set<string>()
      : new Set(
          moves
            .filter((choice) => sameSquare(choice.from, selected!))
            .map((choice) => squareKey(choice.to)),
        ),
  )

  /** Everything one Attack could take: pieces, rocks among them, and castles. */
  function threatened(threat: Threat): Square[] {
    return [...threat.threatenedPieces, ...threat.threatenedCastles]
  }

  function targeting(outcome: MoveOutcome): boolean {
    return threatened(outcome).length > 0
  }

  /** The move to each destination of the selected piece, keyed by destination. */
  let outcomeByDestination = $derived.by(() => {
    const map = new Map<string, MoveOutcome>()
    if (!selected) return map
    for (const outcome of moveOutcomes) {
      if (sameSquare(outcome.from, selected)) map.set(squareKey(outcome.to), outcome)
    }
    return map
  })

  /** Pieces holding a move that would put something under the Attack. */
  let armingMovers = $derived(
    new Set(
      showOutcomes ? moveOutcomes.filter(targeting).map((o) => squareKey(o.from)) : [],
    ),
  )

  /** What the destination under the cursor would bring within reach. */
  let inReach = $derived.by(() => {
    if (!showOutcomes || !hovered) return new Set<string>()
    const outcome = outcomeByDestination.get(squareKey(hovered))
    if (!outcome) return new Set<string>()
    return keys(threatened(outcome))
  })

  /** The highlighted piece's Attack Zone — from where it would land, if the
      cursor is on one of its destinations. */
  let zoneUnderCursor = $derived.by(() => {
    const landing = hovered && destinations.has(squareKey(hovered)) ? hovered : null
    const origin = landing ?? selected ?? hovered
    if (!origin) return new Set<string>()
    const piece = pieceAt(game, landing ? selected! : origin)
    if (!piece) return new Set<string>()
    return keys(zoneSquares(origin, piece.kind))
  })

  function keys(squares: Square[]): Set<string> {
    return new Set(squares.map(squareKey))
  }

  /** The Attack the board is explaining: the armed token's, or the one a
      hovered slide would arm a turn from now. Suppressed while an attacker is
      choosing its target, where the legal targets say it better. */
  let ownAttack = $derived.by(() => {
    if (hoveredSlide && showOutcomes) {
      return {
        zone: keys(hoveredSlide.zone),
        marks: keys(threatened(hoveredSlide)),
      }
    }
    const preview = attacker !== null ? null : attacks[viewer]
    return {
      zone: keys(preview?.zone ?? []),
      marks: preview ? keys(threatened(preview)) : new Set<string>(),
    }
  })

  let enemyAttack = $derived.by(() => {
    const preview = showThreats ? attacks[1 - viewer] : null
    return {
      zone: keys(preview?.zone ?? []),
      marks: preview ? keys(threatened(preview)) : new Set<string>(),
    }
  })

  /** Attackers behind the one now choosing: the targets still to come. */
  let queuedAttackers = $derived(keys(pendingAttackers.slice(1)))
  let previewMovers = $derived.by(() => keys(hoveredSlide?.movers ?? []))
  let destroyedKeys = $derived(keys(destroyed))

  /** Lines each side's tokens currently name, for the faint band under them. */
  let bands = $derived.by(() => {
    const rows = new Map<number, string[]>()
    const cols = new Map<number, string[]>()
    for (let player = 0; player < 2; player++) {
      for (const kind of TOKEN_KINDS) {
        const token = tokenOf(game, player, kind)
        const target = kind === 'march' ? rows : cols
        const label = `${player}-${token.side}`
        target.set(token.line, [...(target.get(token.line) ?? []), label])
      }
    }
    return { rows, cols }
  })

  /** An armed line reads by whose it is: the line you are about to be
      attacked along is not the one you are about to attack along. */
  function bandClass(square: Square): string {
    const labels = [
      ...(bands.rows.get(square.row) ?? []),
      ...(bands.cols.get(square.col) ?? []),
    ]
    if (labels.includes(`${1 - viewer}-attack`)) return 'band-threat'
    if (labels.includes(`${viewer}-attack`)) return 'band-armed'
    if (labels.length > 0) return 'band-move'
    return ''
  }

  function onPreviewedLine(square: Square): boolean {
    return hoveredSlide !== null && lineOf(hoveredSlide.token, square) === hoveredSlide.line
  }

  function clickTrack(kind: TokenKind, line: number) {
    if (!slideTargets.has(`${kind}:${line}`)) return
    onChoice({ type: 'slide', line })
  }

  function enterTrack(kind: TokenKind, line: number) {
    const outcome = slideTargets.get(`${kind}:${line}`)
    if (!outcome) return
    hoveredSlide = outcome
    onhint(showOutcomes ? describeSlideOutcome(game, outcome) : null)
  }

  function leaveTrack() {
    hoveredSlide = null
    onhint(null)
  }

  function enterSquare(square: Square) {
    hovered = square
    const outcome = outcomeByDestination.get(squareKey(square))
    onhint(outcome && showOutcomes ? describeMoveOutcome(game, outcome) : null)
  }

  function leaveSquare() {
    hovered = null
    onhint(null)
  }

  function clickSquare(square: Square) {
    if (attacker && targets.has(squareKey(square))) {
      onChoice({ type: 'attack', from: attacker, target: square })
      return
    }
    if (selected && destinations.has(squareKey(square))) {
      onChoice({ type: 'move', from: selected, to: square })
      return
    }
    if (movable.has(squareKey(square))) {
      selected = selected && sameSquare(selected, square) ? null : square
      return
    }
    selected = null
  }

  function trackActive(player: number, kind: TokenKind, line: number): boolean {
    return viewer === player && slideTargets.has(`${kind}:${line}`)
  }

  function trackHasToken(player: number, kind: TokenKind, line: number): boolean {
    return tokenOf(game, player, kind).line === line
  }

  function squareTitle(square: Square): string {
    const piece = pieceAt(game, square)
    const parts: string[] = [squareName(square)]
    if (piece) parts.push(PIECE_NAMES[piece.kind])
    const slot = castleSlotAt(square)
    if (slot) parts.push(game.castles[slot.owner][slot.index] ? 'castle' : 'destroyed castle')
    if (isHilltop(game, square)) parts.push('hilltop')
    return parts.join(' · ')
  }

  /** Each side's pieces sit on a disc of its colour; a rock sits on nothing. */
  function pieceClass(piece: Piece): string {
    if (piece.owner === NEUTRAL) return 'rock'
    return piece.owner === 0 ? 'ivory' : 'umber'
  }
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === 'Escape') selected = null
  }}
/>

<div class="board-frame">
  <div class="grid">
    <div class="corner"></div>
    {#each cols as col (col)}
      <button
        class="track flank"
        class:live={trackActive(far, 'flank', col)}
        class:occupied={trackHasToken(far, 'flank', col)}
        disabled={!trackActive(far, 'flank', col)}
        onclick={() => clickTrack('flank', col)}
        onmouseenter={() => enterTrack('flank', col)}
        onmouseleave={leaveTrack}
        onfocus={() => enterTrack('flank', col)}
        onblur={leaveTrack}
        aria-label={`${PLAYER_NAMES[far]} ${TOKEN_NAMES.flank} token to ${lineName('flank', col)}`}
      >
        {#if trackHasToken(far, 'flank', col)}
          <span class={`token ${far === 0 ? 'ivory' : 'umber'}`}>
            <TokenIcon kind="flank" side={tokenOf(game, far, 'flank').side} owner={far} />
          </span>
        {:else}
          <span class="slot"></span>
        {/if}
      </button>
    {/each}
    <div class="corner"></div>

    {#each rows as row (row)}
      <button
        class="track march"
        class:live={trackActive(far, 'march', row)}
        class:occupied={trackHasToken(far, 'march', row)}
        disabled={!trackActive(far, 'march', row)}
        onclick={() => clickTrack('march', row)}
        onmouseenter={() => enterTrack('march', row)}
        onmouseleave={leaveTrack}
        onfocus={() => enterTrack('march', row)}
        onblur={leaveTrack}
        aria-label={`${PLAYER_NAMES[far]} ${TOKEN_NAMES.march} token to ${lineName('march', row)}`}
      >
        {#if trackHasToken(far, 'march', row)}
          <span class={`token ${far === 0 ? 'ivory' : 'umber'}`}>
            <TokenIcon kind="march" side={tokenOf(game, far, 'march').side} owner={far} />
          </span>
        {:else}
          <span class="slot"></span>
        {/if}
      </button>

      {#each cols as col (col)}
        {@const square = { row, col }}
        {@const key = squareKey(square)}
        {@const piece = pieceAt(game, square)}
        {@const slot = castleSlotAt(square)}
        {@const outcome = outcomeByDestination.get(key)}
        <button
          class={`square ${bandClass(square)}`}
          class:castle-square={slot !== null && game.castles[slot.owner][slot.index]}
          class:castle-destroyed={slot !== null && !game.castles[slot.owner][slot.index]}
          class:hilltop={isHilltop(game, square)}
          class:armed-siege={piece !== null &&
            piece.kind === 'trebuchet' &&
            isHilltop(game, square)}
          class:line-preview={onPreviewedLine(square)}
          class:preview-mover={previewMovers.has(key)}
          class:selected={selected !== null && sameSquare(selected, square)}
          class:firing={attacker !== null && sameSquare(attacker, square)}
          class:queued={queuedAttackers.has(key)}
          class:target={targets.has(key)}
          class:movable={movable.has(key)}
          class:arming={armingMovers.has(key)}
          class:destination={destinations.has(key)}
          class:aims={showOutcomes && outcome !== undefined && targeting(outcome)}
          class:pattern={zoneUnderCursor.has(key)}
          class:zone-own={ownAttack.zone.has(key)}
          class:zone-threat={enemyAttack.zone.has(key)}
          class:own-mark={ownAttack.marks.has(key)}
          class:threat={enemyAttack.marks.has(key)}
          class:in-reach={inReach.has(key)}
          class:flash={destroyedKeys.has(key)}
          onclick={() => clickSquare(square)}
          onmouseenter={() => enterSquare(square)}
          onmouseleave={leaveSquare}
          onfocus={() => enterSquare(square)}
          onblur={leaveSquare}
          title={squareTitle(square)}
        >
          <span class="terrain">
            {#if slot && game.castles[slot.owner][slot.index]}
              <span class="castle"><TerrainIcon kind="castle" /></span>
            {:else if isHilltop(game, square)}
              <span class="hill"><TerrainIcon kind="hilltop" /></span>
            {/if}
          </span>

          {#if piece}
            <span class={`token piece ${pieceClass(piece)}`}>
              <PieceIcon kind={piece.kind} owner={piece.owner} />
            </span>
          {/if}

          <span class="overlay"></span>
        </button>
      {/each}

      <button
        class="track march"
        class:live={trackActive(near, 'march', row)}
        class:occupied={trackHasToken(near, 'march', row)}
        disabled={!trackActive(near, 'march', row)}
        onclick={() => clickTrack('march', row)}
        onmouseenter={() => enterTrack('march', row)}
        onmouseleave={leaveTrack}
        onfocus={() => enterTrack('march', row)}
        onblur={leaveTrack}
        aria-label={`${PLAYER_NAMES[near]} ${TOKEN_NAMES.march} token to ${lineName('march', row)}`}
      >
        {#if trackHasToken(near, 'march', row)}
          <span class={`token ${near === 0 ? 'ivory' : 'umber'}`}>
            <TokenIcon kind="march" side={tokenOf(game, near, 'march').side} owner={near} />
          </span>
        {:else}
          <span class="slot"></span>
        {/if}
      </button>
    {/each}

    <div class="corner"></div>
    {#each cols as col (col)}
      <button
        class="track flank"
        class:live={trackActive(near, 'flank', col)}
        class:occupied={trackHasToken(near, 'flank', col)}
        disabled={!trackActive(near, 'flank', col)}
        onclick={() => clickTrack('flank', col)}
        onmouseenter={() => enterTrack('flank', col)}
        onmouseleave={leaveTrack}
        onfocus={() => enterTrack('flank', col)}
        onblur={leaveTrack}
        aria-label={`${PLAYER_NAMES[near]} ${TOKEN_NAMES.flank} token to ${lineName('flank', col)}`}
      >
        {#if trackHasToken(near, 'flank', col)}
          <span class={`token ${near === 0 ? 'ivory' : 'umber'}`}>
            <TokenIcon kind="flank" side={tokenOf(game, near, 'flank').side} owner={near} />
          </span>
        {:else}
          <span class="slot"></span>
        {/if}
      </button>
    {/each}
    <div class="corner"></div>
  </div>
</div>

<style>
  .board-frame {
    --pad: 0.4rem;
    /* Nine columns plus a token track each side, at 0.72 of a cell: the grid is
       this many cells wide, and the cell is the width divided by it. */
    --spans: 10.44;
    /* The board answers to the space it is actually in, not to the window:
       `cqw` is measured against the containing column, so the same rule fills
       a phone edge to edge and leaves the desktop sidebar alone. Height still
       has a say, for a window that is wide and short. */
    --cell: clamp(
      26px,
      min(9vh, calc((100cqw - 2 * var(--pad)) / var(--spans))),
      68px
    );
    --track: calc(var(--cell) * 0.72);
    display: inline-block;
    padding: var(--pad);
  }

  .grid {
    display: grid;
    grid-template-columns: var(--track) repeat(9, var(--cell)) var(--track);
    grid-template-rows: var(--track) repeat(7, var(--cell)) var(--track);
  }

  .corner {
    width: var(--track);
    height: var(--track);
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

  .track:hover:not(:disabled) {
    background: none;
    transform: none;
  }

  .slot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ink-faint);
    opacity: 0.5;
  }

  /* A token never slides to the line it already holds, so its own track is
     always disabled. The art still has to read at full strength. */
  .track:disabled {
    opacity: 1;
  }

  .track.live {
    cursor: pointer;
  }

  .track.live .slot {
    width: calc(var(--track) * 0.6);
    height: calc(var(--track) * 0.6);
    background: transparent;
    border: 2px dashed var(--gold);
    opacity: 0.85;
  }

  .track.live:hover .slot {
    background: var(--gold-soft);
    border-style: solid;
  }

  .token {
    display: grid;
    place-items: center;
    width: 88%;
    aspect-ratio: 1;
    border-radius: 50%;
    box-shadow: var(--shadow-soft);
  }

  /* The art is a disc of the owner's colour, edge to edge. The matching
     background only guards the seam where the CSS circle meets the drawn one. */
  .token.ivory {
    background: var(--ivory);
  }

  .token.umber {
    background: var(--umber);
  }

  /* The rules sheet draws a rock straight onto its square, with no disc. */
  .token.rock {
    box-shadow: none;
  }

  /* The track cells are oblong, so a percentage would size the token off the
     long side and spill it onto the board. */
  .track .token {
    width: calc(var(--track) * 0.84);
  }

  .square {
    position: relative;
    display: grid;
    place-items: center;
    padding: 0;
    border: 1px solid var(--board-line);
    border-radius: 0;
    background: rgba(255, 253, 246, 0.55);
    cursor: default;
    transition: background 0.12s;
  }

  .square:hover:not(:disabled) {
    transform: none;
    border-color: var(--board-line);
  }

  .square.band-move {
    background: rgba(43, 110, 168, 0.13);
  }

  .square.band-armed {
    background: rgba(185, 143, 46, 0.16);
  }

  .square.band-threat {
    background: rgba(163, 34, 34, 0.13);
  }

  /* Castle squares stay legible under a piece: the terrain tint carries them
     even when the icon is mostly covered. */
  .square.castle-square {
    background: var(--castle-tint);
    box-shadow: inset 0 0 0 2px rgba(122, 90, 51, 0.45);
  }

  .square.hilltop {
    background: var(--hilltop);
  }

  /* The line a hovered slide destination would name. Amber is the colour of
     the thing you are about to pick, as it is for a selected piece. */
  .square.line-preview {
    background: rgba(208, 115, 26, 0.2);
  }

  /* A destroyed castle becomes a hilltop; the hatch keeps what it was legible. */
  .square.castle-destroyed {
    background:
      repeating-linear-gradient(45deg, rgba(122, 90, 51, 0.16) 0 4px, transparent 4px 8px),
      var(--hilltop);
  }

  .terrain {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    pointer-events: none;
  }

  /* Terrain is drawn as a share of the cell: a fixed size spills off the
     square once the board shrinks to a phone. */
  .castle,
  .hill {
    display: block;
    aspect-ratio: 1;
  }

  .castle {
    width: 59%;
    color: var(--ink);
    opacity: 0.55;
  }

  .hill {
    width: 50%;
    color: var(--ink-soft);
    opacity: 0.6;
  }

  .piece {
    position: relative;
    z-index: 2;
    width: 82%;
  }

  .square.armed-siege .piece {
    box-shadow:
      0 0 0 3px var(--gold),
      0 0 10px rgba(185, 143, 46, 0.7);
  }

  .overlay {
    position: absolute;
    inset: 0;
    z-index: 3;
    pointer-events: none;
  }

  /* An Attack Zone, whether or not anything is standing in it. Gold is yours,
     crimson theirs; a square in both hatches is a trade. */
  .square.zone-own .overlay {
    background-image: repeating-linear-gradient(
      -45deg,
      rgba(185, 143, 46, 0.28) 0 2px,
      transparent 2px 7px
    );
  }

  .square.zone-threat .overlay {
    background-image: repeating-linear-gradient(
      45deg,
      rgba(163, 34, 34, 0.26) 0 2px,
      transparent 2px 7px
    );
  }

  .square.zone-own.zone-threat .overlay {
    background-image:
      repeating-linear-gradient(-45deg, rgba(185, 143, 46, 0.28) 0 2px, transparent 2px 7px),
      repeating-linear-gradient(45deg, rgba(163, 34, 34, 0.26) 0 2px, transparent 2px 7px);
  }

  /* A piece this activation could move. */
  .square.movable {
    cursor: pointer;
  }

  /* Pieces on the line a hovered slide would name. */
  .square.preview-mover .overlay {
    box-shadow: inset 0 0 0 2px var(--gold);
  }

  .square.movable .overlay {
    box-shadow: inset 0 0 0 2px var(--gold);
  }

  .square.selected .overlay {
    box-shadow: inset 0 0 0 3px var(--amber);
    background-color: rgba(208, 115, 26, 0.12);
  }

  /* A piece holding a move that would put something under the Attack. */
  .square.arming .overlay::after {
    content: '';
    position: absolute;
    top: 6%;
    right: 6%;
    width: 22%;
    height: 22%;
    border-radius: 50%;
    background: var(--crimson);
    opacity: 0.75;
  }

  /* The attacker now choosing its one target. */
  .square.firing .overlay {
    box-shadow: inset 0 0 0 3px var(--amber);
    background-color: rgba(208, 115, 26, 0.12);
  }

  /* Attackers behind it: this Attack still has their targets to choose. */
  .square.queued .overlay {
    box-shadow: inset 0 0 0 2px rgba(208, 115, 26, 0.45);
  }

  .square.target {
    cursor: pointer;
  }

  .square.target .overlay {
    box-shadow: inset 0 0 0 3px var(--crimson);
    background-color: rgba(163, 34, 34, 0.22);
  }

  .square.target:hover .overlay {
    background-color: rgba(163, 34, 34, 0.38);
  }

  .square.destination {
    cursor: pointer;
  }

  .square.destination .overlay::after {
    content: '';
    position: absolute;
    inset: 0;
    margin: auto;
    width: 28%;
    height: 28%;
    border-radius: 50%;
    background: var(--azure);
    opacity: 0.55;
  }

  /* A destination that would bring something under the Attack. */
  .square.destination.aims .overlay::after {
    width: 38%;
    height: 38%;
    background: var(--crimson);
    opacity: 0.7;
    box-shadow: 0 0 0 3px rgba(163, 34, 34, 0.25);
  }

  /* The Attack Zone of the piece under the cursor. */
  .square.pattern .overlay {
    box-shadow: inset 0 0 0 2px rgba(163, 34, 34, 0.45);
  }

  /* Something your armed token bears on. */
  .square.own-mark .overlay::before {
    content: '';
    position: absolute;
    inset: 6%;
    border: 2px solid var(--gold);
    border-radius: 3px;
    background: rgba(185, 143, 46, 0.2);
  }

  /* Something the opponent's armed token bears on. */
  .square.threat .overlay::before {
    content: '';
    position: absolute;
    inset: 6%;
    border: 2px solid var(--crimson);
    border-radius: 3px;
    background: rgba(163, 34, 34, 0.22);
  }

  /* What the destination under the cursor would put in reach. Last, so it
     wins over the softer marks it sits on top of. */
  .square.in-reach .overlay {
    box-shadow: inset 0 0 0 3px var(--crimson);
  }

  .square.in-reach .overlay::before {
    content: '';
    position: absolute;
    inset: 6%;
    border: 2px solid var(--crimson);
    border-radius: 3px;
    background: rgba(163, 34, 34, 0.35);
  }

  .square.flash .overlay {
    animation: strike 0.7s ease-out;
  }

  @keyframes strike {
    0% {
      background-color: rgba(163, 34, 34, 0.75);
    }
    100% {
      background-color: transparent;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .square.flash .overlay {
      animation: none;
      background-color: rgba(163, 34, 34, 0.25);
    }
  }
</style>
