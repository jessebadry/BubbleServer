use dashmap::DashMap;
use log::debug;
use std::io;
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::*;
use std::thread;
/*
TODO:
Implement clients list as concurrent hashmap, (dashmap library)
to increase performance, (remove unneccessary looping and have client disconnection more statically implemented),
we will use a atomicu64 to store the current client index then assign and increment each one to the next client and
use that index to remove from the hashmap, all while keeping concurrency (obviously)
*/

/// Events for debugging and getting information from the server.

#[allow(dead_code)]
pub enum ServerEvent {
    None,
    Disconnection(SocketAddr),
}
type SocketData = (u64, TcpStream);
type ErrorSender = Arc<Mutex<Option<Sender<ServerEvent>>>>;

/// Clients List Thread Safe
pub type ClientsListTS = Arc<Mutex<Vec<TcpStream>>>;

/// Data that is sent to user provided FnMut `set in fn start` when a client connects to a
/// started(fn start) server
pub type HandleClientType<'a, T> = (&'a mut TcpStream, Option<T>, &'a mut BubbleServer<T>);
#[derive(Clone)]
pub struct BubbleServer<T: Clone + Send + 'static> {
    ip: String,
    error_sender: ErrorSender,
    clients: Arc<DashMap<u64, TcpStream>>,
    client_index: Arc<AtomicU64>,

    param: Option<T>,
}

impl<T> BubbleServer<T>
where
    T: Clone + Send + 'static,
{
    pub fn new(ip: String) -> Self {
        BubbleServer::<T> {
            ip,
            clients: Default::default(),
            client_index: Default::default(),
            error_sender: Arc::new(Mutex::new(None)),
            param: Option::None,
        }
    }
    // fn get_sock_index(clients: &[TcpStream], socket_addr: &SocketAddr) -> io::Result<usize> {
    //     clients
    //         .iter()
    //         .position(|x| &x.peer_addr().unwrap() == socket_addr)
    //         .ok_or_else(|| {
    //             Error::new(
    //                 ErrorKind::InvalidInput,
    //                 format!("Could not find socket addre'{}' ", socket_addr),
    //             )
    //         })
    // }
    fn send_event(&self, event: ServerEvent) {
        let sender = self.error_sender.as_ref();

        if let Some(sender) = &*sender.lock().unwrap() {
            println!("Sending event to sender..");
            sender
                .send(event)
                .unwrap_or_else(|e| println!("error sending to error_sender {}", e));
        }
    }
    /// # Example:
    /// used to retrieve socket from clients by ip address
    /// ```
    /// let server = BubbleServer::new(String::from("localhost:25568"));
    /// let addr_to_find: SocketAddr = "127.0.0.1:25565"
    ///     .parse()
    ///     .expect("Could not parse ip address!");
    /// let socket = server.get_sock(&addr_to_find);
    /// assert!(socket.is_none());
    /// ```
    // #[allow(dead_code)]
    // pub fn get_sock(&self, socket_addr: &SocketAddr) -> Option<TcpStream> {
    //     let clients = self.get_clients().ok()?;

    //     // if let Ok(index) = BubbleServer::<T>::get_sock_index(&clients, socket_addr) {
    //     //     Some(clients[index].try_clone().unwrap())
    //     // } else {
    //     //     None
    //     // }
    //     None
    // }
    ///Set `param` in HandleClientType for connecting clients.
    pub fn set_hc_param(&mut self, param: T) {
        self.param.get_or_insert(param);
    }
    /// Shutdown the socket, then send a disconnection event to the `error_sender`.
    fn handle_socket_disconnection(&self, socket: &SocketData) -> io::Result<()> {
        let (index, socket) = socket;

        //Remove socket from clients list
        self.remove_socket(&index)?;
        let sock_addr = socket.peer_addr().unwrap();

        socket
            .shutdown(Shutdown::Both)
            .expect("Could not shutdown stream..");
        self.send_event(ServerEvent::Disconnection(sock_addr));

        debug!("Removed client {}, at index {}", sock_addr, index);
        Ok(())
    }

    /// `get_clients` Returns clients list reference using [`Arc::clone`].
    pub fn get_clients(&self) -> io::Result<Arc<DashMap<u64, TcpStream>>> {
        Ok(Arc::clone(&self.clients))
    }
    #[allow(dead_code)]
    pub fn set_on_event<F: Fn(ServerEvent) + Send + Sync + 'static>(&mut self, callback: F) {
        //Check if event already set.
        let err_sender = &mut self.error_sender.lock().unwrap();
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
    /// * Locks `clients` vec ([`Vec`]`<`[`TcpStream`]`>`)
    /// * Removes `socket` from clients list.
    fn remove_socket(&self, socket_index: &u64) -> io::Result<()> {
        //Is this the least verbose way to dereference this?
        let clients = self.get_clients()?;
        clients.remove(socket_index);
        Ok(())
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
            println!("disconnecting..!");
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
        self.client_index.store(
            num,
            Ordering::Relaxed,
        );

        num
    }

    ///Locks `clients` ([`Vec`]`<`[`TcpStream`]`>`) and adds `socket` to Vec
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
        let ip = &self.ip;
        debug!("server with  ip of {} has started...", ip);
        let socket_acceptor = TcpListener::bind(ip).map_err(|e| {
            io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                format!(
                    "'{}' is in use or is an invalid ip address! Raw Err: {}",
                    ip, e
                ),
            )
        })?;

        //I don't want the user to have the potential to deadlock the server
        //without explicitly locking mutex's in their own code.
        //for example it should be obvious that this will deadlock when a user connects
        // ```
        // {
        //     let server = BubbleServer::new(String::from("localhost:25568"));
        //     let clients = server.get_clients().expect("could not get clients!");
        //     let clients = clients.lock().unwrap();
        //     server.start(&|data: HandleClientType| {
        //         println!("new connection! dropping now!");
        //     });

        //     thread::park();
        // }
        // ```
        let mut self_ = self.clone();

        //Accept new incoming connections
        thread::spawn(move || loop {
            debug!("waiting for next client..");
            if let Ok((mut socket, _)) = socket_acceptor.accept() {
                println!("Accepted Client");
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
