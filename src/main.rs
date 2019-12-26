// TODO:
// - Game loop
// - Rendering

use ggez::event::{self, EventHandler};
use ggez::graphics;
use ggez::graphics::{Color, DrawParam, MeshBuilder, Text};
use ggez::input::mouse::MouseButton;
use ggez::{conf::WindowMode, Context, ContextBuilder, GameResult};

const DEEP_COVE: Color = Color {
    r: 19.0 / 256.0,
    g: 15.0 / 256.0,
    b: 64.0 / 256.0,
    a: 1.0,
};

const QUINCE_JELLY: Color = Color {
    r: 240.0 / 256.0,
    g: 147.0 / 256.0,
    b: 43.0 / 256.0,
    a: 1.0,
};

const PURE_APPLE: Color = Color {
    r: 106.0 / 256.0,
    g: 176.0 / 256.0,
    b: 76.0 / 256.0,
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

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, _b: MouseButton, x: f32, y: f32) {
        let (w, h) = (800.0, 600.0);
        let (w_size, h_size) = (w / 8.0, h / 8.0);
        let (col, row) = ((x / w_size).floor() as i32, (y / h_size).floor() as i32);
        // TODO: Sanity check.
        // TODO: Refactor out movement code.
        match self.board.0[row as usize][col as usize] {
            None => {
                if let Some((x, y)) = self.selected_piece {
                    if self.moves().contains(&(col, row)) {
                        if let Some(Piece {
                            unit,
                            player,
                            moved,
                        }) = self.board.0[y as usize][x as usize].take()
                        {
                            self.board.0[row as usize][col as usize] = Some(Piece {
                                unit: unit,
                                player: player,
                                moved: moved + 1,
                            });
                        }
                    }
                }
            }
            Some(_) => {
                if self.contains_enemy((col, row)) {
                    if let Some((x, y)) = self.selected_piece {
                        if self.moves().contains(&(col, row)) {
                            if let Some(Piece {
                                unit,
                                player,
                                moved,
                            }) = self.board.0[y as usize][x as usize].take()
                            {
                                self.board.0[row as usize][col as usize] = Some(Piece {
                                    unit: unit,
                                    player: player,
                                    moved: moved + 1,
                                });
                            }
                        }
                    }
                } else {
                    self.selected_piece = Some((col as i32, row as i32));
                }
            }
        };
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, DEEP_COVE);
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
                QUINCE_JELLY,
            )
            .unwrap();
        }
        // Rows.
        for ii in 0..9 {
            mb.line(
                &[[0.0, ii as f32 * h_size], [w, ii as f32 * h_size]],
                2.0,
                QUINCE_JELLY,
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
                    let color = match self.selected_piece {
                        Some((xx, yy)) => {
                            if xx as usize == x && yy as usize == y {
                                PURE_APPLE
                            } else {
                                color
                            }
                        }
                        _ => color,
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
pub enum Unit {
    Pawn,
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
}

/// Player denotes the two unique players that can own units.
#[derive(Eq, PartialEq)]
pub enum Player {
    White,
    Black,
}

/// Piece is a Unit-Player pair that represents a piece on the board.
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
    pub selected_piece: Option<(i32, i32)>,
}

impl Game {
    /// New creates a default chess game.
    pub fn new() -> Self {
        Game {
            board: Board::new(),
            turn: Player::White,
            selected_piece: None,
        }
    }
    /// Moves calculates all valid moves for the currently selected piece.
    // TODO: Finish movement logic.
    pub fn moves(&self) -> Vec<(i32, i32)> {
        use Unit::*;
        match self.selected_piece {
            Some((x, y)) => match &self.board.0[y as usize][x as usize] {
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
                                moves.push((x - 1, y - 1));
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
                    Rook => vec![],
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
                    _ => vec![],
                },
                None => vec![],
            },
            None => vec![],
        }
        .into_iter()
        .filter(|(x, y)| self.contains_ally((*x, *y)))
        .collect()
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
                None => true,
            }
        } else {
            false
        }
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
}
