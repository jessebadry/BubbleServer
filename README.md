# BubbleServer

A multi-cliented TCP server library, where individual clients can contain custom data.

## Why make BubbleServer?

I wanted to create a general purpose client-server library that made creating multi-cliented servers easy, I also wanted to be 
able to share data across clients, and BubbleServer achieves this through generics.

I also wanted to learn good practices using Rust, and wanted to get very familiar with the borrow-checker in Rust. 

This project has helped me greatly in understanding multi-threading and concurrency, debugging dead-locks, and building network-infrastructure. 

## Room for improvement
I hope to continue developing this project post graduation, hopefully while working in a position dealing with data-communications.

For example, the next step for this project is implementing async-await where better suited.

## Echo Example

Bubble-Server doesn't include any client-side functionalities, and instead leaves the user up to it's implementation. In future, a client-side library could be built for BubbleServer, and may become standard.


### Starting a Echo server using BubbleServer! Full source code  in /examples
```rust
type AtomicInteger = Arc<AtomicU64>;

fn main() -> ResultIO<()> {
    // An echo server, where every client receives which nth echo it contributed.
    // i.e client(1) sent 1 message and receives. (echo count = 1), 
    // now client(2) sends and receives. (echo count = 2)
    let ip = "localhost:3000";
    let mut server = BubbleServer::<AtomicInteger>::new(ip);

    // Initalize shared-client data, used to count how many message echos.
    let echo_count = Arc::new(AtomicU64::new(1));
    // Set shared data
    server.set_hc_param(echo_count);
    set_disconnection_event(&mut server);

    println!("Listening for clients on '{}'", ip);
    // Starts the server, *non-blocking
    server.start(|data| {
        // Makes new thread for this newly connected client.

        // get socket, shared_data, and the server reference from data, (server ref is unused)
        let (socket, echo_count, _server_ref) = data;

        // retrieve shared client data (Atomic unsigned integer)
        let echo_count = echo_count.unwrap();

        // handle client in some custom implementation with sockets
        handle_new_client(socket, echo_count); // blocking function

        // after this closure finishes, BubbleServer will automatically delete and cleanup this client
    })?;

    // pause thread.
    std::thread::park(); 

    Ok(())
}
```

### echo_client.rs
```rust
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
```
