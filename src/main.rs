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
        // map x,y to cell.
        // break into 8 rows and columns.
        // test for which cell we are in.
        let (w, h) = (800.0, 600.0);
        let (w_size, h_size) = (w / 8.0, h / 8.0);
        let (col, row) = ((x / w_size).floor() as u8, (y / h_size).floor() as u8);
        // TODO: Sanity check.
        match self.board.0[row as usize][col as usize] {
            None => {
                if let Some((x, y)) = self.selected_piece {
                    println!(
                        "(x, y) = ({}, {}), (col, row) = ({}, {}), moves = {:?}",
                        x,
                        y,
                        col,
                        row,
                        self.moves()
                    );
                    if self.moves().contains(&(col, row)) {
                        if let Some(piece) = self.board.0[y as usize][x as usize].take() {
                            self.board.0[row as usize][col as usize] = Some(piece);
                        }
                    }
                }
            }
            Some(_) => {
                self.selected_piece = Some((col as u8, row as u8));
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
                if let Some(Piece { player, unit }) = cell {
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

impl Unit {
    /// Calculate valid moves for the given unit at the given location.
    /// Assumes standard chess rules.
    pub fn moves(&self, location: (u8, u8)) -> Vec<(u8, u8)> {
        use Unit::*;
        let (x, y) = location;
        match self {
            Pawn => vec![(x, y + 1)],
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
        }
    }
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
}

/// Board contains the location information of each piece.
#[derive(Default)]
pub struct Board([[Option<Piece>; 8]; 8]);

/// Game contains meta information.
pub struct Game {
    pub board: Board,
    pub turn: Player,
    pub selected_piece: Option<(u8, u8)>,
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
    pub fn moves(&self) -> Vec<(u8, u8)> {
        use Unit::*;
        match self.selected_piece {
            Some((x, y)) => match &self.board.0[y as usize][x as usize] {
                Some(Piece { unit, player }) => match unit {
                    // Pawn can move in the direction of the player by 1 square.
                    // For the first move, a pawn can move up to 2 squares.
                    // Pawns can only attack diagonally in the direction of the
                    // player.
                    Pawn => vec![(x, y - 1)],
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
                    ]
                    .into_iter()
                    .filter(|(x, y)| {
                        if x - 1 < 7 && y - 1 < 7 {
                            match &self.board.0[*y as usize][*x as usize] {
                                Some(Piece { player: p, .. }) => player != p,
                                None => true,
                            }
                        } else {
                            false
                        }
                    })
                    .collect(),
                    _ => vec![],
                },
                None => vec![],
            },
            None => vec![],
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
                }),
                Some(Piece {
                    unit: Knight,
                    player: White,
                }),
                Some(Piece {
                    unit: Bishop,
                    player: White,
                }),
                Some(Piece {
                    unit: Queen,
                    player: White,
                }),
                Some(Piece {
                    unit: King,
                    player: White,
                }),
                Some(Piece {
                    unit: Bishop,
                    player: White,
                }),
                Some(Piece {
                    unit: Knight,
                    player: White,
                }),
                Some(Piece {
                    unit: Rook,
                    player: White,
                }),
            ],
            [
                Some(Piece {
                    unit: Pawn,
                    player: White,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: White,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: White,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: White,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: White,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: White,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: White,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: White,
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
                }),
                Some(Piece {
                    unit: Pawn,
                    player: Black,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: Black,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: Black,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: Black,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: Black,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: Black,
                }),
                Some(Piece {
                    unit: Pawn,
                    player: Black,
                }),
            ],
            [
                Some(Piece {
                    unit: Rook,
                    player: Black,
                }),
                Some(Piece {
                    unit: Knight,
                    player: Black,
                }),
                Some(Piece {
                    unit: Bishop,
                    player: Black,
                }),
                Some(Piece {
                    unit: Queen,
                    player: Black,
                }),
                Some(Piece {
                    unit: King,
                    player: Black,
                }),
                Some(Piece {
                    unit: Bishop,
                    player: Black,
                }),
                Some(Piece {
                    unit: Knight,
                    player: Black,
                }),
                Some(Piece {
                    unit: Rook,
                    player: Black,
                }),
            ],
        ])
    }
}
