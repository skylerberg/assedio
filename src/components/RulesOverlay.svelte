<script lang="ts">
  import Legend from './Legend.svelte'
  import type { LegendEntry } from './Legend.svelte'
  import Modal from './Modal.svelte'
  import PieceIcon from './PieceIcon.svelte'
  import TerrainIcon from './TerrainIcon.svelte'
  import TokenIcon from './TokenIcon.svelte'
  import { PIECE_NAMES, attackZone } from '../data/types'
  import type { PieceKind } from '../data/types'

  interface Props {
    onclose: () => void
  }

  let { onclose }: Props = $props()

  const FIGHTERS: PieceKind[] = ['swordsman', 'flail', 'spearman', 'archer']
  const RANGE = [-2, -1, 0, 1, 2]
  const KEYS: LegendEntry[] = [
    { key: 'slide', tone: 'var(--march)' },
    { key: 'movable', tone: 'var(--march)' },
    { key: 'destination', tone: 'var(--march)' },
    { key: 'aim', tone: 'var(--march)' },
    { key: 'reticle' },
    { key: 'target' },
    { key: 'frame' },
    { key: 'zone' },
    { key: 'attacker' },
    { key: 'last' },
    { key: 'kill' },
  ]

  function inZone(kind: PieceKind, drow: number, dcol: number): boolean {
    return attackZone(kind).some(([r, c]) => r === drow && c === dcol)
  }
</script>

<Modal label="Rules of Assedio" {onclose}>
  <header class="top">
    <h2>Assedio</h2>
    <button data-autofocus onclick={onclose}>Close</button>
  </header>

  <div class="sections">
    <section class="box">
      <h3 class="bar">Objective</h3>
      <div class="inner objective">
        <p><strong>Win immediately by accomplishing either:</strong></p>
        <div class="either">
          <span class="art castle"><TerrainIcon kind="castle" /></span>
          <span>Destroy all 3 enemy castles.</span>
        </div>
        <span class="or">or</span>
        <div class="either">
          <span class="art pair">
            <PieceIcon kind="batteringRam" owner={0} />
            <PieceIcon kind="trebuchet" owner={0} />
          </span>
          <span>Destroy both enemy siege engines.</span>
        </div>
      </div>
    </section>

    <section class="box">
      <h3 class="bar">Your Turn</h3>
      <ol class="inner steps">
        <li>
          <span class="diamond"><span>1</span></span>
          <p>Slide the command token that is on its Move side to a new row or column.</p>
        </li>
        <li>
          <span class="diamond"><span>2</span></span>
          <p>Activate one of your command tokens, then flip it over.</p>
        </li>
        <li>
          <span class="diamond"><span>3</span></span>
          <p>Activate your other command token, then flip it over.</p>
        </li>
      </ol>
      <p class="inner aside">
        Both sides open with the March token Attack side up on the middle row and the Flank token
        Move side up on the middle column. So the token you slid last turn is the one that attacks
        this turn, from the line you put it on: every Attack is announced a full turn before it is
        made. In this game you start an activation by doing it — move a piece, or pick a target —
        and whichever you start first goes first.
      </p>
    </section>

    <section class="box">
      <div class="sides">
        <div>
          <h3 class="bar maroon">March (Move)</h3>
          <div class="inner side">
            <span class="token"><TokenIcon kind="march" side="move" owner={0} /></span>
            <p>Move 1 piece in the row vertically or horizontally any number of spaces.</p>
          </div>
        </div>
        <div>
          <h3 class="bar maroon">Flank (Move)</h3>
          <div class="inner side">
            <span class="token"><TokenIcon kind="flank" side="move" owner={0} /></span>
            <p>Move 1 piece in the column diagonally any number of spaces.</p>
          </div>
        </div>
        <div>
          <h3 class="bar maroon">Attack</h3>
          <div class="inner side">
            <span class="token"><TokenIcon kind="march" side="attack" owner={0} /></span>
            <p>Attack with any number of your pieces in the row or column.</p>
          </div>
        </div>
      </div>
      <p class="inner aside">
        The token picks which line a piece comes <em>from</em>; the move may carry it off that
        line. Moves never capture and never jump. You may flip a Move side and take no action.
      </p>
    </section>

    <section class="box">
      <h3 class="bar">Pieces do not move when attacking!</h3>
      <div class="inner">
        <p>
          When an Attack is issued to a row or column a piece is in, it may kill one piece that is
          in its Attack Zone. Nothing blocks an attack: a spearman reaches over the square in front
          of it.
        </p>
        <div class="patterns">
          {#each FIGHTERS as kind (kind)}
            <figure>
              <div class="mini">
                {#each RANGE as drow (drow)}
                  {#each RANGE as dcol (dcol)}
                    <span class="cell" class:hit={inZone(kind, drow, dcol)}>
                      {#if drow === 0 && dcol === 0}
                        <PieceIcon {kind} owner={0} />
                      {/if}
                    </span>
                  {/each}
                {/each}
              </div>
              <figcaption>{PIECE_NAMES[kind]}</figcaption>
            </figure>
          {/each}
        </div>
      </div>
      <div class="sides two">
        <div>
          <h3 class="bar">Trebuchet</h3>
          <div class="inner side">
            <span class="token"><PieceIcon kind="trebuchet" owner={0} /></span>
            <p>
              Destroy a castle that is 3 spaces away, vertically, horizontally, or diagonally. From
              the centre hilltop it bears on every castle on the board.
            </p>
          </div>
        </div>
        <div>
          <h3 class="bar">Battering Ram</h3>
          <div class="inner side">
            <span class="token"><PieceIcon kind="batteringRam" owner={0} /></span>
            <p>Destroy a castle that is 1 space away vertically or horizontally.</p>
          </div>
        </div>
      </div>
    </section>

    <section class="box">
      <div class="sides">
        <div>
          <h3 class="bar">Castles</h3>
          <div class="inner side">
            <span class="token castle"><TerrainIcon kind="castle" /></span>
            <p>
              You cannot move onto an enemy castle space, nor through one. If a castle is destroyed,
              it becomes a hilltop.
            </p>
          </div>
        </div>
        <div>
          <h3 class="bar">Hilltops</h3>
          <div class="inner side">
            <span class="token"><TerrainIcon kind="hilltop" /></span>
            <p>Pieces can move onto hilltops. They mark where trebuchets can attack from.</p>
          </div>
        </div>
        <div>
          <h3 class="bar">Rocks</h3>
          <div class="inner side">
            <span class="token"><PieceIcon kind="rock" /></span>
            <p>Rocks are neutral pieces that can be killed by your attacks. They block moves.</p>
          </div>
        </div>
      </div>
    </section>

    <section class="box">
      <h3 class="bar">Reading the Board</h3>
      <div class="inner">
        <Legend entries={KEYS} mine="Your" theirs="The opponent's" />
        <p class="aside">
          Moves are marked in the colour of the token making them: cyan for March, orange for
          Flank. Hover a piece to shade its Attack Zone, a threatened square to see who threatens
          it, or a dispatch to light the squares it names. On a touch screen, the first tap on a
          slide or a destination shows where it leads and a second tap commits it. Decisions with
          only one answer worth making are taken for you.
        </p>
      </div>
    </section>
  </div>
</Modal>

<style>
  .top {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.7rem 1.1rem;
    background: var(--navy);
    color: var(--peach);
    border-radius: 11px 11px 0 0;
  }

  .top h2 {
    font-size: 1.6rem;
    font-weight: 900;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .sections {
    display: grid;
    gap: 1rem;
    padding: 1rem 1.1rem 1.3rem;
  }

  .box {
    border: 2px solid var(--copper);
    border-radius: 14px;
    overflow: hidden;
    background: rgba(255, 252, 240, 0.35);
  }

  .bar {
    background: var(--navy);
    color: var(--peach);
    text-align: center;
    font-size: 1.05rem;
    font-weight: 800;
    padding: 0.25rem 0.6rem;
  }

  .bar.maroon {
    background: var(--maroon);
  }

  .inner {
    padding: 0.7rem 0.9rem;
  }

  .aside {
    color: var(--ink-soft);
    font-style: italic;
    font-size: 0.92rem;
    border-top: 1px solid var(--copper-soft);
  }

  .inner > .aside,
  .inner .aside {
    border-top: none;
    margin-top: 0.6rem;
    padding: 0;
  }

  .objective {
    display: grid;
    gap: 0.45rem;
    justify-items: center;
    text-align: center;
  }

  .either {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    font-weight: 700;
  }

  .art {
    display: inline-flex;
    gap: 0.3rem;
  }

  .art.castle {
    width: 3.2rem;
    height: 3rem;
  }

  .art.pair :global(img) {
    width: 2.4rem;
    height: 2.4rem;
  }

  .or {
    font-family: var(--font-display);
    font-weight: 800;
    font-size: 0.8rem;
    text-transform: uppercase;
    color: var(--copper-deep);
  }

  .steps {
    list-style: none;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.8rem;
  }

  .steps li {
    display: grid;
    justify-items: center;
    gap: 0.4rem;
    text-align: center;
    font-weight: 700;
  }

  .diamond {
    display: inline-grid;
    place-items: center;
    width: 1.5rem;
    height: 1.5rem;
    transform: rotate(45deg);
    border: 2px solid var(--copper);
    background: var(--peach);
  }

  .diamond span {
    transform: rotate(-45deg);
    font-weight: 800;
  }

  .sides {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
  }

  .sides.two {
    grid-template-columns: repeat(2, 1fr);
    border-top: 2px solid var(--copper);
  }

  .sides > div + div {
    border-left: 2px solid var(--copper);
  }

  .side {
    display: grid;
    justify-items: center;
    gap: 0.45rem;
    text-align: center;
    font-weight: 600;
  }

  .token {
    width: 2.6rem;
    height: 2.6rem;
  }

  .token.castle {
    width: 3rem;
  }

  .patterns {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 1.1rem;
    margin-top: 0.7rem;
  }

  figure {
    display: grid;
    gap: 0.3rem;
    justify-items: center;
  }

  .mini {
    display: grid;
    grid-template-columns: repeat(5, 1.6rem);
    grid-template-rows: repeat(5, 1.6rem);
    border: 2px solid var(--board-line);
  }

  .cell {
    box-shadow: inset 0 0 0 0.75px var(--board-line);
    display: grid;
    place-items: center;
    padding: 2px;
  }

  .cell.hit {
    background: var(--attack);
  }

  figcaption {
    font-family: var(--font-display);
    font-size: 0.8rem;
    font-weight: 800;
    color: var(--ink-soft);
  }

  @media (max-width: 600px) {
    .steps,
    .sides,
    .sides.two {
      grid-template-columns: 1fr;
    }

    .sides > div + div {
      border-left: none;
      border-top: 2px solid var(--copper);
    }
  }
</style>
