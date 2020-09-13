extern crate bubble_server;
use bubble_server::bubble_host::{BubbleServer, ServerEvent};
use std::io::{Read, Write};
use std::net::TcpStream;
fn make_invalid_socket_msg(socket: &TcpStream) -> String {
    let sock_addr = socket.peer_addr().unwrap().to_string();

    format!("{} sent an invalid msg!", sock_addr)
}

fn main() {
    let mut server = BubbleServer::<()>::new("127.0.0.1:25565".into());

    //Start the server, pass in a closure/function that will handle each socket that connects
    //to the server.
    server
        .start(|client_data| {
            //unpack HandleClientType data
            let (socket, _, _server) = client_data;

            //now we can use the socket like it's any other Rust socket!

            //get the socket's address then convert to String.
            let socket_addr = socket.peer_addr().unwrap().to_string();

            let mut msg_buf = vec![0; 100];

            let r = socket.read(&mut msg_buf).unwrap_or(0);
            if r == 0 {
                println!("Nothing read from socket, returning");
                return;
            }

            let msg = String::from_utf8((&msg_buf[..r]).to_vec())
                .unwrap_or(make_invalid_socket_msg(&socket));

            socket
                .write_all(format!("Message received has been processed as '{}'", msg).as_bytes())
                .unwrap_or_else(|e| {
                    println!(
                        "Error couldn't send message to '{}'! Why: {}",
                        socket_addr, e
                    )
                });
            //End of handle so the socket will be closed/disposed of.
            println!("Leaving handle thread, which will close socket!");
        })
        .unwrap_or_else(|why| {
            println!("Could not start server, why: {}", why);
        });
    server.set_on_event(|event| {
        if let ServerEvent::Disconnection(sock) = event {
            println!("sock disconnected: {}", sock);
        }
    });

    //Park the thread as `start` is non-blocking!!!
    std::thread::park();
}
