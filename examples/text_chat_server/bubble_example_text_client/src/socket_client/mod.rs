use std::io;
use std::io::prelude::*;
use std::net::{Shutdown, TcpStream};
use std::thread;
pub type Packet = (Vec<u8>, TcpStream);
pub trait TextClientTrait {
     fn send_str(&mut self, msg: &str) -> io::Result<()>;
     fn listen<F: Fn(Packet) + Send + Sync>(&mut self, output: F);
}
impl TextClientTrait for TcpStream {
     fn send_str(&mut self, msg: &str) -> io::Result<()> {
          let data = msg.as_bytes();
          self.write_all(&data)?;
          Ok(())
     }

     fn listen<F: Fn(Packet) + Send + Sync>(&mut self, output: F) {
          let mut buffer = vec![0u8; 1000];
          let mut listen_stream = &*self;
          while match listen_stream.read(&mut buffer) {
               Ok(byte_length) => {
                    if byte_length > 0 {
                         let data = (
                              buffer[..byte_length].to_vec(),
                              listen_stream.try_clone().unwrap(),
                         );
                         output(data);
                         true
                    } else {
                         false
                    }
               }
               Err(_) => {
                    listen_stream.shutdown(Shutdown::Both).unwrap();
                    false
               } //Leave Loop
          } {}
     }
}

fn connect(ip: &str) -> TcpStream {
     loop {
          thread::sleep(std::time::Duration::new(2,0));
          match TcpStream::connect(ip) {
               Ok(connection) => {
                    return connection;
               }
               Err(_) => {
                    println!("Failed to connect.");
                    continue;
               }
          }
     }
}
pub fn create_new<F: Fn(Packet) + Send + Sync>(ip: &str, listen_call: &'static F) {
     let ip_string = String::from(ip);
     let call = listen_call;
     thread::spawn(move || loop {
          let mut stream = connect(&ip_string);
          stream.listen(call);
     });
}
