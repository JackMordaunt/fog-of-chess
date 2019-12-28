use ggez::event::{self, EventHandler};
use ggez::graphics;
use ggez::graphics::{Color, DrawParam, MeshBuilder, Text};
use ggez::input::keyboard::{is_key_pressed, KeyCode};
use ggez::input::mouse::MouseButton;
use ggez::{conf::WindowMode, Context, ContextBuilder, GameResult};

const PURE_APPLE: Color = Color {
    r: 106.0 / 256.0,
    g: 176.0 / 256.0,
    b: 76.0 / 256.0,
    a: 1.0,
};

const SOARING_EAGLE: Color = Color {
    r: 149.0 / 256.0,
    g: 175.0 / 256.0,
    b: 192.0 / 256.0,
    a: 1.0,
};

const WIZARD_GREY: Color = Color {
    r: 83.0 / 256.0,
    g: 92.0 / 256.0,
    b: 104.0 / 256.0,
    a: 1.0,
};

fn main() {
    let (mut ctx, mut event_loop) = ContextBuilder::new("Fog of War", "Jack Mordaunt")
        .window_mode(WindowMode::default().dimensions(800.0, 600.0))
        .build()
        .unwrap();
    let mut g = Game::new();
    match event::run(&mut ctx, &mut event_loop, &mut g) {
        Ok(_) => {}
        Err(e) => println!("error: {}", e),
    }
}

impl EventHandler for Game {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        Ok(())
    }

    fn mouse_button_up_event(&mut self, ctx: &mut Context, _b: MouseButton, x: f32, y: f32) {
        let (w, h) = (800.0, 600.0);
        let (w_size, h_size) = (w / 8.0, h / 8.0);
        let (col, row) = ((x / w_size).floor() as i32, (y / h_size).floor() as i32);
        if is_key_pressed(ctx, KeyCode::LShift) {
            if self.contains_ally((col, row)) {
                self.selected.push((col, row));
            }
        } else {
            match self.board.get((col, row)) {
                // Move.
                None => {
                    // Multi selection is a compound move.
                    // Given the only compound move in standard chess is the
                    // "castle", we directly call into it.
                    if self.selected.len() > 1 {
                        self.castle_move()
                    } else {
                        if let Some((x, y)) = self.first_selected() {
                            if self.moves((x, y)).contains(&(col, row)) {
                                self.move_turn((x, y), (col, row));
                            }
                        }
                    }
                }
                // Attack move.
                Some(Piece { player, .. }) => {
                    if *player != self.turn && self.selected.len() == 1 {
                        if let Some((x, y)) = self.first_selected() {
                            if self.moves((x, y)).contains(&(col, row)) {
                                self.move_turn((x, y), (col, row));
                            }
                        }
                    } else {
                        if self.contains_ally((col, row)) {
                            self.selected = vec![(col as i32, row as i32)];
                        }
                    }
                }
            };
        }
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, SOARING_EAGLE);
        // TODO: Get actual size of window instead of hardcoding.
        let (w, h) = (800.0, 600.0);
        let (w_size, h_size) = (w / 8.0, h / 8.0);
        let mut mb = MeshBuilder::new();
        // Draw grid lines.
        // Columns.
        for ii in 0..9 {
            mb.line(
                &[[ii as f32 * w_size, h], [ii as f32 * w_size, 0.0]],
                2.0,
                WIZARD_GREY,
            )
            .unwrap();
        }
        // Rows.
        for ii in 0..9 {
            mb.line(
                &[[0.0, ii as f32 * h_size], [w, ii as f32 * h_size]],
                2.0,
                WIZARD_GREY,
            )
            .unwrap();
        }
        // Draw pieces.
        for (y, row) in self.board.0.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if let Some(Piece { player, unit, .. }) = cell {
                    let color = match player {
                        Player::White => graphics::WHITE,
                        Player::Black => graphics::BLACK,
                    };
                    // Highlight if selected piece.
                    let color = if self.selected.contains(&(x as i32, y as i32)) {
                        PURE_APPLE
                    } else {
                        color
                    };
                    // Render each unit as a text fragment.
                    // TODO: Use nice textures.
                    let text = match unit {
                        Unit::Pawn => "Pawn",
                        Unit::King => "King",
                        Unit::Queen => "Queen",
                        Unit::Bishop => "Bishop",
                        Unit::Knight => "Knight",
                        Unit::Rook => "Rook",
                    };
                    let font = graphics::Font::default();
                    let fragment: graphics::TextFragment = (text, font, 30.0).into();
                    graphics::queue_text(
                        ctx,
                        &Text::new(fragment),
                        // TODO: Center dynamically instead of hardcoded padding.
                        [x as f32 * w_size + 7.0, y as f32 * h_size + 7.0],
                        Some(color),
                    );
                }
            }
        }
        let mut mesh = mb.build(ctx)?;
        graphics::draw(ctx, &mut mesh, DrawParam::default())?;
        graphics::draw_queued_text(
            ctx,
            DrawParam::default(),
            None,
            graphics::FilterMode::Nearest,
        )?;
        graphics::present(ctx)
    }
}

/// Unique chess units.
#[derive(Debug)]
pub enum Unit {
    Pawn,
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
}

/// Player denotes the two unique players that can own units.
#[derive(Debug, Eq, PartialEq)]
pub enum Player {
    White,
    Black,
}

/// Piece is a Unit-Player pair that represents a piece on the board.
#[derive(Debug)]
pub struct Piece {
    pub unit: Unit,
    pub player: Player,
    // Track number of times this piece has been moved.
    pub moved: u32,
}

/// Board contains the location information of each piece.
#[derive(Default)]
pub struct Board([[Option<Piece>; 8]; 8]);

/// Game contains meta information.
pub struct Game {
    pub board: Board,
    pub turn: Player,
    // Track selected pieces.
    pub selected: Vec<(i32, i32)>,
}

impl Game {
    /// New creates a default chess game.
    pub fn new() -> Self {
        Game {
            board: Board::new(),
            turn: Player::White,
            selected: vec![],
        }
    }
    /// Moves calculates all valid moves for the currently selected piece.
    // TODO: Finish movement logic.
    pub fn moves(&self, pos: (i32, i32)) -> Vec<(i32, i32)> {
        let (x, y) = pos;
        // How do we handle a vec of selected pieces while satisfying the BC.
        use Unit::*;
        match self.board.get((x, y)) {
            Some(Piece {
                unit,
                player,
                moved,
            }) => match unit {
                // Pawn can move in the direction of the player by 1 square.
                // For the first move, a pawn can move up to 2 squares.
                // Pawns can only attack diagonally in the direction of the
                // player.
                // Cannot attack straight ahead.
                Pawn => {
                    let mut moves = vec![];
                    match player {
                        // Clean: The only difference between these two
                        // blocks is the direction.
                        Player::White => {
                            if self.contains_enemy((x - 1, y + 1)) {
                                moves.push((x - 1, y + 1));
                            }
                            if self.contains_enemy((x + 1, y + 1)) {
                                moves.push((x + 1, y + 1));
                            }
                            if self.board.0[y as usize + 1][x as usize].is_none() {
                                moves.push((x, y + 1));
                                if *moved == 0 {
                                    moves.push((x, y + 2));
                                }
                            }
                        }
                        Player::Black => {
                            if self.contains_enemy((x - 1, y - 1)) {
                                moves.push((x - 1, y - 1));
                            }
                            if self.contains_enemy((x + 1, y - 1)) {
                                moves.push((x + 1, y - 1));
                            }
                            if self.board.0[y as usize - 1][x as usize].is_none() {
                                moves.push((x, y - 1));
                                if *moved == 0 {
                                    moves.push((x, y - 2));
                                }
                            }
                        }
                    };
                    moves
                }
                // Knight moves in an L shape: two out, one across.
                Knight => vec![
                    (x + 2, y - 1),
                    (x + 2, y + 1),
                    (x - 2, y - 1),
                    (x - 2, y + 1),
                    (x + 1, y + 2),
                    (x - 1, y + 2),
                    (x + 1, y - 2),
                    (x - 1, y - 2),
                ],
                // Rook moves in all non diagonal directions.
                Rook => vec![]
                    .into_iter()
                    .chain((1..8).map(|ii| (x + ii, y)))
                    .chain((1..8).map(|ii| (x - ii, y)))
                    .chain((1..8).map(|ii| (x, y + ii)))
                    .chain((1..8).map(|ii| (x, y - ii)))
                    .collect(),
                // Bishop moves all diagonal directions.
                Bishop => vec![]
                    .into_iter()
                    .chain((1..8).map(|ii| (x + ii, y + ii)))
                    .chain((1..8).map(|ii| (x - ii, y - ii)))
                    .chain((1..8).map(|ii| (x - ii, y + ii)))
                    .chain((1..8).map(|ii| (x + ii, y - ii)))
                    .collect(),
                // Queen moves in all eight directions.
                Queen => vec![]
                    .into_iter()
                    .chain((1..8).map(|ii| (x + ii, y)))
                    .chain((1..8).map(|ii| (x - ii, y)))
                    .chain((1..8).map(|ii| (x, y + ii)))
                    .chain((1..8).map(|ii| (x, y - ii)))
                    .chain((1..8).map(|ii| (x + ii, y + ii)))
                    .chain((1..8).map(|ii| (x - ii, y - ii)))
                    .chain((1..8).map(|ii| (x - ii, y + ii)))
                    .chain((1..8).map(|ii| (x + ii, y - ii)))
                    .collect(),
                // King can move to any adjacent cell that isn't occupied by
                // a piece of the same player.
                King => vec![
                    (x + 1, y + 1),
                    (x - 1, y - 1),
                    (x + 1, y - 1),
                    (x - 1, y + 1),
                    (x + 1, y),
                    (x - 1, y),
                    (x, y + 1),
                    (x, y - 1),
                ],
            },
            None => vec![],
        }
        .into_iter()
        .filter(|(x, y)| !self.contains_ally((*x, *y)))
        .collect()
    }
    /// Move a piece and conclude the turn.
    pub fn move_turn(&mut self, from: (i32, i32), to: (i32, i32)) {
        if self.contains_ally(from) {
            self.board.move_piece((from.0, from.1), (to.0, to.1));
            self.turn = match self.turn {
                Player::Black => Player::White,
                Player::White => Player::Black,
            };
            self.selected = vec![];
        }
    }
    /// Contains enemy if the specified position is occupied by a piece owned
    /// by the other player.
    pub fn contains_enemy(&self, pos: (i32, i32)) -> bool {
        let (x, y) = pos;
        if x > -1 && y > -1 && x - 1 < 7 && y - 1 < 7 {
            match &self.board.0[y as usize][x as usize] {
                Some(Piece { player, .. }) => *player != self.turn,
                _ => false,
            }
        } else {
            false
        }
    }
    /// Contains ally if the specified position is occupied by a piece owned by
    /// the currently player.
    pub fn contains_ally(&self, pos: (i32, i32)) -> bool {
        let (x, y) = pos;
        if x > -1 && y > -1 && x - 1 < 7 && y - 1 < 7 {
            match &self.board.0[y as usize][x as usize] {
                Some(Piece { player, .. }) => *player == self.turn,
                None => false,
            }
        } else {
            false
        }
    }
    /// Coordinate of the first selected piece.
    fn first_selected(&mut self) -> Option<(i32, i32)> {
        if self.selected.len() > 0 {
            Some(self.selected[0].clone())
        } else {
            None
        }
    }
    // TODO: impl castle move.
    fn castle_move(&mut self) {
        // Consider the first two moves of the selection as king and rook.
        // Attempt the castle:
        // - King and Rook must in original positions.
        // - The two spaces between them must be empty.
        // Clone out the first two selected coordinates.
        // let pieces = self
        //     .selected
        //     .iter()
        //     .cloned()
        //     .take(2)
        //     .collect::<Vec<(i32, i32)>>();
        // // Check for King and Rook.
        // if let (
        //     Some(Piece {
        //         unit: Unit::King, ..
        //     }),
        //     Some(Piece {
        //         unit: Unit::Rook, ..
        //     }),
        // ) = (self.board.get(pieces[0]), self.board.get(pieces[1]))
        // {
        //     let (king, rook) = (pieces[0], pieces[1]);
        //     if (king.0 - rook.0).abs() == 2 {
        //         // correct distance

        //     }
        //     // - Original positions
        //     // - Empty between them
        // }
    }
}

impl Board {
    pub fn new() -> Self {
        use Player::*;
        use Unit::*;
        Board([
            [
                Some(Piece {
                    unit: Rook,
                    player: White,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Knight,
                    player: White,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Bishop,
                    player: White,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Queen,
                    player: White,
                    moved: 0,
                }),
                Some(Piece {
                    unit: King,
                    player: White,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Bishop,
                    player: White,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Knight,
                    player: White,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Rook,
                    player: White,
                    moved: 0,
                }),
            ],
            [
                Some(Piece {
                    unit: Pawn,
                    player: White,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: White,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: White,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: White,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: White,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: White,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: White,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: White,
                    moved: 0,
                }),
            ],
            [None, None, None, None, None, None, None, None],
            [None, None, None, None, None, None, None, None],
            [None, None, None, None, None, None, None, None],
            [None, None, None, None, None, None, None, None],
            [
                Some(Piece {
                    unit: Pawn,
                    player: Black,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: Black,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: Black,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: Black,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: Black,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: Black,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: Black,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: Black,
                    moved: 0,
                }),
            ],
            [
                Some(Piece {
                    unit: Rook,
                    player: Black,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Knight,
                    player: Black,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Bishop,
                    player: Black,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Queen,
                    player: Black,
                    moved: 0,
                }),
                Some(Piece {
                    unit: King,
                    player: Black,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Bishop,
                    player: Black,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Knight,
                    player: Black,
                    moved: 0,
                }),
                Some(Piece {
                    unit: Rook,
                    player: Black,
                    moved: 0,
                }),
            ],
        ])
    }
    /// Get the piece at the specified (x, y) coordinate.
    pub fn get(&self, pos: (i32, i32)) -> Option<&Piece> {
        let (x, y) = pos;
        if x < 0 || y < 0 || x > 7 || y > 7 {
            None
        } else {
            self.0[y as usize][x as usize].as_ref()
        }
    }
    /// Set the piece to the specified (x, y) coordinate.
    /// Overwrites anything already at the location.
    /// Noop if the coordinates are out of bounds.
    pub fn set(&mut self, pos: (i32, i32), p: Piece) {
        let (x, y) = pos;
        if !(x < 0 || y < 0 || x > 7 || y > 7) {
            self.0[y as usize][x as usize] = Some(p);
        }
    }
    /// Move any piece at `from` to `to`.
    /// Noop if there is no piece at `from`.
    pub fn move_piece(&mut self, from: (i32, i32), to: (i32, i32)) {
        if [from.0, from.1, to.0, to.1]
            .iter()
            .fold(false, |outofbounds, next| {
                outofbounds || *next > 7 || *next < 0
            })
        {
            return;
        }
        if let Some(Piece {
            unit,
            player,
            moved,
        }) = self.0[from.1 as usize][from.0 as usize].take()
        {
            self.set(
                (to.0, to.1),
                Piece {
                    unit: unit,
                    player: player,
                    moved: moved + 1,
                },
            );
        }
    }
}
