use clap::{App, Arg, SubCommand};
use derive_builder::*;
use ggez::event::{self, EventHandler};
use ggez::graphics::{self, Color, DrawMode, DrawParam, Font, MeshBuilder, Rect, Text};
use ggez::input::keyboard::{is_key_pressed, KeyCode, KeyMods};
use ggez::input::mouse::MouseButton;
use ggez::{conf::WindowMode, conf::WindowSetup};
use ggez::{Context, ContextBuilder, GameResult};
use std::collections::HashSet;

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
        .arg(
            Arg::with_name("debug-stats")
                .takes_value(false)
                .long("debug-stats")
                .help("Show useful information for debugging."),
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
    let (width, height) = (800.0, 800.0);
    let (mut ctx, mut event_loop) = ContextBuilder::new("Fog of War", "Jack Mordaunt")
        .window_mode(
            WindowMode::default()
                .dimensions(width, height)
                .resizable(true),
        )
        .window_setup(WindowSetup::default().title("Fog of Chess"))
        .build()
        .expect("creating game loop");
    let state = StateBuilder::default()
        .board(board)
        .single_player(single_player)
        .fog(!app.is_present("no-fog"))
        .selected(HashSet::new())
        .turn(Player::White)
        .font(
            Font::new_glyph_font_bytes(&mut ctx, include_bytes!("../res/DejaVuSansMono.ttf"))
                .expect("loading font"),
        )
        .debug_stats(app.is_present("debug-stats"))
        .build()
        .expect("building game object");
    event::run(
        ctx,
        event_loop,
        Game {
            state: state.clone(),
            initial: state.clone(),
        },
    )
}

impl EventHandler for Game {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        Ok(())
    }

    fn key_up_event(&mut self, _ctx: &mut Context, kc: KeyCode, _keymods: KeyMods) {
        if cfg!(debug_assertions) {
            match kc {
                KeyCode::F => self.state.fog = !self.state.fog,
                KeyCode::F3 => self.state.debug_stats = !self.state.debug_stats,
                KeyCode::R => self.state = self.initial.clone(),
                _ => {}
            };
        }
    }

    fn mouse_button_up_event(&mut self, ctx: &mut Context, _b: MouseButton, x: f32, y: f32) {
        let (col, row) = self.pixels_to_grid(ctx, (x, y));
        if is_key_pressed(ctx, KeyCode::LShift) {
            if self.contains_ally((col, row)) {
                // BUG: Avoid duplicates.
                self.state.selected.insert((col, row));
            }
        } else {
            match self.state.board.get((col, row)) {
                None => {
                    // Multi selection is a potential compound move.
                    // Given the only compound move in standard chess is the
                    // "castle", we directly call into it.
                    if self.state.selected.len() > 1 {
                        self.castle_move();
                    } else {
                        if let Some((x, y)) = self.state.selected.iter().next().cloned() {
                            if self.moves((x, y)).contains(&(col, row)) {
                                self.move_turn((x, y), (col, row));
                            }
                        }
                    }
                }
                Some(Piece { player, .. }) => {
                    if self.is_enemy(player) && self.state.selected.len() == 1 {
                        if let Some((x, y)) = self.state.selected.iter().next().cloned() {
                            self.attack_move((x, y), (col, row));
                        }
                    } else {
                        if self.contains_ally((col, row)) {
                            self.state.selected.clear();
                            self.state.selected.insert((col, row));
                        }
                    }
                }
            };
        }
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        self.draw_board(ctx)?;
        self.draw_pieces(ctx)?;
        self.draw_highlights(ctx)?;
        if self.state.fog {
            self.draw_fog(ctx)?;
        }
        if self.state.debug_stats {
            self.draw_debug_stats(ctx)?;
        }
        graphics::draw_queued_text(
            ctx,
            DrawParam::default(),
            None,
            graphics::FilterMode::Linear,
        )?;
        graphics::present(ctx)
    }

    fn resize_event(&mut self, ctx: &mut Context, width: f32, height: f32) {
        graphics::set_screen_coordinates(ctx, Rect::new(0.0, 0.0, width, height))
            .expect("graphics::set_screen_coordinates");
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
#[derive(Clone)]
pub struct Game {
    pub initial: State,
    pub state: State,
}

#[derive(Clone, Builder)]
pub struct State {
    pub board: Board,
    pub turn: Player,
    // TODO: Use a set to avoid duplicates.
    pub selected: HashSet<(i32, i32)>,
    pub font: graphics::Font,
    pub fog: bool,
    pub single_player: bool,
    pub debug_stats: bool,
}

impl Game {
    /// Moves calculates all valid moves for the currently selected piece.
    pub fn moves(&self, pos: (i32, i32)) -> Vec<(i32, i32)> {
        let (x, y) = pos;
        use Unit::*;
        match self.state.board.get((x, y)) {
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
                            if self.state.board.0[y as usize + 1][x as usize].is_none() {
                                moves.push((x, y + 1));
                                if *moved == 0
                                    && self.state.board.0[y as usize + 2][x as usize].is_none()
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
                            if self.state.board.0[y as usize - 1][x as usize].is_none() {
                                moves.push((x, y - 1));
                                if *moved == 0
                                    && self.state.board.0[y as usize - 2][x as usize].is_none()
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
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x + ii, y)),
                        &self.state.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x - ii, y)),
                        &self.state.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x, y + ii)),
                        &self.state.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x, y - ii)),
                        &self.state.board,
                    ))
                    .collect(),
                // Bishop moves all diagonal directions.
                Bishop => vec![]
                    .into_iter()
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x + ii, y + ii)),
                        &self.state.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x - ii, y - ii)),
                        &self.state.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x - ii, y + ii)),
                        &self.state.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x + ii, y - ii)),
                        &self.state.board,
                    ))
                    .collect(),
                // Queen moves in all eight directions.
                Queen => vec![]
                    .into_iter()
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x + ii, y)),
                        &self.state.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x - ii, y)),
                        &self.state.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x, y + ii)),
                        &self.state.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x, y - ii)),
                        &self.state.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x + ii, y + ii)),
                        &self.state.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x - ii, y - ii)),
                        &self.state.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x - ii, y + ii)),
                        &self.state.board,
                    ))
                    .chain(LineOfSight::new(
                        (1..8).map(|ii| (x + ii, y - ii)),
                        &self.state.board,
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
            self.state.board.move_piece((from.0, from.1), (to.0, to.1));
            if !self.state.single_player {
                self.state.turn = match self.state.turn {
                    Player::Black => Player::White,
                    Player::White => Player::Black,
                };
            }
            self.state.selected.clear();
        }
    }
    /// Attack move one piece onto another.
    pub fn attack_move(&mut self, from: (i32, i32), to: (i32, i32)) {
        if self.moves((from.0, from.1)).contains(&(to.0, to.1)) {
            self.move_turn((from.0, from.1), (to.0, to.1));
        }
    }
    /// Contains enemy if the specified position is occupied by a piece owned
    /// by the other player.
    pub fn contains_enemy(&self, pos: (i32, i32)) -> bool {
        let (x, y) = pos;
        if x > -1 && y > -1 && x - 1 < 7 && y - 1 < 7 {
            match &self.state.board.0[y as usize][x as usize] {
                Some(Piece { player, .. }) => self.is_enemy(player),
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
            match &self.state.board.0[y as usize][x as usize] {
                Some(Piece { player, .. }) => *player == self.state.turn,
                None => false,
            }
        } else {
            false
        }
    }
    /// Perform castle move if valid.
    /// Castle move where King and Rook crossover into the 2 spaces between them.
    /// Only valid if:
    /// - Pieces are the same player (duh).
    /// - Neither piece has been moved.
    /// - Nothing is in the two spaces between them.
    fn castle_move(&mut self) {
        let moves = self
            .state
            .selected
            .iter()
            .take(2)
            .filter_map(|pos| match self.state.board.get(*pos).cloned() {
                Some(piece) => Some((pos, piece)),
                None => None,
            })
            .filter_map(|(pos, piece)| {
                // Direction is derived from standard chess layout,
                // where Rook is 3 positions to the left of the King.
                let projected_move = match piece {
                    Piece {
                        unit: Unit::Rook, ..
                    } => (pos.0 + 2, pos.1),
                    Piece {
                        unit: Unit::King, ..
                    } => (pos.0 - 2, pos.1),
                    _ => return None,
                };
                if piece.moved > 0 || self.state.board.get(projected_move).is_some() {
                    None
                } else {
                    Some((*pos, projected_move))
                }
            })
            .collect::<Vec<((i32, i32), (i32, i32))>>();
        if moves.len() == 2 {
            for (from, to) in moves {
                self.state.board.move_piece(from, to);
            }
            self.state.selected.clear();
        }
    }
    /// Draw the board which the pieces are placed onto.
    fn draw_board(&self, ctx: &mut Context) -> GameResult<()> {
        let (w, h) = self.cell_size(ctx);
        let mut mb = MeshBuilder::new();
        for Position { x, y, .. } in self.state.board.iter() {
            let (x, y) = (x as i32, y as i32);
            // TODO: get color from color map.
            let color = if x % 2 == 0 && y % 2 == 0 {
                SOARING_EAGLE
            } else if x % 2 != 0 && y % 2 != 0 {
                SOARING_EAGLE
            } else {
                WIZARD_GREY
            };
            let (x, y) = (x as f32, y as f32);
            mb.rectangle(
                graphics::DrawMode::fill(),
                graphics::Rect::new(x * w, y * h, w, h),
                color,
            );
        }
        let mut mesh = mb.build(ctx)?;
        graphics::draw(ctx, &mut mesh, DrawParam::default())
    }
    // Draw the chess pieces onto the baord.
    fn draw_pieces(&self, ctx: &mut Context) -> GameResult<()> {
        let (w, h) = self.cell_size(ctx);
        let size = w.min(h);
        for Position { x, y, piece } in self.state.board.iter() {
            if let Some(Piece { player, unit, .. }) = piece {
                // Chess pieces are part of unicode.
                // All we need is a font that provides these.
                let text = match unit {
                    Unit::Pawn => '\u{265F}',
                    Unit::King => '\u{265A}',
                    Unit::Queen => '\u{265B}',
                    Unit::Bishop => '\u{265D}',
                    Unit::Knight => '\u{265E}',
                    Unit::Rook => '\u{265C}',
                };
                let color = match player {
                    Player::White => graphics::Color::WHITE,
                    Player::Black => graphics::Color::BLACK,
                };
                // In order to center the pieces there are a few tricks to do.
                // First, scale the text by the larger side to "fill out" the space.
                // Then queue and draw the text immediately, centering the text horizontally.
                // The fixed offset of -2.0 is required to counteract 1px borders (I think!).
                // The text must be drawn individually so that we can scale each fragment individually.
                let fragment: graphics::TextFragment = (text, self.state.font, size).into();
                graphics::queue_text(ctx, &Text::new(fragment), [0.0, 0.0], Some(color));
                let scale = if h > w {
                    [1.0, h / w]
                } else if w > h {
                    [w / h, 1.0]
                } else {
                    [1.0, 1.0]
                };
                graphics::draw_queued_text(
                    ctx,
                    DrawParam::default()
                        .dest([x as f32 * w + (w / 4.0 - 2.0), y as f32 * h])
                        .scale(scale),
                    None,
                    graphics::FilterMode::Linear,
                )?;
            }
        }
        Ok(())
    }
    // Draw highlights for selected pieces.
    fn draw_highlights(&self, ctx: &mut Context) -> GameResult<()> {
        let mut mb = MeshBuilder::new();
        let (w, h) = self.cell_size(ctx);
        for (x, y) in self.state.selected.iter() {
            let (x, y) = (*x as f32, *y as f32);
            mb.rectangle(
                DrawMode::stroke(2.0),
                Rect::new(x * w, y * h, w, h),
                PURE_APPLE,
            );
        }
        if let Ok(mut mesh) = mb.build(ctx) {
            graphics::draw(ctx, &mut mesh, DrawParam::default())?;
        }
        Ok(())
    }
    // Draw the fog over war over the enemy pieces.
    fn draw_fog(&self, ctx: &mut Context) -> GameResult<()> {
        #[derive(Copy, Clone)]
        enum Visibility {
            Fog,
            Clear,
        }
        let mut mask = [[Visibility::Fog; 8]; 8];
        let mut mb = MeshBuilder::new();
        let (w, h) = self.cell_size(ctx);
        for Position { x, y, piece } in self.state.board.iter() {
            if let Some(Piece { player, .. }) = piece {
                if self.is_enemy(player) {
                    continue;
                }
                let (x, y) = (x as i32, y as i32);
                for (x, y) in self.line_of_sight((x, y)).into_iter().chain(vec![(x, y)]) {
                    // TODO: Better way to handle these bounds checks?
                    // 1. Let trait define valid usize.
                    // 2. Let board size be dynamic.
                    if y >= 0 && x >= 0 && y < 8 && x < 8 {
                        mask[y as usize][x as usize] = Visibility::Clear;
                    }
                }
            }
        }
        for (y, row) in mask.iter().enumerate() {
            for (x, visibility) in row.iter().enumerate() {
                if let Visibility::Fog = visibility {
                    let (x, y) = (x as f32, y as f32);
                    mb.rectangle(
                        graphics::DrawMode::fill(),
                        graphics::Rect::new(x * w, y * h, w, h),
                        graphics::Color::BLACK,
                    );
                }
            }
        }
        let mut mesh = mb.build(ctx)?;
        graphics::draw(ctx, &mut mesh, DrawParam::default())
    }
    // Draw meta information useful for debugging.
    fn draw_debug_stats(&self, ctx: &mut Context) -> GameResult<()> {
        let (text_size, padding) = (20.0, 5.0);
        let (width, height) = graphics::size(ctx);
        let (w, h) = self.cell_size(ctx);
        let stats = vec![
            format!("window: {} x {}", width, height),
            format!("  cell: {} x {}", w, h),
        ];
        for (ii, stat) in stats.iter().enumerate() {
            self.text(
                ctx,
                stat,
                (10.0, ii as f32 * text_size + padding),
                text_size,
                None,
            );
        }
        Ok(())
    }
    fn is_enemy(&self, player: &Player) -> bool {
        self.state.turn != *player
    }
    // Calculate cell size based on window size (width, height).
    fn cell_size(&self, ctx: &mut Context) -> (f32, f32) {
        let (w, h) = graphics::drawable_size(ctx);
        ((w / 8.0), (h / 8.0))
    }
    fn text(
        &self,
        ctx: &mut Context,
        text: &str,
        coord: (f32, f32),
        scale: f32,
        color: Option<Color>,
    ) {
        let fragment: graphics::TextFragment = (text, self.state.font, scale).into();
        graphics::queue_text(
            ctx,
            &Text::new(fragment),
            [coord.0, coord.1],
            color.or(Some(graphics::Color::BLACK)),
        );
    }
    // Translate pixel coordinates to grid cells.
    fn pixels_to_grid(&self, ctx: &mut Context, coord: (f32, f32)) -> (i32, i32) {
        let (x, y) = coord;
        let (w, h) = self.cell_size(ctx);
        ((x / w).floor() as i32, (y / h).floor() as i32)
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
    fn iter(&self) -> BoardIter {
        BoardIter {
            pos: None,
            board: self,
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

/// Position is a coordinate on the board, potentially containing a piece.
struct Position<'a> {
    piece: Option<&'a Piece>,
    x: usize,
    y: usize,
}

/// Iterate over a chess board, left to right.
struct BoardIter<'a> {
    pos: Option<(usize, usize)>,
    board: &'a Board,
}

impl<'a> Iterator for BoardIter<'a> {
    type Item = Position<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((x, y)) = self.pos.as_mut() {
            *x += 1;
            if *x > 7 {
                *x = 0;
                *y += 1;
            }
            if *y > 7 {
                return None;
            }
        } else {
            self.pos = Some((0, 0));
        }
        if let Some((x, y)) = self.pos {
            match self.board.0.get(y) {
                Some(cell) => match cell.get(x) {
                    Some(piece) => Some(Position {
                        piece: piece.as_ref(),
                        x,
                        y,
                    }),
                    None => None,
                },
                None => None,
            }
        } else {
            None
        }
    }
}
