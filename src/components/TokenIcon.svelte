<script lang="ts">
  import type { TokenKind, TokenSide } from '../data/types'

  import attackIvory from '../assets/tokens/attack-ivory.png'
  import attackUmber from '../assets/tokens/attack-umber.png'
  import flankIvory from '../assets/tokens/flank-ivory.png'
  import flankUmber from '../assets/tokens/flank-umber.png'
  import marchIvory from '../assets/tokens/march-ivory.png'
  import marchUmber from '../assets/tokens/march-umber.png'

  /** Indexed by owner, so the tuples are [ivory, umber]. Both tokens share one
      Attack side; only the Move sides tell March from Flank. */
  const ART: Record<TokenKind | 'attack', [string, string]> = {
    attack: [attackIvory, attackUmber],
    march: [marchIvory, marchUmber],
    flank: [flankIvory, flankUmber],
  }

  interface Props {
    kind: TokenKind
    side: TokenSide
    /** Which player's colours the art carries. */
    owner?: number
    /** Omit to fill the parent, which is how the board sizes its discs. */
    size?: number
  }

  let { kind, side, owner = 1, size }: Props = $props()
</script>

<img
  class="token-icon"
  src={ART[side === 'attack' ? 'attack' : kind][owner]}
  alt=""
  draggable="false"
  style={size === undefined ? undefined : `width:${size}px;height:${size}px`}
/>

<style>
  .token-icon {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
