extern crate bubble_server;
use bubble_server::ServerEvent::*;
use bubble_server::*;

use std::io::{Read, Result as ResultIO, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

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
        let (socket, echo_count, _) = data;
        // retrieve shared client data (Atomic unsigned integer)
        let echo_count = echo_count.unwrap();
        handle_new_client(socket, echo_count);
    })?;

    // pause thread.
    std::thread::park(); 

    Ok(())
}

fn handle_new_client(socket: &mut TcpStream, echo_count: Arc<AtomicU64>) {
    //Send initial message
    socket
        .write_all(b"Message Number(1)")
        .expect("Error sending to server!");

    let mut buffer = [0; 3000];
    loop {
        if let Err(_) = socket.read(&mut buffer) {
            break;
        }

        let msg_string = std::str::from_utf8(&buffer).unwrap_or("Invalid Data");

        println!(
            "Received: {} from client {}",
            msg_string,
            socket.peer_addr().unwrap().to_string()
        );
        // Abort check
        if msg_string == "Invalid Data" {
            println!("the client sent invalid data, quitting.");
            return;
        };

        // Increment atomic u64
        let new_count = echo_count.load(Ordering::SeqCst) + 1;
        echo_count.store(new_count, Ordering::SeqCst);

        socket
            .write_all(format!("Message Number({})", new_count).as_bytes())
            .unwrap();

        // pause thread for 2 seconds
        std::thread::sleep(std::time::Duration::new(2, 0));
    }

    //when the function ends, the client is automatically disconnected from BubbleServer
}

fn set_disconnection_event(server: &mut BubbleServer<AtomicInteger>) {
    server.set_on_event(|event| match event {
        Disconnection(socket_addr) => {
            println!("Client '{}'  disconnected", socket_addr);
        }
        _ => {}
    });
}
