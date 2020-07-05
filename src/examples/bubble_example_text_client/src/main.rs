mod socket_client;
use socket_client::TextClientTrait;
use std::thread;
fn get_line() -> String {
    let mut text = String::new();
    std::io::stdin().read_line(&mut text).unwrap();
    text.replace("\r", "").replace("\n", "")
}
//Implement the example  text chat behaviour
fn text_client_recv(data: socket_client::Packet) {
    let (buf, socket_) = data;
    let text_msg = std::str::from_utf8(&buf).unwrap_or_else(|err| {
        println!("Err converting msg, : {} ", err);
        ""
    });
    println!("Received message : {}\n\n", text_msg);
    //If the servers sends this sockets IP Address, the user's input will open.

    if text_msg == socket_.local_addr().unwrap().to_string() {
        let mut socket_clone = socket_.try_clone().unwrap();
        thread::spawn(move || loop {
            println!("Enter message to chat:\n");
            let input = get_line();
            if let Err(e) = socket_clone.send_str(&input){
                println!("error sending msg, {}", e);
                break;
            }
        });
    } else {
        println!("message from server: '{}'", text_msg);
    }
}

fn main() {
    socket_client::create_new("localhost:25568", &text_client_recv);
    thread::park();
}
