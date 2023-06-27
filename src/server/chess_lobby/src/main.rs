use std::collections::HashMap;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn handle_clients(
    stream: &mut TcpStream,
    stream_two: &mut TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    stream.write_all(&[0])?;
    stream_two.write_all(&[1])?;
    Ok(())
}

fn get_id(thread: &mut TcpStream) -> u32 {
    let mut buf = [0; 4]; // u32 = 4 * u8
    thread.read_exact(&mut buf).unwrap();

    u32::from_be_bytes(buf)
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:3333").unwrap();
    println!("Server listening on port 3333");

    let mut connection_lookup: HashMap<u32, Vec<TcpStream>> = HashMap::new();
    for mut stream in listener.incoming().flatten() {
        println!("new connection: {:?}", stream.peer_addr().unwrap());
        let number = get_id(&mut stream);
        match connection_lookup.get_mut(&number) {
            Some(key) => {
                key.push(stream);

                if key.len() >= 2 {
                    let mut stream1 = key.pop().unwrap();
                    let mut stream2 = key.pop().unwrap();
                    // TODO: a dead thread can make this connection fail
                    thread::spawn(move || {
                        let _ = handle_clients(&mut stream1, &mut stream2);
                    });
                }
            }
            None => {
                connection_lookup.insert(number, vec![stream]);
            }
        }
        for key in &connection_lookup {
            println!("{}, {}", key.0, key.1.len());
        }
    }
}
