use godot::engine::{AudioStreamPlayer2D, Engine, Label, Node2D, Node2DVirtual, Sprite2D};
use godot::engine::{TextureRect, TileMap};
use godot::prelude::*;
use std::collections::HashMap;
use std::convert::From;
use std::io::{Read, Write};
use std::mem::discriminant;
use std::net::TcpStream;
use std::thread;

use crossbeam_channel::{unbounded, Receiver, Sender};

use yacp::*;

const BOARD_SIZE: i32 = 8;
const NO_VECT: Vector2i = Vector2i { x: 0, y: 0 };
const NO_LOCATION: Location = Location { x: 0, y: 0 };
const BOARD_TILE: i64 = 16;
const STARTING_POSITION_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 yy1";

enum ThreadMessage {
    ChessMove(u8),
    Color(ChessColor),
}

#[derive(Copy, Clone, PartialEq)]
pub enum Layers {
    //BoardLight = 0,
    //BoardDark = 1,
    Origin = 2,
    Destination = 3,
    Select = 4,
    Check = 5,
    Pieces = 6,
}

fn wait_to_send_move(stream: &mut TcpStream, move_getter: &Receiver<ThreadMessage>) {
    for received_move in move_getter {
        if let ThreadMessage::ChessMove(index) = received_move {
            stream.write_all(&[index]).unwrap();
            break; // Exit the loop after processing one element
        }
    }
}

fn get_move(stream: &mut TcpStream, move_sender: &Sender<ThreadMessage>) {
    let mut buffer = vec![0u8];
    stream.read_exact(&mut buffer).unwrap();
    move_sender.send(ThreadMessage::ChessMove(buffer[0])).unwrap();
}

fn networking(move_getter: &Receiver<ThreadMessage>, move_sender: &Sender<ThreadMessage>) {
    match TcpStream::connect("127.0.0.1:3457") {
        Ok(mut stream) => {
            let mut buffer = vec![0u8];
            loop {
                stream.read_exact(&mut buffer).unwrap();
                if buffer[0] != 255 {
                    break;
                }
                stream.write_all(&buffer).unwrap();
            }
            let chess_color = [ChessColor::White, ChessColor::Black][buffer[0] as usize];
            move_sender.send(ThreadMessage::Color(chess_color)).unwrap();
            if chess_color == ChessColor::White {
                loop {
                    wait_to_send_move(&mut stream, move_getter);
                    get_move(&mut stream, move_sender);
                }
            } else {
                loop {
                    get_move(&mut stream, move_sender);
                    wait_to_send_move(&mut stream, move_getter);
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    }
}

fn get_piece_id(piece: ChessPiece) -> i32 {
    let piece_ids = HashMap::from([(discriminant(&ChessPiece::None), -1), (discriminant(&ChessPiece::Pawn(ChessColor::None)), 3), (discriminant(&ChessPiece::Knight(ChessColor::None)), 2), (discriminant(&ChessPiece::Bishop(ChessColor::None)), 0), (discriminant(&ChessPiece::Rook(ChessColor::None, RookType::Unknown)), 5), (discriminant(&ChessPiece::Queen(ChessColor::None)), 4), (discriminant(&ChessPiece::King(ChessColor::None)), 1)]);
    let color = get_piece_color(piece);
    *piece_ids.get(&discriminant(&piece)).unwrap() + ((color == ChessColor::White) as usize * 10) as i32
}

fn usize_to_vector2i(position: usize) -> Vector2i {
    Vector2i { x: position as i32 % BOARD_SIZE, y: position as i32 / BOARD_SIZE }
}

fn location_to_vector2i(position: Location) -> Vector2i {
    Vector2i { x: position.x, y: position.y }
}

fn vector2i_to_location(position: Vector2i) -> Location {
    Location { x: position.x, y: position.y }
}

fn invert_usize(position: usize) -> usize {
    ((BOARD_SIZE * BOARD_SIZE - 1) - position as i32) as usize
}

fn invert_location(position: Location) -> Location {
    Location { x: 7 - position.x, y: 7 - position.y }
}

fn update_board(position: &ChessPosition, tilemap: &mut Gd<TileMap>, invert_board: bool) {
    for i in 0..(BOARD_SIZE * BOARD_SIZE) {
        let location = if invert_board { usize_to_vector2i(invert_usize(i as usize)) } else { usize_to_vector2i(i as usize) };
        tilemap.set_cell(Layers::Pieces as i64, location, get_piece_id(position.pieces[i as usize]).into(), NO_VECT, 0);
    }
}

fn play_sound(board: &mut ChessBoard, move_type: MoveType) {
    match move_type {
        MoveType::Relocation | MoveType::Castle | MoveType::LongMove => {
            board.get_node_as::<AudioStreamPlayer2D>("move_sound").play(0.0);
        }
        MoveType::Capture | MoveType::EnPassant => {
            board.get_node_as::<AudioStreamPlayer2D>("capture_sound").play(0.0);
        }
        _ => (),
    }
}

fn handle_input(board: &mut ChessBoard, board_tilemap: &mut Gd<TileMap>, thread_send: &Sender<ThreadMessage>) {
    let mut click_position = vector2i_to_location(board_tilemap.local_to_map(board.get_local_mouse_position()));

    if board.invert_board {
        click_position = invert_location(click_position);
    }

    if vaild_position(click_position) {
        let piece_is_mine = get_piece_color(board.position.get_piece(click_position)) == board.position.turn;
        let clicked_on_same_piece = click_position == board.held_piece;
        match (board.is_holding_piece, clicked_on_same_piece, piece_is_mine) {
            (true, true, _) => {
                board.is_holding_piece = false;
            }
            (_, _, true) => {
                board.held_piece = click_position;
                board.is_holding_piece = true;
            }
            (true, _, _) => {
                let chess_move = board.position.get_move_data(board.held_piece, click_position);

                if chess_move.is_legal {
                    play_sound(board, chess_move.move_type);
                    let move_index = board.position.get_move_index(chess_move).unwrap();
                    thread_send.send(ThreadMessage::ChessMove(move_index)).unwrap();
                    board.position.make_chess_move_from_index(move_index);

                    board.last_move_origin = Some(chess_move.origin);
                    board.last_move_destination = Some(chess_move.destination);
                } else {
                    board.is_holding_piece = false;
                }
            }
            _ => board.is_holding_piece = false,
        };
    }
}

#[derive(GodotClass, Debug)]
#[class(base=Node2D)]
pub struct ChessBoard {
    #[base]
    base: Base<Node2D>,
    position: ChessPosition,
    player_color: ChessColor,
    is_holding_piece: bool,
    held_piece: Location,
    last_move_origin: Option<Location>,
    last_move_destination: Option<Location>,
    invert_board: bool,
    thread_send: Option<Sender<ThreadMessage>>,
    thread_recv: Option<Receiver<ThreadMessage>>,
}

fn draw_move_tiles(board: &mut ChessBoard, board_tilemap: &mut TileMap) {
    if board.last_move_destination.is_some() {
        board_tilemap.clear_layer(Layers::Origin as i64);
        board_tilemap.clear_layer(Layers::Destination as i64);
        let origin_location;
        let destination_location;
        if !board.invert_board {
            origin_location = location_to_vector2i(board.last_move_origin.unwrap());
            destination_location = location_to_vector2i(board.last_move_destination.unwrap());
        } else {
            origin_location = location_to_vector2i(invert_location(board.last_move_origin.unwrap()));
            destination_location = location_to_vector2i(invert_location(board.last_move_destination.unwrap()))
        }
        board_tilemap.set_cell(Layers::Origin as i64, origin_location, BOARD_TILE, NO_VECT, 0);
        board_tilemap.set_cell(Layers::Destination as i64, destination_location, BOARD_TILE, NO_VECT, 0);
    }
}

#[godot_api]
impl Node2DVirtual for ChessBoard {
    fn init(base: Base<Node2D>) -> Self {
        if Engine::singleton().is_editor_hint() {
            return ChessBoard { base, position: fen_parser(STARTING_POSITION_FEN).unwrap(), player_color: ChessColor::None, is_holding_piece: false, held_piece: NO_LOCATION, last_move_origin: None, last_move_destination: None, invert_board: false, thread_send: None, thread_recv: None };
        }
        let (client_to_server_sender, client_to_server_receiver) = unbounded();
        let (server_to_client_sender, server_to_client_receiver) = unbounded();

        let _thread = thread::spawn(move || {
            networking(&server_to_client_receiver, &client_to_server_sender);
        });

        ChessBoard {
            base,
            position: fen_parser(STARTING_POSITION_FEN).unwrap(),
            player_color: ChessColor::None,
            is_holding_piece: false,
            held_piece: NO_LOCATION,
            last_move_origin: None,
            last_move_destination: None,
            invert_board: false,
            thread_send: Some(server_to_client_sender),
            thread_recv: Some(client_to_server_receiver),
        }
    }
    fn ready(&mut self) {
        // Stop execution if running in the editor
        if Engine::singleton().is_editor_hint() {
            return;
        }
        let mut board_tilemap = self.base.get_node_as::<TileMap>("board_tilemap");
        update_board(&self.position, &mut board_tilemap, self.invert_board);
    }
    fn process(&mut self, _delta: f64) {
        // Stop execution if running in the editor
        if Engine::singleton().is_editor_hint() {
            return;
        }

        let mut board_tilemap = self.base.get_node_as::<TileMap>("board_tilemap");
        // If game is over, don't continue execution
        if self.position.get_board_state() == BoardState::Checkmate {
            self.get_node_as::<Label>("win_window/Label").set_text(format!("{:?} won by checkmate!", get_opposite_color(self.position.turn)).into());

            self.get_node_as::<Sprite2D>("win_window").set_visible(true);
            return;
        }
        if self.position.get_board_state() == BoardState::Checkmate {
            self.get_node_as::<Label>("win_window/Label").set_text("Stalemate!".into());

            self.get_node_as::<Sprite2D>("win_window").set_visible(true);
            return;
        }
        let input = Input::singleton();
        if self.position.turn == self.player_color && input.is_action_just_pressed("click_piece".into(), false) {
            handle_input(self, &mut board_tilemap, &self.thread_send.clone().unwrap());
        }
        for message in &self.thread_recv.clone().unwrap().try_recv() {
            match message {
                ThreadMessage::ChessMove(index) => {
                    let move_data = self.position.make_chess_move_from_index(*index);
                    self.last_move_origin = Some(move_data.origin);
                    self.last_move_destination = Some(move_data.destination);
                    play_sound(self, move_data.move_type);
                }
                ThreadMessage::Color(color) => {
                    self.player_color = *color;
                    self.invert_board = match color {
                        ChessColor::White => false,
                        ChessColor::Black => true,
                        ChessColor::None => panic!("Player's color is None? Shouldn't be possible"),
                    };
                    let mut cover = self.get_node_as::<TextureRect>("cover_window");
                    cover.set_visible(false);
                }
            }
        }

        board_tilemap.clear_layer(Layers::Check as i64);
        if self.position.is_in_check(self.position.turn) {
            let king_position = self.position.find_king(self.position.turn);
            if !self.invert_board {
                let location = location_to_vector2i(king_position.unwrap());
                board_tilemap.set_cell(Layers::Check as i64, location, BOARD_TILE, NO_VECT, 0);
            } else {
                board_tilemap.set_cell(Layers::Check as i64, location_to_vector2i(invert_location(king_position.unwrap())), BOARD_TILE, NO_VECT, 0);
            }
        }

        board_tilemap.clear_layer(Layers::Select as i64);
        if self.is_holding_piece {
            for possible_move in self.position.get_piece_moves(self.held_piece, self.position.turn, true) {
                if self.position.get_move_data(possible_move.origin, possible_move.destination).is_legal {
                    let location = if self.invert_board { location_to_vector2i(invert_location(possible_move.destination)) } else { location_to_vector2i(possible_move.destination) };
                    board_tilemap.set_cell(Layers::Select as i64, location, BOARD_TILE, NO_VECT, 0);

                    board_tilemap.set_cell(Layers::Origin as i64, location, -1, NO_VECT, 0);
                    board_tilemap.set_cell(Layers::Destination as i64, location, -1, NO_VECT, 0);
                }
            }
        }
        update_board(&self.position, &mut board_tilemap, self.invert_board);
        draw_move_tiles(self, &mut board_tilemap);
    }
}
