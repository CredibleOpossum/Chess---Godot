use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::OnceLock;
use std::thread::{self};

use crossbeam_channel::{unbounded, Receiver, Sender};
use godot::engine::Engine;
use godot::prelude::*;
use godot::prelude::{Base, GodotClass};

const ADDR: &str = "127.0.0.1:3333";

#[derive(Debug, PartialEq)]
enum ThreadMessage {
    TryConnect(u32),
    GameStart,
}

static SENDER: OnceLock<Sender<ThreadMessage>> = OnceLock::new();

fn game_connection_thread(recv: Receiver<ThreadMessage>, send: Sender<ThreadMessage>) {
    loop {
        let message = recv.recv().unwrap();
        godot_print!("somehow, somehow, it worked? {:?}", message);
        match message {
            ThreadMessage::TryConnect(id) => {
                let mut stream = TcpStream::connect(ADDR).unwrap();
                stream.write_all(&id.to_be_bytes()).unwrap();
                let mut buffer = [0_u8];
                stream.read_exact(&mut buffer).unwrap();
                send.send(ThreadMessage::GameStart).unwrap();
            }
            ThreadMessage::GameStart => {
                send.send(ThreadMessage::GameStart).unwrap();
            }
        }
    }
}

fn wait_for_connection(num: u32) {
    let mut stream = TcpStream::connect(ADDR).unwrap();
    stream.write_all(&num.to_be_bytes()).unwrap();
    let mut buffer = [0_u8];
    stream.read_exact(&mut buffer).unwrap();
    {
        let sender = SENDER.get().unwrap();
        sender.send(ThreadMessage::GameStart).unwrap();
    }
}

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct WaitRoom {
    #[base]
    base: Base<Node2D>,
    thread_recv: Option<Receiver<ThreadMessage>>,
}

#[godot_api]
impl WaitRoom {
    #[func]
    fn try_connect(&mut self, text: GodotString) {
        let string = text.to_string();
        let num = string.parse::<u32>().unwrap();

        // So this is pretty horrible, the reason we do this is because this function 'try_connect' has no
        // context to use to actually obtain (WaitRoom) the sender unless it's somehow global and thread-safe
        // luckily once-lock makes this possible so until I find a better way of doing this, this is going to be the solution.
        {
            let sender = SENDER.get().unwrap();
            sender.send(ThreadMessage::TryConnect(num)).unwrap();
        }
    }
    #[func]
    fn spawn_waiting_thread(&mut self, text: GodotString) {
        let string = text.to_string();
        let num = string.parse::<u32>().unwrap();
        let _ = thread::spawn(move || wait_for_connection(num));
    }
}

#[godot_api]
impl Node2DVirtual for WaitRoom {
    fn init(base: Base<Node2D>) -> Self {
        if Engine::singleton().is_editor_hint() {
            return WaitRoom { base, thread_recv: None };
        }
        let (client_to_server_sender, client_to_server_receiver) = unbounded();
        let (server_to_client_sender, server_to_client_receiver) = unbounded();

        SENDER.get_or_init(|| client_to_server_sender);

        let _ = thread::spawn(|| {
            game_connection_thread(client_to_server_receiver, server_to_client_sender);
        });

        WaitRoom { base, thread_recv: Some(server_to_client_receiver) }
    }
    fn ready(&mut self) {}
    fn process(&mut self, _delta: f64) {
        if Engine::singleton().is_editor_hint() {
            return;
        }
        if let Ok(message) = self.thread_recv.clone().unwrap().try_recv() {
            if message == ThreadMessage::GameStart {
                self.get_tree().unwrap().change_scene_to_file("res://scenes/debug_scene.tscn".into());
            }
        }
    }
}
