use dashmap::DashMap;
use log::debug;
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::*;
use std::thread;

type SocketData = (u64, TcpStream);
type EventSender = Arc<Mutex<Option<Sender<ServerEvent>>>>;

/// Clients List Thread Safe
pub type ClientsListTS = Arc<Mutex<Vec<TcpStream>>>;

/// Data that is sent to user provided FnMut `set in fn start` when a client connects to a
/// started(fn start) server
pub type HandleClientType<'a, T> = (&'a mut TcpStream, Option<T>, &'a mut BubbleServer<T>);

/// Events for debugging and getting information from the server.
#[allow(dead_code)]
pub enum ServerEvent {
    None,
    Disconnection(SocketAddr),
}

#[derive(Clone)]
pub struct BubbleServer<T: Clone + Send + 'static> {
    ip: Arc<String>,
    event_sender: EventSender,
    clients: Arc<DashMap<u64, TcpStream>>,
    client_index: Arc<AtomicU64>,
    param: Option<T>,
}

impl<T> BubbleServer<T>
where
    T: Clone + Send + 'static,
{
    ///Initializes a new instance of BubbleServer.
    ///
    ///
    pub fn new(ip: impl AsRef<str>) -> Self {
        BubbleServer::<T> {
            ip: Arc::new(ip.as_ref().to_string()),
            clients: Default::default(),
            client_index: Default::default(),
            event_sender: Default::default(),
            param: Default::default(),
        }
    }
    /// Emits a ServerEvent to the event sender
    fn send_event(&self, event: ServerEvent) {
        let sender = self.event_sender.as_ref();

        if let Some(sender) = &*sender.lock().unwrap() {
            sender
                .send(event)
                .unwrap_or_else(|e| println!("error sending to error_sender {}", e));
        }
    }

    ///Set `param` in HandleClientType for connecting clients.
    pub fn set_hc_param(&mut self, param: T) {
        self.param.get_or_insert(param);
    }
    /// Shutdown the socket, then send a disconnection event to the `error_sender`.
    fn handle_socket_disconnection(&self, socket_data: &SocketData) -> io::Result<()> {
        let (index, socket) = socket_data;

        //Remove socket from clients list
        self.remove_socket(&index)?;
        let sock_addr = socket.peer_addr().unwrap();

        self.send_event(ServerEvent::Disconnection(sock_addr));

        debug!("Removed client {}, at index {}", sock_addr, index);
        Ok(())
    }

    /// `get_clients` Returns clients list reference using [`Arc::clone`].
    pub fn get_clients(&self) -> Arc<DashMap<u64, TcpStream>> {
        Arc::clone(&self.clients)
    }

    #[allow(dead_code)]
    pub fn set_on_event<F: Fn(ServerEvent) + Send + Sync + 'static>(&mut self, callback: F) {
        //Check if event already set.
        let err_sender = &mut self.event_sender.lock().unwrap();
        if err_sender.is_some() {
            panic!("BubbleServer Event already set!");
        }
        let (err_send, err_rec) = channel::<ServerEvent>();
        err_sender.get_or_insert(err_send);
        std::thread::spawn(move || loop {
            let event = err_rec.recv().unwrap();

            callback(event);
        });
    }
    /// Removes given TcpStream from server's `clients`
    ///
    /// ## Arguments
    /// `socket_index` - index of client-socket
    ///
    /// ## Result
    fn remove_socket(&self, socket_index: &u64) -> io::Result<()> {
        self.get_clients().remove(socket_index);
        Ok(())
    }

    pub fn clear_sockets(&mut self) {
        self.get_clients().clear();
    }

    /// Runs the user's defined function in a new thread passing in the newly connected socket.
    fn handle_client(
        &mut self,
        socket_data: (u64, &mut TcpStream),
        handle_client: impl Fn(HandleClientType<T>) + Send + 'static,
    ) -> io::Result<()> {
        let (index, socket) = socket_data;
        let mut socket = socket.try_clone()?;
        let mut _self = self.clone();
        thread::spawn(move || {
            handle_client((&mut socket, _self.param.clone(), &mut _self));
            _self
                .handle_socket_disconnection(&(index, socket))
                .unwrap_or_else(|why| {
                    panic!("Error in handling socket disconnection, Err: '{}' ", why);
                });
        });
        Ok(())
    }
    fn get_next_client_index(&self) -> u64 {
        let num = self.client_index.load(Ordering::Relaxed) + 1;
        self.client_index.store(num, Ordering::Relaxed);

        num
    }

    /// Locks `clients` ([`Vec`]`<`[`TcpStream`]`>`) and adds `socket` to Vec
    fn add_client(&self, socket: TcpStream) -> u64 {
        let next_index = self.get_next_client_index();
        self.clients.as_ref().insert(next_index, socket);
        next_index
    }

    /// Continously accepts incoming connections
    /// * Is non blocking.
    /// * Locks clients list when user connects
    pub fn start(
        &mut self,
        handle_client_cb: impl Fn(HandleClientType<T>) + Send + Clone + 'static,
    ) -> io::Result<()> {
        let ip = (*self.ip).clone();
        debug!("server with  ip of {} has started...", &ip);
        let socket_acceptor = TcpListener::bind(&ip).map_err(|e| {
            io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                format!(
                    "'{}' is in use or is an invalid ip address! Raw Err: {}",
                    &ip, e
                ),
            )
        })?;

        let mut self_ = self.clone();

        //Accept new incoming connections
        thread::spawn(move || loop {
            debug!("waiting for next client..");
            if let Ok((mut socket, _)) = socket_acceptor.accept() {
                // Add Client to the clients vector
                let client_index = self_.add_client(
                    socket
                        .try_clone()
                        .expect("Could not clone socket before handling socket.."),
                );
                //Run user's implementation on how to deal with new client
                self_
                    .handle_client((client_index, &mut socket), handle_client_cb.clone())
                    .unwrap_or_else(|e| {
                        println!("Error in handle_client : {}", e);
                    });
            }
            // Could provide implementation for an erroneous accept, and send that to the error_sender.
            // but if many clients were to make erroneous connections this might just clutter the error_sender's receive.
        });
        Ok(())
    }
}
