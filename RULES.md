# Assedio

A two-player siege game on a board nine columns across and nine rows deep.
Each side commands ten pieces, two command tokens and three castles, and four
neutral rocks stand between them.

## The board

As the rules sheet draws it: Umber across the top, Ivory along the bottom.
Rows count up from Ivory's back row, and columns run a to i from Ivory's left.

```
            a    b    c    d    e    f    g    h    i
                                [F]                         Umber's Flank token
         ┌────┬────┬────┬────┬────┬────┬────┬────┬────┐
       9 │    │    │ Sp │ Ar │ Tr │ Ar │ Sp │    │    │      Umber back row
         ├────┼────┼────┼────┼────┼────┼────┼────┼────┤
       8 │    │ ▣C │ Sw │ Fl │ ▣C │ Fl │ Sw │ ▣C │    │      Umber castle row
         ├────┼────┼────┼────┼────┼────┼────┼────┼────┤      (the ram starts on
       7 │    │    │    │    │    │    │    │    │    │       the middle castle)
         ├────┼────┼────┼────┼────┼────┼────┼────┼────┤
       6 │    │    │    │    │    │    │    │    │    │
         ├────┼────┼────┼────┼────┼────┼────┼────┼────┤
  [A]  5 │    │ ▲H │ Rk │ Rk │ ▲H │ Rk │ Rk │ ▲H │    │ [A]  hilltops and rocks
         ├────┼────┼────┼────┼────┼────┼────┼────┼────┤
       4 │    │    │    │    │    │    │    │    │    │
         ├────┼────┼────┼────┼────┼────┼────┼────┼────┤
       3 │    │    │    │    │    │    │    │    │    │
         ├────┼────┼────┼────┼────┼────┼────┼────┼────┤
       2 │    │ ▣C │ Sw │ Fl │ ▣C │ Fl │ Sw │ ▣C │    │      Ivory castle row
         ├────┼────┼────┼────┼────┼────┼────┼────┼────┤
       1 │    │    │ Sp │ Ar │ Tr │ Ar │ Sp │    │    │      Ivory back row
         └────┴────┴────┴────┴────┴────┴────┴────┴────┘
                                [F]                         Ivory's Flank token
```

`Sp` spearman · `Ar` archer · `Tr` trebuchet · `Sw` swordsman · `Fl` flail ·
`Rk` rock. The battering ram (not shown) stands on each side's middle castle at
e2 and e8. `[A]` is a March token Attack side up — Umber's on the left edge,
Ivory's on the right — and `[F]` a Flank token Move side up. The outer columns,
a and i, carry no terrain and start empty: they are the room the armies have to
work around the rocks. The outer rows, 1 and 9, lie behind the castles, and each
army's back row starts there, three rows of open ground from the middle.

**Castles** are terrain, not pieces. They never move and never attack, and only
a siege engine can destroy one. You may stand a piece on your own castle — the
battering ram opens the game doing exactly that — but you cannot move onto an enemy
castle space, nor through one. **If a castle is destroyed, it becomes a
hilltop**, ordinary high ground either side may occupy.

**Hilltops** are the three middle-row squares b5, e5 and h5, plus every castle
that has been destroyed. Pieces can move onto hilltops. They mark where
trebuchets can attack from: castles and hilltops share columns, which puts every
castle exactly three squares from a hilltop.

**Rocks** are neutral pieces on the four middle-row squares between the
hilltops — c5, d5, f5 and g5 — and can be killed by your attacks. Like any
piece, a rock stops a move: nothing may enter its square or pass through it.
Siege engines only ever destroy castles, so it is the rest of an army that
kills a rock, and the square is open ground thereafter. Each hilltop is walled
in by rocks on the middle row, so the road to the high ground runs either around
them or through the infantry that clears them.

Ivory moves first.

## Objective

Win immediately by accomplishing either:

* Destroy all 3 enemy castles.
* Destroy both enemy siege engines.

The second condition follows from the first: only siege engines destroy
castles, so a side with neither can no longer win, and the game is over rather
than drawn out.

If neither side has won after 300 turns the game is a draw. Nothing in the
rules forces progress; the cap exists so that every line terminates.

## Command tokens

Each side has two command tokens, and each token has a Move side and an Attack
side.

| Token | Rides | Move side | Attack side |
|---|---|---|---|
| March | the left or right edge, naming a **row** | **March**: move 1 piece in the row vertically or horizontally any number of spaces | attack with any number of your pieces in the row |
| Flank | the top or bottom edge, naming a **column** | **Flank**: move 1 piece in the column diagonally any number of spaces | attack with any number of your pieces in the column |

Each side's Flank token rides the edge beside its own back row, and its March
token the edge to its right. Both sides start with the March token Attack side
up on the middle row, row 5, and the Flank token Move side up on the middle
column, column e.

## Your turn

1. Slide the command token that is on its Move side to a new row or column.
2. Activate one of your command tokens, then flip it over.
3. Activate your other command token, then flip it over.

The first turn is played like every other. Because the tokens start on opposite
sides and every turn flips both, exactly one token shows its Move side at the
start of every turn, and the two take turns to be slid: the Flank token on a
side's first turn, the March token on its second, and so on. **The token you
slid last turn is the one that attacks this turn, from the line you put it
on**: every Attack is announced a full turn before it is made, and the standard
combination is to move a piece into a line you armed on the previous turn.

A slide may go to any other line, whether or not it holds a piece of yours.
Arming an empty line now and moving a piece onto it next turn is as good a plan
as any.

### March and Flank

Choose one of your pieces on the named line and move it. The token decides which
line the piece comes *from*; the move itself may carry it anywhere it can go
vertically and horizontally, or diagonally, including out of that line. Moves
never capture and never jump — a piece stops before any occupied square, friend,
enemy or rock.

You may also flip the token and take no action: a move is never forced, and if
no piece on the line has a legal move the activation passes anyway.

### Attack

**Pieces do not move when attacking.** When an Attack is issued to a row or
column, each of your pieces in it may kill one piece in its Attack Zone. Work
along the line: every piece with something in its zone picks one target, or
holds its fire. Targets are taken as they are chosen, so a piece further down
the line cannot spend itself on one an earlier attacker has already killed.
Nothing blocks an attack — a spearman reaches over the square in front of it.

Ordinary pieces kill enemy pieces and rocks, and never damage a castle. Siege
engines destroy enemy castles and never touch a piece. Your own pieces are never
hit.

## Attack Zones

Relative to the piece, `X` marks its Attack Zone.

```
   swordsman        flail          spearman         archer
   . . X . .      . X . X .        . . X . .      X . . . X
   . X ■ X .      . . ■ . .        . . . . .      . . . . .
   . . X . .      . X . X .        X . ■ . X      . . ■ . .
                                   . . . . .      . . . . .
                                   . . X . .      X . . . X
```

* **Swordsman** — the four squares orthogonally adjacent.
* **Flail** — the four squares diagonally adjacent.
* **Spearman** — two squares away, orthogonally.
* **Archer** — two squares away, diagonally.
* **Trebuchet** — destroys a castle that is 3 spaces away, vertically,
  horizontally or diagonally.
* **Battering Ram** — destroys a castle that is 1 space away vertically or
  horizontally.

The hilltops need no rule of their own to be where a trebuchet attacks from.
Every square three spaces from a castle is a hilltop or another castle, and a
trebuchet can only stand on an enemy castle once it has been destroyed — when it
has become a hilltop.

From the centre hilltop a trebuchet bears on *every* castle on the board — and
brings down one of them a turn, since a piece takes a single target. That makes
e5 the sharpest square in the game and the trebuchet's approach the thing both
sides are really playing around — and the rocks on d5 and f5 mean no March along
row 5 reaches it until one of them is killed. A castle that falls hands back high
ground: destroy Umber's outer castle on b8 and a trebuchet standing there bears
on e8, three squares along the castle row.

## Interpretations

The rules sheet leaves a few things unstated. Where a reading had to be chosen,
this is the one the engine implements:

* **The opening tokens come from the board diagram.** The sheet's text never
  says where the tokens start; its board shows the March tokens Attack side up
  beside the middle row and the Flank tokens Move side up beside the middle
  column. That is also the only kind of opening in which "the token on its Move
  side" names a single token from the very first turn.
* **Castles are terrain.** The sheet says three castles and its board draws two
  per side — but the middle square between them is shaded like the outer two.
  Castles therefore stand in columns b, e and h, one row in from each side's
  edge, and the battering ram starts standing on the middle one.
* **An enemy castle blocks a move entirely.** "You cannot move onto an enemy
  Castle space" is read as an obstacle rather than a forbidden destination: a
  move stops before it, the way it stops before a piece.
* **Moves never capture.** "Pieces do not move when attacking" is read as the
  converse too: moving and attacking are separate, and a piece may not be taken
  by being moved onto.
* **Moving is optional.** The sheet does not say whether a Move side may be
  flipped without moving; an earlier edition said it may, and so it may.
* **The trebuchet's range is exactly three**, not up to three.
* **Targets are taken as they are chosen.** The attackers on a line act in
  order along it rather than all at once. The set of outcomes is the same either
  way — doubling two attackers onto one victim only ever wastes an attack — but
  resolving in order means a later attacker is never offered a target that is
  already gone.
* **Your own pieces are never hit.** "It may kill one piece that is in its
  Attack Zone" does not say an enemy piece, but the separate rule that rocks can
  be killed would say nothing if any piece could be.
* **Ivory moves first.** The sheet does not say who starts.
