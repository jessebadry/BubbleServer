use std::io::{Read, Result as IOResult, Write};
use std::net::TcpStream;

fn main() -> IOResult<()> {
    let mut stream = TcpStream::connect("localhost:3000")
        .expect("Could not connect to server. Did you run the server-example?");

    loop {
        let mut buf = [0; 300];
        let read = stream.read(&mut buf)?;
        if read == 0 {
            break Ok(());
        }

        // Convert data to string
        if let Ok(msg) = std::str::from_utf8(&buf[..read]) {
            println!("From Server: {}", msg);
            let new_msg = format!("{}", msg);
            stream.write_all(new_msg.as_bytes())?;
        } else {
            println!("Non-Utf8 data received!");
        };
    }
}
