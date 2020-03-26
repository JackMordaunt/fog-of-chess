// #![windows_subsystem = "windows"]

use clap::{App, Arg, SubCommand};
use derive_builder::*;
use ggez::event::{self, EventHandler};
use ggez::graphics;
use ggez::graphics::{Color, DrawParam, Font, MeshBuilder, Text};
use ggez::input::keyboard::{is_key_pressed, KeyCode};
use ggez::input::mouse::MouseButton;
use ggez::{conf::WindowMode, conf::WindowSetup};
use ggez::{Context, ContextBuilder, GameResult};

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
    let app = App::new("Fog Of Chess")
        .arg(
            Arg::with_name("no-fog")
                .takes_value(false)
                .long("no-fog")
                .help("Turn off the fog of war."),
        )
        .subcommand(
            SubCommand::with_name("test").arg(
                Arg::with_name("scenario")
                    .required(true)
                    .help("Name of scenario to test."),
            ),
        )
        .get_matches();
    let (board, single_player) = match app.subcommand_matches("test") {
        Some(test) => match Board::scenario(
            test.value_of("scenario")
                .expect("scenario argument missing"),
        ) {
            Some(board) => (board, true),
            None => panic!("scenario does not exist"),
        },
        None => (Board::new(), false),
    };
    let (mut ctx, mut event_loop) = ContextBuilder::new("Fog of War", "Jack Mordaunt")
        .window_mode(WindowMode::default().dimensions(800.0, 600.0))
        .window_setup(WindowSetup::default().title("Fog of Chess"))
        .build()
        .expect("creating game loop");
    let mut g = GameBuilder::default()
        .board(board)
        .single_player(single_player)
        .fog(!app.is_present("no-fog"))
        .selected(vec![])
        .turn(Player::White)
        .font(
            Font::new_glyph_font_bytes(&mut ctx, include_bytes!("../res/DejaVuSansMono.ttf"))
                .expect("loading font"),
        )
        .build()
        .expect("building game object");
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
                // BUG: Avoid duplicates.
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
        graphics::clear(ctx, graphics::BLACK);
        // TODO: Get actual size of window instead of hardcoding.
        let (w, h) = (800.0, 600.0);
        let (w_size, h_size) = (w / 8.0, h / 8.0);
        let mut mb = MeshBuilder::new();
        // Draw pieces.
        for (y, row) in self.board.0.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if let Some(Piece { player, unit, .. }) = cell {
                    if *player != self.turn {
                        continue;
                    }
                    // Draw the current piece.
                    if *player == self.turn {
                        // Chess pieces are part of unicode.
                        // All we need is a font that provides these.
                        // let font = graphics::Font::default();
                        let text = match unit {
                            Unit::Pawn => '\u{265F}',
                            Unit::King => '\u{265A}',
                            Unit::Queen => '\u{265B}',
                            Unit::Bishop => '\u{265D}',
                            Unit::Knight => '\u{265E}',
                            Unit::Rook => '\u{265C}',
                        };
                        let color = match player {
                            Player::White => graphics::WHITE,
                            Player::Black => graphics::BLACK,
                        };
                        let fragment: graphics::TextFragment = (text, self.font, 80.0).into();
                        graphics::queue_text(
                            ctx,
                            &Text::new(fragment),
                            // TODO: Center dynamically instead of hardcoded padding.
                            [x as f32 * w_size + 25.0, y as f32 * h_size - 10.0],
                            Some(color),
                        );
                        // Draw everything that is in line of sight of the
                        // current piece.
                        // TOOD: Given that all allied pieces will be drawn by
                        // previous code we can tighten this to "draw all
                        // enemies in line of sight".
                        for (x, y) in self
                            .line_of_sight((x as i32, y as i32))
                            .into_iter()
                            .chain(vec![(x as i32, y as i32)])
                        {
                            // Draw cell.
                            let color = if x % 2 == 0 && y % 2 == 0 {
                                SOARING_EAGLE
                            } else if x % 2 != 0 && y % 2 != 0 {
                                SOARING_EAGLE
                            } else {
                                WIZARD_GREY
                            };
                            mb.rectangle(
                                graphics::DrawMode::fill(),
                                graphics::Rect::new_i32(
                                    x as i32 * w_size as i32,
                                    y as i32 * h_size as i32,
                                    w_size as i32,
                                    h_size as i32,
                                ),
                                color,
                            );
                            // Highlight selected pieces.
                            if self.selected.contains(&(x as i32, y as i32)) {
                                mb.rectangle(
                                    graphics::DrawMode::stroke(2.0),
                                    graphics::Rect::new_i32(
                                        x as i32 * w_size as i32 + 1,
                                        y as i32 * h_size as i32 + 1,
                                        w_size as i32 - 2,
                                        h_size as i32 - 2,
                                    ),
                                    PURE_APPLE,
                                );
                            };
                            if let Some(Piece { player, unit, .. }) =
                                self.board.get((x as i32, y as i32))
                            {
                                if *player != self.turn {
                                    let text = match unit {
                                        Unit::Pawn => '\u{265F}',
                                        Unit::King => '\u{265A}',
                                        Unit::Queen => '\u{265B}',
                                        Unit::Bishop => '\u{265D}',
                                        Unit::Knight => '\u{265E}',
                                        Unit::Rook => '\u{265C}',
                                    };
                                    let color = match player {
                                        Player::White => graphics::WHITE,
                                        Player::Black => graphics::BLACK,
                                    };
                                    let fragment: graphics::TextFragment =
                                        (text, self.font, 80.0).into();
                                    graphics::queue_text(
                                        ctx,
                                        &Text::new(fragment),
                                        // TODO: Center dynamically instead of hardcoded padding.
                                        [x as f32 * w_size + 25.0, y as f32 * h_size - 10.0],
                                        Some(color),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        let mut mesh = mb.build(ctx)?;
        graphics::draw(ctx, &mut mesh, DrawParam::default())?;
        graphics::draw_queued_text(
            ctx,
            DrawParam::default(),
            None,
            graphics::FilterMode::Linear,
        )?;
        graphics::present(ctx)
    }
}

/// Unique chess units.
#[derive(Clone, Debug)]
pub enum Unit {
    Pawn,
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
}

/// Player denotes the two unique players that can own units.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Player {
    White,
    Black,
}

/// Piece is a Unit-Player pair that represents a piece on the board.
#[derive(Clone, Debug)]
pub struct Piece {
    pub unit: Unit,
    pub player: Player,
    // Track number of times this piece has been moved.
    pub moved: u32,
}

/// Board contains the location information of each piece.
#[derive(Clone, Default)]
pub struct Board([[Option<Piece>; 8]; 8]);

/// Game contains meta information.
#[derive(Clone, Builder)]
pub struct Game {
    pub board: Board,
    pub turn: Player,
    // TODO: Use a set to avoid duplicates.
    pub selected: Vec<(i32, i32)>,
    pub font: graphics::Font,
    pub fog: bool,
    pub single_player: bool,
}

impl Game {
    /// Moves calculates all valid moves for the currently selected piece.
    pub fn moves(&self, pos: (i32, i32)) -> Vec<(i32, i32)> {
        let (x, y) = pos;
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
                                if *moved == 0 && self.board.0[y as usize + 2][x as usize].is_none()
                                {
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
                                if *moved == 0 && self.board.0[y as usize - 2][x as usize].is_none()
                                {
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
                    .chain(LineOfSight::new((1..8).map(|ii| (x + ii, y)), &self.board))
                    .chain(LineOfSight::new((1..8).map(|ii| (x - ii, y)), &self.board))
                    .chain(LineOfSight::new((1..8).map(|ii| (x, y + ii)), &self.board))
                    .chain(LineOfSight::new((1..8).map(|ii| (x, y - ii)), &self.board))
                    .collect(),
                // Bishop moves all diagonal directions.
                Bishop => vec![]
                    .into_iter()
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x + ii, y + ii)),
                        &self.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x - ii, y - ii)),
                        &self.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x - ii, y + ii)),
                        &self.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x + ii, y - ii)),
                        &self.board,
                    ))
                    .collect(),
                // Queen moves in all eight directions.
                Queen => vec![]
                    .into_iter()
                    .chain(LineOfSight::new((1..8).map(|ii| (x + ii, y)), &self.board))
                    .chain(LineOfSight::new((1..8).map(|ii| (x - ii, y)), &self.board))
                    .chain(LineOfSight::new((1..8).map(|ii| (x, y + ii)), &self.board))
                    .chain(LineOfSight::new((1..8).map(|ii| (x, y - ii)), &self.board))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x + ii, y + ii)),
                        &self.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x - ii, y - ii)),
                        &self.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x - ii, y + ii)),
                        &self.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x + ii, y - ii)),
                        &self.board,
                    ))
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
    // Calculate line of sight for any piece at the given coordinate.
    pub fn line_of_sight(&self, pos: (i32, i32)) -> Vec<(i32, i32)> {
        let (x, y) = pos;
        self.moves(pos)
            .into_iter()
            .chain(
                vec![
                    (x + 1, y + 1),
                    (x - 1, y - 1),
                    (x + 1, y - 1),
                    (x - 1, y + 1),
                    (x + 1, y),
                    (x - 1, y),
                    (x, y + 1),
                    (x, y - 1),
                ]
                .into_iter(),
            )
            .collect()
    }
    /// Move a piece and conclude the turn.
    pub fn move_turn(&mut self, from: (i32, i32), to: (i32, i32)) {
        if self.contains_ally(from) {
            self.board.move_piece((from.0, from.1), (to.0, to.1));
            if !self.single_player {
                self.turn = match self.turn {
                    Player::Black => Player::White,
                    Player::White => Player::Black,
                };
            }
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
        // Outline:
        //  Consider the first two moves of the selection as king and rook.
        //  Attempt the castle:
        //  - King and Rook must be in original positions.
        //  - The two spaces between them must be empty.
        //  - King and Rook swap to the middle two pieces, completing the castle.
        //
        // Clone out the first two selected coordinates.
        let pieces = self
            .selected
            .iter()
            .cloned()
            .take(2)
            .collect::<Vec<(i32, i32)>>();
        let (king_pos, rook_pos) = (pieces[0], pieces[1]);
        // Check for King and Rook.
        if let (
            Some(&Piece {
                unit: Unit::King, ..
            }),
            Some(&Piece {
                unit: Unit::Rook, ..
            }),
        ) = (self.board.get(king_pos), self.board.get(rook_pos))
        {
            if (king_pos.0 - rook_pos.0).abs() == 2 {
                // correct distance
                println!("can castle!");
            }
            // - Original positions
            // - Empty between them
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
    /// scenario sets up a board for the given scenario, identified by name.
    pub fn scenario(title: &str) -> Option<Self> {
        match title {
            "castle" => Some(Board::castle_test()),
            _ => None,
        }
    }
    /// castle_test creates a new board for testing castle moves.
    fn castle_test() -> Self {
        use Player::*;
        use Unit::*;
        Board([
            [
                Some(Piece {
                    unit: Rook,
                    player: White,
                    moved: 0,
                }),
                None,
                None,
                Some(Piece {
                    unit: King,
                    player: White,
                    moved: 0,
                }),
                None,
                None,
                None,
                None,
            ],
            [None, None, None, None, None, None, None, None],
            [None, None, None, None, None, None, None, None],
            [None, None, None, None, None, None, None, None],
            [None, None, None, None, None, None, None, None],
            [None, None, None, None, None, None, None, None],
            [None, None, None, None, None, None, None, None],
            [None, None, None, None, None, None, None, None],
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

// LineOfSight yields coordinates from a move-set until a piece is found.
// Truncate move-set for Queen/Rook/Bishop such that these pieces cannot
// jump over another.
struct LineOfSight<'a, Moves>
where
    Moves: Iterator<Item = (i32, i32)>,
{
    moves: Moves,
    board: &'a Board,
    stop: bool,
}

impl<'a, Moves> LineOfSight<'a, Moves>
where
    Moves: Iterator<Item = (i32, i32)>,
{
    fn new(moves: Moves, board: &'a Board) -> Self {
        LineOfSight {
            moves,
            board,
            stop: false,
        }
    }
}

impl<'a, Moves> Iterator for LineOfSight<'a, Moves>
where
    Moves: Iterator<Item = (i32, i32)>,
{
    type Item = (i32, i32);
    fn next(&mut self) -> Option<Self::Item> {
        if self.stop {
            return None;
        }
        match self.moves.next() {
            Some((x, y)) => match self.board.get((x, y)) {
                Some(_) => {
                    self.stop = true;
                    Some((x, y))
                }
                None => Some((x, y)),
            },
            None => None,
        }
    }
}
