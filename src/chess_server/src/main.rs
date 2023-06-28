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

    let mut buffer = [0u8];
    loop {
        stream.read_exact(&mut buffer)?;
        stream_two.write_all(&buffer)?;
        stream_two.read_exact(&mut buffer)?;
        stream.write_all(&buffer)?;
    }
}

fn get_id(thread: &mut TcpStream) -> Option<u32> {
    let mut buf = [0; 4]; // u32 = 4 * u8
    match thread.read_exact(&mut buf) {
        Ok(_) => Some(u32::from_be_bytes(buf)),
        Err(_) => None,
    }
}

fn health_check(stream: &mut TcpStream) -> bool {
    // This function could possibly hang for quite a awhile,
    // TODO: add timeout
    if stream.write_all(&[0]).is_ok() {
        return true;
    }
    false
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:3333").unwrap();
    println!("Server listening on port 3333");

    let mut connection_lookup: HashMap<u32, Vec<TcpStream>> = HashMap::new();
    for mut stream in listener.incoming().flatten() {
        let possible_id = get_id(&mut stream);
        if let Some(number) = possible_id {
            match connection_lookup.get_mut(&number) {
                Some(key) => {
                    key.push(stream);

                    if key.len() >= 2 {
                        let mut stream1 = key.pop().unwrap();
                        if !health_check(&mut stream1) {
                            continue;
                        }
                        let mut stream2 = key.pop().unwrap();
                        if !health_check(&mut stream2) {
                            continue;
                        }

                        // Both the streams survived the life check
                        // lets let them know they found someone
                        stream1.write_all(&[255]).unwrap();
                        stream2.write_all(&[255]).unwrap();
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
}
