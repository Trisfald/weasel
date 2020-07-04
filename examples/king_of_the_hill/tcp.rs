use crate::rules::CustomRules;
use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::{io, thread, thread::JoinHandle, time};
use weasel::battle::{Battle, BattleRules};
use weasel::event::{
    ClientEventPrototype, ClientSink, EventReceiver, EventServer, EventSink, EventSinkId,
    ServerSink, VersionedEventWrapper,
};
use weasel::serde::{FlatClientEvent, FlatVersionedEvent};
use weasel::team::TeamId;
use weasel::{Client, Server, WeaselResult};

const REMOTE_CLIENTS: usize = 2;
/// Shutdown byte.
const BYE: u8 = 0xFD;
/// Delimiter byte between messages.
const DELIMITER: u8 = 0xFE;
/// This byte followed by another byte (containing the team id)
/// signals to the client that the game is ready.
const READY_BYTE: u8 = 0xFB;
/// This bytes followed by a serialized event is used to transfer events
/// to remote clients.
const EVENT_BYTE: u8 = 0xFC;

/// A tpc `ServerSink` sending events to a remote server.
pub(crate) struct TcpServerSink<R: BattleRules> {
    /// A tcp stream to send data.
    stream: TcpStream,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: BattleRules> Drop for TcpServerSink<R> {
    fn drop(&mut self) {
        // Send a bye message and shutdown the stream.
        self.stream.write_all(&[BYE, DELIMITER]).unwrap();
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

impl<R: BattleRules + 'static> TcpServerSink<R> {
    pub(crate) fn new(stream: TcpStream) -> TcpServerSink<R> {
        TcpServerSink {
            stream,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<R: BattleRules> EventSink for TcpServerSink<R> {
    fn id(&self) -> EventSinkId {
        0
    }

    fn on_disconnect(&mut self) {
        println!("Server sink disconnected!");
    }
}

impl<R: BattleRules + 'static> ServerSink<R> for TcpServerSink<R> {
    fn send(&mut self, event: &ClientEventPrototype<R>) -> WeaselResult<(), R> {
        // Serialize the event and send it over tcp.
        // We use json for simplicity. There are more network friendly formats.
        let event: FlatClientEvent<R> = event.clone().into();
        let mut buffer = serde_json::to_vec(&event).unwrap();
        // Write the delimiter.
        buffer.push(DELIMITER);
        self.stream.write_all(&buffer).unwrap();
        Ok(())
    }
}

/// A tcp `ClientSink` sending events to a remote client.
pub(crate) struct TcpClientSink<R: BattleRules> {
    /// The id of this sink.
    id: EventSinkId,
    /// A tcp stream to send data.
    stream: TcpStream,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: BattleRules + 'static> TcpClientSink<R> {
    pub(crate) fn new(id: EventSinkId, stream: TcpStream) -> TcpClientSink<R> {
        TcpClientSink {
            id,
            stream,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<R: BattleRules> EventSink for TcpClientSink<R> {
    fn id(&self) -> EventSinkId {
        self.id
    }

    fn on_disconnect(&mut self) {
        println!("Client sink {} disconnected!", self.id);
    }
}

impl<R: BattleRules + 'static> ClientSink<R> for TcpClientSink<R> {
    fn send(&mut self, event: &VersionedEventWrapper<R>) -> WeaselResult<(), R> {
        // Serialize the event and send it over tcp.
        // We use json for simplicity. There are more network friendly formats.
        let event: FlatVersionedEvent<R> = event.clone().into();
        // Write the event byte to signal this's an event.
        let mut buffer = vec![EVENT_BYTE];
        buffer.append(&mut serde_json::to_vec(&event).unwrap());
        // Write the delimiter.
        buffer.push(DELIMITER);
        self.stream.write_all(&buffer).unwrap();
        Ok(())
    }
}

/// A game server working over tcp
pub(crate) struct TcpServer {
    pub(crate) game_server: Arc<Mutex<Server<CustomRules>>>,
    thread: Option<JoinHandle<()>>,
    running: Arc<Mutex<bool>>,
}

impl Drop for TcpServer {
    fn drop(&mut self) {
        *self.running.lock().unwrap() = false;
        self.thread.take().unwrap().join().unwrap();
    }
}

impl TcpServer {
    pub(crate) fn new(server: Server<CustomRules>) -> TcpServer {
        let game_server = Arc::new(Mutex::new(server));
        let game_server_clone = game_server.clone();
        let running = Arc::new(Mutex::new(true));
        let running_clone = running.clone();
        // Start a thread to listen for tcp connections.
        let thread = thread::spawn(move || {
            let listener = TcpListener::bind("0.0.0.0:3000").unwrap();
            listener
                .set_nonblocking(true)
                .expect("Cannot set non-blocking");
            // Threads to serve clients.
            let mut client_threads = Vec::new();
            for stream in listener.incoming() {
                match stream {
                    Ok(s) => {
                        // We don't want more than two remote clients.
                        if game_server_clone
                            .lock()
                            .unwrap()
                            .client_sinks()
                            .sinks()
                            .count()
                            == REMOTE_CLIENTS
                        {
                            println!("Dropping connection to additional client");
                            continue;
                        }
                        let game_server_clone = game_server_clone.clone();
                        let running_clone = running_clone.clone();
                        let handle = thread::spawn(move || {
                            TcpServer::handle_client(s, game_server_clone, running_clone);
                        });
                        client_threads.push(handle);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        if !*running_clone.lock().unwrap() {
                            break;
                        }
                    }
                    Err(e) => panic!("encountered IO error: {}", e),
                }
                thread::sleep(time::Duration::from_millis(100));
            }
            for handle in client_threads {
                handle.join().unwrap();
            }
        });
        // Wait for players.
        println!("Waiting for all players to connect...");
        loop {
            if game_server.lock().unwrap().client_sinks().sinks().count() == REMOTE_CLIENTS {
                println!("All players connected!");
                break;
            }
            thread::sleep(time::Duration::from_millis(10));
        }
        TcpServer {
            game_server,
            thread: Some(thread),
            running,
        }
    }

    fn handle_client(
        mut stream: TcpStream,
        game_server: Arc<Mutex<Server<CustomRules>>>,
        running: Arc<Mutex<bool>>,
    ) {
        let id = {
            println!("A client connected");
            // Find out the id of the newly connected player.
            let mut server = game_server.lock().unwrap();
            let id = if server
                .client_sinks()
                .sinks()
                .find(|s| s.id() == 1)
                .is_some()
            {
                2 as u8
            } else {
                1 as u8
            };
            // Register a client sink and share the battle history, from the beginning.
            let sink = Box::new(TcpClientSink::new(id.into(), stream.try_clone().unwrap()));
            server.client_sinks_mut().add_sink_from(sink, 0).unwrap();
            id
        };
        // Send the ready signal to the client.
        if let Err(_) = stream.write_all(&[READY_BYTE, id, DELIMITER]) {
            println!(
                "An error occurred, terminating connection with {}",
                stream.peer_addr().unwrap()
            );
            stream.shutdown(Shutdown::Both).unwrap();
            game_server
                .lock()
                .unwrap()
                .client_sinks_mut()
                .remove_sink(id.into());
            return;
        }
        // Listen for the client's events.
        stream
            .set_nonblocking(true)
            .expect("Cannot set non-blocking");
        let mut buffer = Vec::new();
        let mut reader = BufReader::new(stream);
        // Keep the connection until we get an error or we are closing the server.
        loop {
            match reader.read_until(DELIMITER, &mut buffer) {
                Ok(size) => {
                    if size == 0 {
                        println!("A client disconnected");
                        break;
                    }
                    if size > 1 {
                        if size == 2 && buffer[0] == BYE {
                            println!("A client disconnected");
                            break;
                        }
                        // Process the event.
                        let event: FlatClientEvent<CustomRules> =
                            serde_json::from_slice(&buffer[..size - 1]).unwrap();
                        game_server
                            .lock()
                            .unwrap()
                            .process_client(event.into())
                            .unwrap();
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(_) => {
                    println!("An error occurred, terminating connection.");
                    break;
                }
            }
            buffer.clear();
            if !*running.lock().unwrap() {
                break;
            }
            thread::sleep(time::Duration::from_millis(10));
        }
        game_server
            .lock()
            .unwrap()
            .client_sinks_mut()
            .remove_sink(id.into());
    }
}

/// A game client working over tcp.
pub(crate) struct TcpClient {
    /// The id assigned to this client by the server.
    pub(crate) id: TeamId<CustomRules>,
    pub(crate) game_client: Arc<Mutex<Client<CustomRules>>>,
    thread: Option<JoinHandle<()>>,
    running: Arc<Mutex<bool>>,
}

impl Drop for TcpClient {
    fn drop(&mut self) {
        *self.running.lock().unwrap() = false;
        self.thread.take().unwrap().join().unwrap();
    }
}

impl TcpClient {
    pub(crate) fn new(server_address: &str) -> TcpClient {
        // Open a connection to the server.
        let stream = TcpStream::connect(server_address).unwrap();
        stream
            .set_nonblocking(true)
            .expect("Cannot set non-blocking");
        println!("Connected to the server!");
        // Create a battle object with our game rules.
        let battle = Battle::builder(CustomRules::new()).build();
        // Create a server sink to send event over tcp.
        let sink = Box::new(TcpServerSink::new(stream.try_clone().unwrap()));
        let game_client = Arc::new(Mutex::new(Client::builder(battle, sink).build()));
        // Read everything the server has to send to us until the READY_BYTE.
        println!("Waiting for the game to start...");
        let game_client_clone = game_client.clone();
        let mut buffer = Vec::new();
        let mut reader = BufReader::new(stream);
        let id;
        loop {
            match reader.read_until(DELIMITER, &mut buffer) {
                Ok(size) => {
                    if size == 0 {
                        panic!("Disconnected from the server during initialization!");
                    }
                    // Wait for the ready byte.
                    if size == 3 && buffer[0] == READY_BYTE {
                        id = Some(buffer[1] as u8);
                        break;
                    } else if size > 1 && buffer[0] == EVENT_BYTE {
                        let event: FlatVersionedEvent<CustomRules> =
                            serde_json::from_slice(&buffer[1..size - 1]).unwrap();
                        game_client_clone
                            .lock()
                            .unwrap()
                            .receive(event.into())
                            .unwrap();
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(_) => {
                    panic!("An error occurred, terminating connection.");
                }
            }
            buffer.clear();
            thread::sleep(time::Duration::from_millis(10));
        }
        let id = id.unwrap();
        println!("You are player {}", id + 1);
        let running = Arc::new(Mutex::new(true));
        let running_clone = running.clone();
        // Keep the tcp channel open in another thread.
        let thread = thread::spawn(move || {
            loop {
                // Read events coming from the server.
                match reader.read_until(DELIMITER, &mut buffer) {
                    Ok(size) => {
                        if size == 0 {
                            println!("Disconnected from the server.");
                            break;
                        }
                        if size > 1 && buffer[0] == EVENT_BYTE {
                            let event: FlatVersionedEvent<CustomRules> =
                                serde_json::from_slice(&buffer[1..size - 1]).unwrap();
                            game_client_clone
                                .lock()
                                .unwrap()
                                .receive(event.into())
                                .unwrap();
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                    Err(_) => {
                        println!("An error occurred, terminating connection.");
                        break;
                    }
                }
                buffer.clear();
                if !*running_clone.lock().unwrap() {
                    break;
                }
                thread::sleep(time::Duration::from_millis(10));
            }
        });
        TcpClient {
            id: id.into(),
            game_client,
            thread: Some(thread),
            running,
        }
    }
}
