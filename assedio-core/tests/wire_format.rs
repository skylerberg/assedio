//! The browser reads these shapes directly, so they are part of the interface
//! rather than an implementation detail. A rename that slips through here
//! breaks the UI silently at runtime.

use assedio_core::preview::{move_outcomes, slide_outcomes};
use assedio_core::rules::apply_logged;
use assedio_core::types::*;
use assedio_core::{initial_state, settle_state};
use serde_json::{json, Value};

fn field(value: &Value, key: &str) -> Value {
    value
        .get(key)
        .unwrap_or_else(|| panic!("missing {key} in {value}"))
        .clone()
}

#[test]
fn game_state_serializes_with_the_keys_the_ui_reads() {
    let state = initial_state();
    let value: Value = serde_json::to_value(state).unwrap();

    for key in [
        "board",
        "castles",
        "tokens",
        "currentPlayer",
        "phase",
        "pending",
        "pendingLen",
        "attackIndex",
        "turn",
        "outcome",
    ] {
        assert!(value.get(key).is_some(), "GameState is missing {key}");
    }

    assert_eq!(field(&value, "phase"), json!("slide"));
    assert_eq!(field(&value, "outcome"), json!(null));
    // Nine columns to a row, seven rows to the board.
    assert_eq!(field(&value, "board").as_array().unwrap().len(), 7);
    assert_eq!(field(&value, "board")[0].as_array().unwrap().len(), 9);
    assert_eq!(
        field(&value, "board")[0][4],
        json!({ "kind": "trebuchet", "owner": 0 }),
    );
    assert_eq!(
        field(&value, "board")[0][1],
        json!(null),
        "its castle square"
    );
    assert_eq!(field(&value, "board")[0][0], json!(null), "the empty wing");
    assert_eq!(
        field(&value, "board")[3][2],
        json!({ "kind": "rock", "owner": 2 }),
        "a rock is a piece that belongs to neither side",
    );
    assert_eq!(
        field(&value, "castles"),
        json!([[true, true, true], [true, true, true]])
    );
    // Index 0 is the March token, index 1 the Flank token, matching
    // `TokenKind::index`. Both sides open the same way.
    for player in 0..2 {
        assert_eq!(
            field(&value, "tokens")[player],
            json!([
                { "line": 3, "side": "attack" },
                { "line": 4, "side": "move" },
            ]),
        );
    }

    let round_tripped: GameState = serde_json::from_value(value).unwrap();
    assert_eq!(round_tripped, initial_state());
}

#[test]
fn choices_are_a_tagged_union() {
    let cases = [
        (
            Choice::Slide { line: 4 },
            json!({ "type": "slide", "line": 4 }),
        ),
        (
            Choice::Order {
                first: TokenKind::Flank,
            },
            json!({ "type": "order", "first": "flank" }),
        ),
        (
            Choice::Move {
                from: Square::new(1, 2),
                to: Square::new(3, 2),
            },
            json!({
                "type": "move",
                "from": { "row": 1, "col": 2 },
                "to": { "row": 3, "col": 2 },
            }),
        ),
        (Choice::Pass, json!({ "type": "pass" })),
        (
            Choice::Attack {
                from: Square::new(3, 4),
                target: Square::new(6, 1),
            },
            json!({
                "type": "attack",
                "from": { "row": 3, "col": 4 },
                "target": { "row": 6, "col": 1 },
            }),
        ),
        (
            Choice::HoldFire {
                from: Square::new(3, 4),
            },
            json!({ "type": "holdFire", "from": { "row": 3, "col": 4 } }),
        ),
    ];

    for (choice, expected) in cases {
        assert_eq!(serde_json::to_value(choice).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<Choice>(expected.clone()).unwrap(),
            choice
        );
    }
}

#[test]
fn events_use_camel_case_field_names() {
    let mut state = initial_state();
    let events = apply_logged(&mut state, &Choice::Slide { line: 1 });
    assert_eq!(
        serde_json::to_value(&events[0]).unwrap(),
        json!({ "type": "slid", "player": 0, "token": "flank", "from": 4, "to": 1 }),
    );

    // An Attack is announced once, and every attacker in it is its own event.
    // This row carries one of each casualty.
    let mut state = initial_state();
    state.board = [[None; BOARD_COLS]; BOARD_ROWS];
    for (row, col, kind, owner) in [
        (3, 2, PieceKind::Rock, NEUTRAL),
        (3, 6, PieceKind::Rock, NEUTRAL),
        (3, 1, PieceKind::Swordsman, 0),
        (3, 4, PieceKind::Trebuchet, 0),
        (3, 7, PieceKind::Swordsman, 0),
        (0, 3, PieceKind::BatteringRam, 0),
        (2, 1, PieceKind::Archer, 1),
        (6, 2, PieceKind::Trebuchet, 1),
        (6, 3, PieceKind::BatteringRam, 1),
    ] {
        state.set_piece(Square::new(row, col), Some(Piece { kind, owner }));
    }
    *state.token_mut(0, TokenKind::March) = Token {
        line: 3,
        side: TokenSide::Attack,
    };
    state.pending = [TokenKind::March, TokenKind::Flank];
    state.pending_len = 1;
    state.phase = Phase::Activate;

    let events = settle_state(&mut state);
    assert_eq!(
        serde_json::to_value(&events[0]).unwrap(),
        json!({ "type": "attacked", "player": 0, "token": "march", "line": 3 }),
    );

    // The swordsman on the left hilltop kills the archer next to it, passing
    // over the rock it could have taken instead.
    let events = apply_logged(
        &mut state,
        &Choice::Attack {
            from: Square::new(3, 1),
            target: Square::new(2, 1),
        },
    );
    assert_eq!(
        serde_json::to_value(&events[0]).unwrap(),
        json!({
            "type": "struck",
            "player": 0,
            "token": "march",
            "kind": "swordsman",
            "from": { "row": 3, "col": 1 },
            "target": { "row": 2, "col": 1 },
            "casualty": { "type": "piece", "piece": { "kind": "archer", "owner": 1 } },
        }),
    );

    // The trebuchet on the centre hilltop bears on all three enemy castles and
    // brings down one of them.
    let events = apply_logged(
        &mut state,
        &Choice::Attack {
            from: Square::new(3, 4),
            target: Square::new(6, 1),
        },
    );
    assert_eq!(
        serde_json::to_value(&events[0]).unwrap(),
        json!({
            "type": "struck",
            "player": 0,
            "token": "march",
            "kind": "trebuchet",
            "from": { "row": 3, "col": 4 },
            "target": { "row": 6, "col": 1 },
            "casualty": { "type": "castle" },
        }),
    );

    // And the swordsman on the right hilltop kills the rock beside it.
    let events = apply_logged(
        &mut state,
        &Choice::Attack {
            from: Square::new(3, 7),
            target: Square::new(3, 6),
        },
    );
    assert_eq!(
        serde_json::to_value(&events[0]).unwrap(),
        json!({
            "type": "struck",
            "player": 0,
            "token": "march",
            "kind": "swordsman",
            "from": { "row": 3, "col": 7 },
            "target": { "row": 3, "col": 6 },
            "casualty": { "type": "piece", "piece": { "kind": "rock", "owner": 2 } },
        }),
    );
    assert_eq!(state.piece_at(Square::new(3, 6)), None);
}

#[test]
fn holding_fire_is_its_own_event() {
    let mut state = initial_state();
    state.board = [[None; BOARD_COLS]; BOARD_ROWS];
    for (row, col, kind, owner) in [
        (3, 1, PieceKind::Swordsman, 0),
        (0, 3, PieceKind::Trebuchet, 0),
        (2, 1, PieceKind::Archer, 1),
        (6, 2, PieceKind::Trebuchet, 1),
    ] {
        state.set_piece(Square::new(row, col), Some(Piece { kind, owner }));
    }
    *state.token_mut(0, TokenKind::March) = Token {
        line: 3,
        side: TokenSide::Attack,
    };
    state.pending = [TokenKind::March, TokenKind::Flank];
    state.pending_len = 1;
    state.phase = Phase::Activate;
    settle_state(&mut state);

    let events = apply_logged(
        &mut state,
        &Choice::HoldFire {
            from: Square::new(3, 1),
        },
    );
    assert_eq!(
        serde_json::to_value(&events[0]).unwrap(),
        json!({
            "type": "heldFire",
            "player": 0,
            "token": "march",
            "kind": "swordsman",
            "from": { "row": 3, "col": 1 },
        }),
    );
}

#[test]
fn outcomes_are_tagged() {
    assert_eq!(
        serde_json::to_value(Outcome::Winner { player: 1 }).unwrap(),
        json!({ "type": "winner", "player": 1 }),
    );
    assert_eq!(
        serde_json::to_value(Outcome::Draw).unwrap(),
        json!({ "type": "draw" }),
    );
}

#[test]
fn preview_shapes_use_the_keys_the_board_draws_from() {
    let mut state = initial_state();
    state.board = [[None; BOARD_COLS]; BOARD_ROWS];
    for (row, col, kind, owner) in [
        (0, 1, PieceKind::Swordsman, 0),
        (1, 3, PieceKind::Swordsman, 1),
        (5, 0, PieceKind::Trebuchet, 0),
        (5, 8, PieceKind::BatteringRam, 0),
        (6, 0, PieceKind::Trebuchet, 1),
        (6, 8, PieceKind::BatteringRam, 1),
    ] {
        state.set_piece(Square::new(row, col), Some(Piece { kind, owner }));
    }
    *state.token_mut(0, TokenKind::March) = Token {
        line: 1,
        side: TokenSide::Attack,
    };
    *state.token_mut(0, TokenKind::Flank) = Token {
        line: 1,
        side: TokenSide::Move,
    };

    let mut activating = state;
    activating.pending = [TokenKind::Flank, TokenKind::March];
    activating.pending_len = 2;
    activating.phase = Phase::Activate;

    let moves = serde_json::to_value(move_outcomes(&activating)).unwrap();
    let onto_the_row = moves
        .as_array()
        .unwrap()
        .iter()
        .find(|value| field(value, "to") == json!({ "row": 1, "col": 2 }))
        .expect("the swordsman can step onto the armed row");
    assert_eq!(field(onto_the_row, "from"), json!({ "row": 0, "col": 1 }));
    assert_eq!(
        field(onto_the_row, "threatenedPieces"),
        json!([{ "row": 1, "col": 3 }]),
    );
    assert_eq!(field(onto_the_row, "threatenedCastles"), json!([]));

    let mut sliding = state;
    sliding.phase = Phase::Slide;
    *sliding.token_mut(0, TokenKind::Flank) = Token {
        line: 6,
        side: TokenSide::Move,
    };
    *sliding.token_mut(0, TokenKind::March) = Token {
        line: 1,
        side: TokenSide::Attack,
    };

    let slides = serde_json::to_value(slide_outcomes(&sliding)).unwrap();
    let onto_column_b = slides
        .as_array()
        .unwrap()
        .iter()
        .find(|value| field(value, "line") == json!(1))
        .expect("column b holds a piece to move");
    assert_eq!(field(onto_column_b, "token"), json!("flank"));
    assert_eq!(
        field(onto_column_b, "movers"),
        json!([{ "row": 0, "col": 1 }])
    );
    for key in ["zone", "threatenedPieces", "threatenedCastles"] {
        assert!(
            onto_column_b.get(key).is_some(),
            "SlideOutcome is missing {key}"
        );
    }
}
