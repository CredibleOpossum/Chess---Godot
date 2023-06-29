use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread::{self};

use crossbeam_channel::{unbounded, Receiver, Sender};
use godot::engine::Engine;
use godot::prelude::*;
use godot::prelude::{Base, GodotClass};

const ADDR: &str = "172.105.24.197:3333";

enum ThreadMessage {
    TryConnect(u32),
    GameStart(TcpStream),
}

fn wait_for_game(sender: &Sender<ThreadMessage>, id: u32) {
    let mut stream = TcpStream::connect(ADDR).unwrap();
    stream.write_all(&id.to_be_bytes()).unwrap();
    let mut buffer = [0_u8];
    loop {
        stream.read_exact(&mut buffer).unwrap();
        if buffer[0] == 255 {
            break;
        }
    }
    sender.send(ThreadMessage::GameStart(stream)).unwrap();
}

fn game_connection_thread(recv: Receiver<ThreadMessage>, sender: Sender<ThreadMessage>) {
    loop {
        let message = recv.recv().unwrap();
        match message {
            ThreadMessage::TryConnect(id) => wait_for_game(&sender, id),
            ThreadMessage::GameStart(stream) => {
                sender.send(ThreadMessage::GameStart(stream)).unwrap();
            }
        }
    }
}

fn wait_for_connection(id: u32, sender: Sender<ThreadMessage>) {
    wait_for_game(&sender, id)
}

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct WaitRoom {
    #[base]
    base: Base<Node2D>,
    thread_send: Option<Sender<ThreadMessage>>,
    thread_recv: Option<Receiver<ThreadMessage>>,
    stream: Option<TcpStream>,
}

#[godot_api]
impl WaitRoom {
    pub fn get_stream_ownership(&mut self) -> TcpStream {
        self.stream.take().unwrap()
    }
    #[func]
    fn try_connect(&mut self, text: GodotString) {
        let string = text.to_string();
        let num = string.parse::<u32>().unwrap();

        self.thread_send.clone().unwrap().send(ThreadMessage::TryConnect(num)).unwrap();
    }
    #[func]
    fn spawn_waiting_thread(&mut self, text: GodotString) {
        let string = text.to_string();
        let num = string.parse::<u32>().unwrap();
        let sender = self.thread_send.clone().unwrap();
        let _ = thread::spawn(move || wait_for_connection(num, sender));
    }
}

#[godot_api]
impl Node2DVirtual for WaitRoom {
    fn init(base: Base<Node2D>) -> Self {
        if Engine::singleton().is_editor_hint() {
            return WaitRoom { base, thread_send: None, thread_recv: None, stream: None };
        }
        let (client_to_server_sender, client_to_server_receiver) = unbounded();
        let (server_to_client_sender, server_to_client_receiver) = unbounded();

        let _ = thread::spawn(|| {
            game_connection_thread(client_to_server_receiver, server_to_client_sender);
        });

        WaitRoom { base, thread_send: Some(client_to_server_sender), thread_recv: Some(server_to_client_receiver), stream: None }
    }
    fn ready(&mut self) {}
    fn process(&mut self, _delta: f64) {
        if Engine::singleton().is_editor_hint() {
            return;
        }
        if let Ok(message) = self.thread_recv.clone().unwrap().try_recv() {
            match message {
                ThreadMessage::GameStart(stream) => {
                    self.stream = Some(stream);
                    self.get_tree().unwrap().change_scene_to_file("res://scenes/timeout.tscn".into());
                }
                _ => panic!("main thread got message for something not GameStart?"),
            };
        }
    }
}
