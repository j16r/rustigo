use conv::TryFrom;
use std::collections::btree_map::BTreeMap;
use std::collections::HashSet;
use std::fmt;
use std::iter::Iterator;

pub type Coordinate = (i8, i8);

#[derive(Eq, PartialEq, Debug, Copy, Serialize, Deserialize, Clone)]
pub enum Stone {
    Black,
    White,
}

custom_derive! {
    #[derive(Debug, Copy, Clone, Eq, PartialEq, TryFrom(usize))]
    pub enum Size {
        Nine = 9,
        Thirteen = 13,
        Seventeen = 17,
        Nineteen = 19
    }
}

type StoneMap = BTreeMap<Coordinate, Stone>;

pub struct Game {
    id: u64,
    board: StoneMap,
    size: Size,
    turn: Stone,
}

pub fn new(size: Size) -> Game {
    Game {
        id: 0,
        board: BTreeMap::new(),
        size,
        turn: Stone::Black,
    }
}

// parse creates a new game from a simple human readable string representation.
pub fn parse(board_str: &str, turn: Stone) -> Option<Game> {
    let mut board = BTreeMap::new();
    let lines: Vec<&str> = board_str.trim().split('\n').collect();

    if lines.len() < (Size::Nine as usize) || lines.len() > (Size::Nineteen as usize) {
        return None;
    }

    let line_length = lines[0].trim().len();
    if line_length != lines.len() {
        return None;
    }

    let size_result = <Size as TryFrom<_>>::try_from(line_length);
    let size = match size_result {
        Ok(size) => size,
        _ => return None,
    };

    for (y, line) in lines.iter().enumerate() {
        for (x, tile) in line.chars().enumerate() {
            match tile {
                'b' => {
                    board.insert((x as i8, y as i8), Stone::Black);
                }
                'w' => {
                    board.insert((x as i8, y as i8), Stone::White);
                }
                _ => (),
            }
        }
    }

    Some(Game {
        id: 0,
        board,
        size,
        turn,
    })
}

// decode reads in the wire transfer format of the game.
pub fn decode(game_str: &str) -> Option<Game> {
    let segments: Vec<&str> = game_str.trim().split(';').collect();

    let size_value = match segments[0].parse::<usize>() {
        Ok(size_value) => size_value,
        _ => return None,
    };
    let size = match <Size as TryFrom<_>>::try_from(size_value) {
        Ok(size) => size,
        _ => return None,
    };

    let mut board = BTreeMap::new();
    for (index, tile) in segments[1].chars().enumerate() {
        let x = index % size_value;
        let y = index / size_value;
        match tile {
            'b' => {
                board.insert((x as i8, y as i8), Stone::Black);
            }
            'w' => {
                board.insert((x as i8, y as i8), Stone::White);
            }
            _ => (),
        };
    }

    let turn = match segments[2] {
        "b" => Stone::Black,
        "w" => Stone::White,
        _ => return None,
    };

    Some(Game {
        id: 0,
        board,
        size,
        turn,
    })
}

// encode produces a tightly packed ASCII safe representation of a game that can be shipped over
// the wire safely.
pub fn encode(game: &Game) -> String {
    let mut output = "".to_string();

    let extent = game.size as i8;
    output.push_str(&*format!("{};", extent));

    for row in 0..extent {
        for column in 0..extent {
            output.push_str(match game.board.get(&(column, row)) {
                Some(&Stone::Black) => "b",
                Some(&Stone::White) => "w",
                None => ".",
            });
        }
    }
    output.push(';');
    match game.turn {
        Stone::Black => output.push('b'),
        Stone::White => output.push('w'),
    };
    output
}

impl Game {
    pub fn turn(&self) -> Stone {
        self.turn
    }

    // valid_coordinate determines if a coordinate is within the bounds of the game board.
    fn valid_coordinate(&self, (x, y): Coordinate) -> bool {
        let extent = self.size as i8;
        x >= 0 && x < extent && y >= 0 && y < extent
    }

    // adjacent_positions returns a list of valid adjacent coordinates.
    //
    // e.g:
    //  (0, 0) => [(1, 0), (0, 1]
    fn adjacent_positions(&self, (x, y): Coordinate) -> Vec<Coordinate> {
        [(x, y - 1), (x + 1, y), (x, y + 1), (x - 1, y)]
            .iter()
            .filter(|coordinate| self.valid_coordinate(**coordinate))
            .cloned()
            .collect()
    }

    // can_play tests if a position is valid and the tile is empty, it DOES NOT check for allies
    // with liberties or foes without.
    fn can_play(&self, position: Coordinate, stone: Stone) -> bool {
        self.turn == stone && self.valid_coordinate(position) && !self.has_stone(position)
    }

    // advance_turn sets the game state so that it's the next player's turn.
    fn advance_turn(&mut self) {
        self.turn = self.foe(self.turn)
    }

    // foe returns the enemy stone of the specified stone
    fn foe(&self, stone: Stone) -> Stone {
        match stone {
            Stone::Black => Stone::White,
            Stone::White => Stone::Black,
        }
    }

    // remove_chain removes all pieces in a chain from the board.
    fn remove_chain(&mut self, chain: &[Coordinate]) {
        for position in chain.iter() {
            self.board.remove(position);
        }
    }

    // attack returns a chain at `to` being attacked by `from` if it has no liberties
    fn attack(&self, from: Coordinate, to: Coordinate, stone: Stone) -> Option<Vec<Coordinate>> {
        let mut chain = vec![to];

        let mut searched_tiles = HashSet::<Coordinate>::new();
        searched_tiles.insert(from);
        searched_tiles.insert(to);

        let mut positions_to_search = vec![to];

        while let Some(position) = positions_to_search.pop() {
            for search_position in self.adjacent_positions(position) {
                if !searched_tiles.contains(&search_position) {
                    match self.board.get(&search_position) {
                        Some(tile) if *tile != stone => {
                            positions_to_search.push(search_position);
                            chain.push(search_position);
                        }
                        Some(_) => {}
                        None => {
                            // Found an empty tile near this chain, it's safe!
                            return None;
                        }
                    }
                }
            }

            searched_tiles.insert(position);
        }

        Some(chain)
    }

    // allie_has_liberty returns true if the chain attached to proposed (indicated by allie) has a
    // liberty.
    fn allie_has_liberty(&self, proposed: Coordinate, allie: Coordinate, stone: Stone) -> bool {
        let mut searched_tiles = HashSet::<Coordinate>::new();
        searched_tiles.insert(proposed);
        searched_tiles.insert(allie);

        let mut positions_to_search = vec![allie];

        while let Some(position) = positions_to_search.pop() {
            for search_position in self.adjacent_positions(position) {
                if !searched_tiles.contains(&search_position) {
                    match self.board.get(&search_position) {
                        Some(tile) if *tile == stone => {
                            positions_to_search.push(search_position);
                        }
                        Some(_) => {}
                        None => {
                            // Found an empty tile near this chain, it's safe!
                            return true;
                        }
                    }
                }
            }

            searched_tiles.insert(position);
        }

        false
    }

    // play_stone places a stone on the board, capturing any defending stones without any
    // liberties. Returns false if the play is invalid, true otherwise.
    pub fn play_stone(&mut self, position: Coordinate, stone: Stone) -> bool {
        if !self.can_play(position, stone) {
            return false;
        }

        let mut safe = false;
        let mut routed_defenders = Vec::<Vec<Coordinate>>::new();

        for neighbour in self.adjacent_positions(position) {
            match self.board.get(&neighbour) {
                Some(tile) if tile == &stone => {
                    if !safe {
                        // safe has not yet been toggled to true, search for a liberty through this
                        // adjacent chain
                        safe = self.allie_has_liberty(position, neighbour, stone);
                    }
                }
                Some(_) => {
                    if let Some(chain) = self.attack(position, neighbour, stone) {
                        routed_defenders.push(chain);
                        safe = true;
                    }
                }
                // found a free adjacent tile, tile is safe to place
                None => {
                    safe = true;
                }
            }
        }

        if safe {
            for defending_chain in routed_defenders.iter() {
                self.remove_chain(defending_chain);
            }

            self.board.insert(position, stone);
            self.advance_turn();
            return true;
        }

        false
    }

    pub fn has_stone(&self, position: Coordinate) -> bool {
        self.board.contains_key(&position)
    }

    pub fn stones(&self) -> usize {
        self.board.len()
    }

    pub fn player_stones(&self, stone: Stone) -> usize {
        self.board
            .iter()
            .filter(|&(_, piece)| *piece == stone)
            .count()
    }

    //pub fn player_score(&self, stone: Stone) -> usize {
    //self.board.iter().filter(|&(_, piece)| *piece == stone).count()
    //}

    pub fn winner(&self) -> Stone {
        Stone::Black
    }
}

impl fmt::Debug for Game {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let extent = self.size as i8;
        formatter.write_str("\n\n")?;
        for row in 0..extent {
            for column in 0..extent {
                formatter.write_str(match self.board.get(&(column, row)) {
                    Some(&Stone::Black) => "b",
                    Some(&Stone::White) => "w",
                    None => ".",
                })?
            }
            formatter.write_str("\n")?;
        }
        formatter.write_str("\n")
    }
}

#[test]
fn test_new() {
    let game = new(Size::Nine);
    assert_eq!(game.id, 0);
    assert_eq!(Stone::Black, game.turn());
    assert_eq!(false, game.has_stone((0, 0)));
}

#[test]
fn test_play_stone() {
    let mut game = new(Size::Nine);
    assert_eq!(false, game.has_stone((0, 0)));

    game.play_stone((0, 0), Stone::Black);
    assert_eq!(true, game.has_stone((0, 0)));
    assert_eq!(1, game.stones());
    assert_eq!(1, game.player_stones(Stone::Black));
    assert_eq!(0, game.player_stones(Stone::White));
}

#[test]
fn test_play_stone_switches_players() {
    let mut game = new(Size::Nine);
    assert_eq!(true, game.play_stone((0, 0), Stone::Black));
    assert_eq!(false, game.play_stone((1, 0), Stone::Black));
}

#[test]
fn test_play_stone_rejects_invalid_plays() {
    let mut game = new(Size::Nine);
    assert_eq!(false, game.play_stone((-1, 0), Stone::Black));
    assert_eq!(false, game.play_stone((-1, -1), Stone::Black));
    assert_eq!(false, game.play_stone((0, -1), Stone::Black));
    assert_eq!(false, game.play_stone((9, 0), Stone::Black));
    assert_eq!(false, game.play_stone((9, 9), Stone::Black));
    assert_eq!(false, game.play_stone((0, 9), Stone::Black));
}

#[test]
fn test_play_stone_rejects_duplicate_plays() {
    let mut game = new(Size::Nine);
    assert_eq!(true, game.play_stone((0, 0), Stone::Black));
    assert_eq!(false, game.play_stone((0, 0), Stone::White));
}

#[test]
fn test_parse() {
    let game = parse(
        "
.b.......
b........
.........
.........
.........
.........
.........
.........
.........",
        Stone::Black,
    )
    .unwrap();

    assert_eq!(false, game.has_stone((0, 0)));
    assert_eq!(true, game.has_stone((1, 0)));
    assert_eq!(true, game.has_stone((0, 1)));
    assert_eq!(false, game.has_stone((1, 1)));
}

#[test]
fn test_play_stone_no_liberties() {
    let mut game = parse(
        "
.b.....b.
b.bbbbbb.
.b....bbb
..bbbbb..
.....b...
.........
.........
b.......b
.b.....b.",
        Stone::White,
    )
    .unwrap();

    // Top left corner
    assert_eq!(false, game.play_stone((0, 0), Stone::White));

    // Surrounded stone
    assert_eq!(false, game.play_stone((1, 1), Stone::White));

    // Bottom right corner
    assert_eq!(false, game.play_stone((8, 8), Stone::White));

    // Bottom left corner
    assert_eq!(false, game.play_stone((0, 8), Stone::White));

    // Top right corner
    assert_eq!(true, game.play_stone((8, 0), Stone::White));
    assert_eq!(false, game.play_stone((8, 1), Stone::White));
}

#[test]
fn test_play_stone_empty_neighbour() {
    let mut game = parse(
        "
.b.....b.
b.bbbbbb.
.b....bbb
..bbb.b..
.....b...
.........
.........
b.......b
.b.....b.",
        Stone::White,
    )
    .unwrap();

    assert_eq!(true, game.play_stone((2, 0), Stone::White));
}

#[test]
fn test_play_stone_capture_piece() {
    let mut game = parse(
        "
.........
bwb......
.b.......
.........
.........
.........
.........
.........
.........",
        Stone::Black,
    )
    .unwrap();

    assert_eq!(true, game.play_stone((1, 0), Stone::Black));
    assert_eq!(false, game.has_stone((1, 1)));
    assert_eq!(Stone::Black, game.winner());
}

#[test]
fn test_play_stone_capture_piece_exchange() {
    let mut game = parse(
        "
.bw......
bw.w.....
.bw......
.........
.........
.........
.........
.........
.........",
        Stone::Black,
    )
    .unwrap();

    assert_eq!(true, game.play_stone((2, 1), Stone::Black));
    assert_eq!(false, game.has_stone((1, 1)));
    assert_eq!(Stone::Black, game.winner());
}

#[test]
fn test_play_stone_cannot_place_neighbour_has_no_liberties() {
    let mut game = parse(
        "
bb.w.....
www......
.........
.........
.........
.........
.........
.........
.........",
        Stone::Black,
    )
    .unwrap();

    assert_eq!(false, game.play_stone((2, 0), Stone::Black));
}
