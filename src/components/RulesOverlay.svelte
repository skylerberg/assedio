<script lang="ts">
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

  function inZone(kind: PieceKind, drow: number, dcol: number): boolean {
    return attackZone(kind).some(([r, c]) => r === drow && c === dcol)
  }
</script>

<div
  class="scrim"
  role="button"
  tabindex="-1"
  onclick={onclose}
  onkeydown={(event) => event.key === 'Escape' && onclose()}
>
  <div
    class="panel sheet"
    role="dialog"
    aria-label="Rules"
    tabindex="-1"
    onclick={(event) => event.stopPropagation()}
    onkeydown={() => {}}
  >
    <header>
      <h2>Assedio</h2>
      <button onclick={onclose}>Close</button>
    </header>

    <section>
      <h3>Objective</h3>
      <p>Win immediately by accomplishing either:</p>
      <ul>
        <li>Destroy all 3 enemy castles.</li>
        <li>Destroy both enemy siege engines.</li>
      </ul>
      <p class="aside">
        Only siege engines destroy castles, so a side that has lost both can never win — which is
        why that ends it.
      </p>
    </section>

    <section>
      <h3>Your turn</h3>
      <p>
        You have two command tokens. The March token rides a row and the Flank token a column, and
        each has a Move side and an Attack side.
      </p>
      <ol class="steps">
        <li>Slide the command token that is on its Move side to a new row or column.</li>
        <li>Activate one of your command tokens, then flip it over.</li>
        <li>Activate your other command token, then flip it over.</li>
      </ol>
      <p class="aside">
        Both sides open with the March token Attack side up on the middle row and the Flank token
        Move side up on the middle column. So the token you slid last turn is the one that attacks
        this turn, from the line you put it on: every Attack is announced a full turn before it is
        made.
      </p>
    </section>

    <section>
      <h3>The sides</h3>
      <div class="sides">
        <div>
          <span><TokenIcon kind="march" side="move" size={26} /></span>
          <span
            ><strong>March</strong> — move 1 piece in the row vertically or horizontally any number of
            spaces.</span
          >
        </div>
        <div>
          <span><TokenIcon kind="flank" side="move" size={26} /></span>
          <span
            ><strong>Flank</strong> — move 1 piece in the column diagonally any number of spaces.</span
          >
        </div>
        <div>
          <span><TokenIcon kind="march" side="attack" size={26} /></span>
          <span
            ><strong>Attack</strong> — attack with any number of your pieces in the row or column.</span
          >
        </div>
      </div>
      <p class="aside">
        The token picks which line a piece comes <em>from</em>; the move may carry it off that
        line. Moves never capture and never jump. You may also flip a Move side and take no action.
      </p>
    </section>

    <section>
      <h3>Attacks</h3>
      <p>
        <strong>Pieces do not move when attacking.</strong> When an Attack is issued to a row or
        column a piece is in, it may kill one piece in its Attack Zone. Nothing blocks an attack: a
        spearman reaches over the square in front of it.
      </p>
      <div class="patterns">
        {#each FIGHTERS as kind (kind)}
          <figure>
            <div class="mini">
              {#each RANGE as drow (drow)}
                {#each RANGE as dcol (dcol)}
                  <span
                    class="cell"
                    class:hit={inZone(kind, drow, dcol)}
                    class:self={drow === 0 && dcol === 0}
                  >
                    {#if drow === 0 && dcol === 0}
                      <PieceIcon {kind} size={20} />
                    {/if}
                  </span>
                {/each}
              {/each}
            </div>
            <figcaption>{PIECE_NAMES[kind]}</figcaption>
          </figure>
        {/each}
      </div>
      <ul class="siege">
        <li>
          <span class="engine"><PieceIcon kind="trebuchet" size={20} /></span>
          <span>
            <strong>Trebuchet</strong> — destroys a castle that is 3 spaces away, vertically,
            horizontally or diagonally. From the centre hilltop it bears on every castle on the
            board, and brings one of them down a turn.
          </span>
        </li>
        <li>
          <span class="engine"><PieceIcon kind="batteringRam" size={20} /></span>
          <span>
            <strong>Battering Ram</strong> — destroys a castle that is 1 space away vertically or
            horizontally.
          </span>
        </li>
      </ul>
    </section>

    <section>
      <h3>The ground</h3>
      <ul class="siege">
        <li>
          <span class="engine"><TerrainIcon kind="castle" size={20} /></span>
          <span>
            <strong>Castles</strong> — you cannot move onto an enemy castle, nor through one. If a
            castle is destroyed, it becomes a hilltop.
          </span>
        </li>
        <li>
          <span class="engine"><TerrainIcon kind="hilltop" size={20} /></span>
          <span>
            <strong>Hilltops</strong> — pieces can move onto hilltops. They mark where trebuchets
            can attack from.
          </span>
        </li>
        <li>
          <span class="engine"><PieceIcon kind="rock" size={20} /></span>
          <span>
            <strong>Rocks</strong> — neutral pieces that can be killed by your attacks. Like any
            piece, a rock blocks a move; siege engines only ever destroy castles, so it falls to
            the rest of your army.
          </span>
        </li>
      </ul>
    </section>

    <section>
      <h3>Reading the board</h3>
      <ul class="marks">
        <li><i class="key own"></i> a piece or castle your Attack can take</li>
        <li><i class="key own-reach"></i> your Attack Zone, occupied or not</li>
        <li><i class="key threat"></i> what the opponent's Attack can take</li>
        <li><i class="key threat-reach"></i> their Attack Zone — move into it and they can attack you</li>
        <li><i class="key movable"></i> a piece this activation may move</li>
        <li><i class="key dest"></i> somewhere it may go</li>
        <li><i class="key kill"></i> a destination that would put a target in your Attack Zone</li>
      </ul>
      <p class="aside">
        Each attacker takes one target, so a line reaching four things still only takes as many as
        it has pieces to attack with. Hover a slide target or a destination and the line under the
        board says what it leads to; both sets of hints have a switch in the top bar. Decisions
        with only one answer worth making — a forced pass, an activation order that leads to the
        same board either way or would attack nothing — are taken for you and recorded in the
        dispatches.
      </p>
    </section>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 20;
    background: rgba(51, 36, 15, 0.5);
    display: grid;
    place-items: center;
    padding: 1rem;
    border: none;
  }

  .sheet {
    max-width: 46rem;
    max-height: 90vh;
    overflow-y: auto;
    padding: 1.2rem 1.4rem 1.6rem;
    display: grid;
    gap: 1.1rem;
    text-align: left;
    cursor: default;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    position: sticky;
    top: -1.2rem;
    background: var(--panel);
    padding: 0.4rem 0;
    margin: -0.4rem 0;
  }

  h2 {
    font-size: 1.5rem;
  }

  .marks {
    display: grid;
    gap: 0.35rem;
    list-style: none;
    padding: 0;
  }

  .marks li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .key {
    width: 14px;
    height: 14px;
    border-radius: 2px;
    flex: none;
  }

  .key.own {
    border: 2px solid var(--gold);
    background: rgba(185, 143, 46, 0.2);
  }

  .key.own-reach {
    border: 1px solid var(--gold-soft);
    background: repeating-linear-gradient(
      -45deg,
      rgba(185, 143, 46, 0.55) 0 2px,
      transparent 2px 6px
    );
  }

  .key.threat {
    border: 2px solid var(--crimson);
    background: rgba(163, 34, 34, 0.22);
  }

  .key.threat-reach {
    border: 1px solid rgba(163, 34, 34, 0.4);
    background: repeating-linear-gradient(
      45deg,
      rgba(163, 34, 34, 0.5) 0 2px,
      transparent 2px 6px
    );
  }

  .key.movable {
    box-shadow: inset 0 0 0 2px var(--gold);
  }

  .key.dest {
    border-radius: 50%;
    background: var(--azure);
    opacity: 0.55;
  }

  .key.kill {
    border-radius: 50%;
    background: var(--crimson);
    opacity: 0.7;
  }

  h3 {
    font-size: 1rem;
    color: var(--ink-soft);
    margin-bottom: 0.3rem;
  }

  ul,
  ol {
    padding-left: 1.1rem;
    display: grid;
    gap: 0.2rem;
  }

  p + ul,
  p + ol,
  ul + p,
  ol + p {
    margin-top: 0.4rem;
  }

  .aside {
    color: var(--ink-soft);
    font-style: italic;
    border-left: 2px solid var(--gold-soft);
    padding-left: 0.7rem;
  }

  .sides {
    display: grid;
    gap: 0.5rem;
  }

  .sides > div {
    display: grid;
    grid-template-columns: 2rem 1fr;
    align-items: center;
    gap: 0.6rem;
  }

  .patterns {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
  }

  figure {
    display: grid;
    gap: 0.3rem;
    justify-items: center;
  }

  .mini {
    display: grid;
    grid-template-columns: repeat(5, 24px);
    grid-template-rows: repeat(5, 24px);
  }

  .cell {
    border: 1px solid var(--board-line);
    display: grid;
    place-items: center;
  }

  .cell.hit {
    background: rgba(163, 34, 34, 0.75);
  }

  .cell.self {
    background: var(--castle-tint);
  }

  figcaption {
    font-family: var(--font-display);
    font-size: 0.75rem;
    letter-spacing: 0.06em;
    color: var(--ink-soft);
  }

  .siege {
    list-style: none;
    padding: 0;
    gap: 0.5rem;
    margin-top: 0.7rem;
  }

  .siege li {
    display: grid;
    grid-template-columns: 1.6rem 1fr;
    gap: 0.6rem;
    align-items: start;
  }
</style>
