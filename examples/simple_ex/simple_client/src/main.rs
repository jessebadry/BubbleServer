use std::io;
use std::io::{Read, Write};
use std::net::TcpStream;
fn main() -> io::Result<()> {
    let mut socket = TcpStream::connect("127.0.0.1:25565")?;
    socket.write_all(b"Hello!")?;
    let mut buf = [0; 1000];
    let r = socket.read(&mut buf)?;
    let msg = std::str::from_utf8(&buf[..r]);
    if let Ok(msg) = msg {
        println!("From Server:\n{}", msg);
    } else {
        println!("Server sent invalid message!");
    }

    Ok(())
}
