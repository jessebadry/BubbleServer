extern crate bubble_server;
use crate::bubble_server::bubble_host::{BubbleServer, HandleClientType, ServerEvent};
use futures::executor::block_on;
use std::io;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::net::TcpStream;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
///Example implementation of BubbleServer
trait TextChatServer {
    fn broad_cast_msg(&mut self, msg: &[u8]) -> io::Result<()>;
}
impl TextChatServer for BubbleServer<Sender<String>> {
    fn broad_cast_msg(&mut self, msg: &[u8]) -> Result<(), std::io::Error> {
        let clients = self.get_clients()?;
        let clients = clients
            .lock()
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        async fn async_write_all(mut sock: &TcpStream, data: &[u8]) -> io::Result<()> {
            sock.write_all(data)
        }
        block_on(async {
            for client in clients.iter() {
                let mut client = client;
                let result = async_write_all(&mut client, msg).await;
                if let Err(why) = result {
                    println!(
                        "Error happened with ip {}, why: {}",
                        client.peer_addr().unwrap().to_string(),
                        why,
                    )
                }
            }
        });
        Ok(())
    }
}

trait TcpHelper {
    fn send_str(&mut self, msg: &str) -> io::Result<()>;
}
impl TcpHelper for TcpStream {
    fn send_str(&mut self, msg: &str) -> io::Result<()> {
        let data = msg.as_bytes();
        self.write_all(&data)?;
        Ok(())
    }
}

fn read_stream(stream: &mut TcpStream, recv_buf: &mut [u8], read_bytes: &mut usize) -> usize {
    *read_bytes = match stream.read(recv_buf) {
        Ok(read_bytes) => read_bytes,
        Err(_) => 0,
    };
    *read_bytes
}
//This Function will be called and ran in a new thread every time a new user connects.
fn handle_new_text_client(client_data: HandleClientType<Sender<String>>) {
    // HandleClientType provides the stream that is being handled and the server it's being called from so the user could access functions from the server
    let buffer_size = 1000;
    let (stream, param, server_reference) = client_data;
    println!(
        "Client connected!! ip {}",
        stream.peer_addr().unwrap().to_string()
    );
    if let Some(sender) = param {
        sender
            .send(format!(
                "Started new client.. socket: {}",
                stream.peer_addr().unwrap().to_string()
            ))
            .unwrap();
    }

    stream
        .send_str(&stream.peer_addr().unwrap().to_string())
        .unwrap_or_else(|err| {
            println!(
                "err writing ip to stream, ip {}, err :{}",
                &stream.peer_addr().unwrap().to_string(),
                err
            )
        });
    let mut recv_buf = vec![0u8; buffer_size];
    let mut bytes_read: usize = 0;
    while read_stream(stream, &mut recv_buf, &mut bytes_read) > 0 {
        if let Err(e) = server_reference.broad_cast_msg(&recv_buf[..bytes_read]) {
            println!("Error: {}", e);
        };
    }
}

fn listen_for_updates(recv: Receiver<String>) {
    thread::spawn(move || {
        while match recv.recv() {
            Ok(data) => {
                println!("message from handle_client = {}", data);
                true
            }
            Err(_) => false,
        } {}
    });
}
fn main() {
    let (sender, receiver) = std::sync::mpsc::channel::<String>();

    listen_for_updates(receiver);
    let mut server = BubbleServer::<Sender<String>>::new(String::from("localhost:25568"));
    server.set_hc_param(sender); // Sets Handle Client Parameter to be sent to client that is connected.
    server.set_on_event(|event: ServerEvent| match event {
        //More event's are being worked on
        ServerEvent::Disconnection(socket_addr) => println!("Disconnection from {}", socket_addr),
        _ => {
            println!("event received not matching..");
        }
    });
    //non blocking
    if let Err(e) = server.start(&handle_new_text_client) {
        println!("Error starting server: {}", e);
    };

    //Block the thread here
    thread::park();
}
