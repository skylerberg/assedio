<script lang="ts">
  import PieceIcon from './PieceIcon.svelte'
  import TerrainIcon from './TerrainIcon.svelte'
  import TokenIcon from './TokenIcon.svelte'
  import { lineName } from '../data/log'
  import { CASTLE_COLS, PLAYER_NAMES, TOKEN_KINDS, TOKEN_NAMES, siegeEngines, tokenOf } from '../data/types'
  import type { GameState, PieceKind } from '../data/types'

  interface Props {
    game: GameState
    player: number
    seat: string
    /** Whether this side is to move. */
    active: boolean
    /** A word on what the side is doing, shown beside the seat. */
    status?: string
  }

  let { game, player, seat, active, status }: Props = $props()

  let engines = $derived(siegeEngines(game, player))
  const SIEGE: PieceKind[] = ['trebuchet', 'batteringRam']
  const SIEGE_NAMES: Record<string, string> = { trebuchet: 'Trebuchet', batteringRam: 'Battering Ram' }
</script>

<section class="panel player" class:active aria-label={`${PLAYER_NAMES[player]}, ${seat}`}>
  <header class="head">
    <span class={`disc side-${player}`}></span>
    <span class="name">{PLAYER_NAMES[player]}</span>
    <span class="seat">{seat}{status ? ` · ${status}` : ''}</span>
  </header>

  <div class="body">
    <div class="row">
      <span class="small-caps">Castles</span>
      <span class="icons">
        {#each CASTLE_COLS as _, index (index)}
          {@const standing = game.castles[player][index]}
          <span
            class="castle"
            class:gone={!standing}
            title={standing ? 'Standing' : 'Destroyed — now a hilltop'}
          >
            <TerrainIcon kind={standing ? 'castle' : 'hilltop'} />
          </span>
        {/each}
      </span>
    </div>

    <div class="row">
      <span class="small-caps">Siege</span>
      <span class="icons">
        {#each SIEGE as kind (kind)}
          {@const alive = engines.includes(kind)}
          <span class="engine" class:gone={!alive} title={`${SIEGE_NAMES[kind]}${alive ? '' : ' — destroyed'}`}>
            <PieceIcon {kind} owner={player} />
          </span>
        {/each}
      </span>
    </div>

    <div class="tokens">
      {#each TOKEN_KINDS as kind (kind)}
        {@const token = tokenOf(game, player, kind)}
        <span class="chip" class:armed={token.side === 'attack'}>
          <span class="chip-icon"><TokenIcon {kind} side={token.side} owner={player} /></span>
          <span>
            <strong>{TOKEN_NAMES[kind]}</strong>
            {token.side === 'attack' ? 'attacks' : 'moves'} · {lineName(kind, token.line)}
          </span>
        </span>
      {/each}
    </div>
  </div>
</section>

<style>
  .player {
    container-type: inline-size;
    transition: box-shadow 0.2s;
  }

  .player.active {
    box-shadow:
      0 0 0 3px var(--peach),
      0 0 0 5px var(--copper),
      var(--shadow-soft);
  }

  .head {
    display: flex !important;
    align-items: center;
    gap: 0.45rem;
    text-align: left !important;
  }

  .name {
    flex: 1;
  }

  .seat {
    font-size: 0.78rem;
    font-weight: 600;
    font-style: italic;
    opacity: 0.85;
  }

  .disc {
    width: 0.85rem;
    height: 0.85rem;
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

  .body {
    display: grid;
    gap: 0.4rem;
    padding: 0.55rem 0.75rem 0.65rem;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .icons {
    display: flex;
    gap: 0.35rem;
  }

  .castle {
    width: 1.7rem;
    height: 1.55rem;
  }

  .engine {
    width: 1.55rem;
    height: 1.55rem;
  }

  .gone {
    opacity: 0.35;
    filter: grayscale(0.7);
  }

  .tokens {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.1rem 0.5rem 0.1rem 0.15rem;
    border-radius: 999px;
    border: 1.5px solid var(--copper-soft);
    font-size: 0.8rem;
    font-weight: 500;
    color: var(--ink-soft);
    line-height: 1.2;
  }

  .chip strong {
    color: var(--ink);
  }

  .chip.armed {
    border-color: var(--attack);
    background: rgba(227, 38, 43, 0.07);
  }

  .chip-icon {
    width: 1.35rem;
    height: 1.35rem;
    flex: none;
  }

  /* Beside the board on a phone the panel is a narrow strip: the labels go
     and the icons carry it. */
  @container (max-width: 15rem) {
    .small-caps {
      display: none;
    }

    .row {
      justify-content: flex-start;
    }

    .chip {
      font-size: 0.72rem;
    }
  }
</style>
