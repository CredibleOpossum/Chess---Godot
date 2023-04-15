use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

fn handle_client(
    stream: &mut TcpStream,
    stream_two: &mut TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    stream.write(&[0])?;
    stream_two.write(&[1])?;

    let mut buffer = [0u8];
    loop {
        stream.read(&mut buffer)?;
        stream_two.write(&buffer)?;
        stream_two.read(&mut buffer)?;
        stream.write(&buffer)?;
    }
}

fn health_check(stream: &mut TcpStream) -> bool {
    let check_byte: u8 = 255;
    stream.write_all(&[check_byte]).unwrap();

    let timeout = Duration::from_secs(5);

    let mut buf = [0u8; 1];
    let start_time = Instant::now();
    while start_time.elapsed() < timeout {
        if let Ok(_) = stream.read_exact(&mut buf) {
            if buf[0] == check_byte {
                return true;
            }
        }
    }

    false
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:3333").unwrap();
    println!("Server listening on port 3333");

    let mut streams: Vec<TcpStream> = Vec::new();
    for stream in listener.incoming() {
        if stream.is_ok() {
            let stream = stream.unwrap();
            println!("new connection: {:?}", stream.peer_addr().unwrap());
            streams.push(stream);
        }

        let mut i = 0;
        while i < streams.len() {
            if !health_check(&mut streams[i]) {
                streams.remove(i);
            } else {
                i += 1;
            }
        }

        if streams.len() >= 2 {
            let mut first_stream = streams.pop().unwrap();
            let mut second_stream = streams.pop().unwrap();
            thread::spawn(move || {
                _ = handle_client(&mut first_stream, &mut second_stream);
            });
        }
    }
    drop(listener);
}
