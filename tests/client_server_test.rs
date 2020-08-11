use std::ops::Range;
use std::sync::{Arc, Mutex};
use weasel::battle::{Battle, BattleController, BattleRules};
use weasel::entity::EntityId;
use weasel::event::{
    ClientEventPrototype, ClientSink, DummyEvent, EventKind, EventReceiver, EventServer, EventSink,
    EventSinkId, EventTrigger, ServerSink, VersionedEventWrapper,
};
use weasel::player::PlayerId;
use weasel::round::StartRound;
use weasel::team::CreateTeam;
use weasel::{battle_rules, rules::empty::*};
use weasel::{Client, Server};
use weasel::{WeaselError, WeaselResult};

#[cfg(feature = "serialization")]
mod helper;

const TEAM_1_ID: u32 = 1;
const CREATURE_1_ID: u32 = 1;
const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
const SERVER_1_ID: EventSinkId = 1;
const CLIENT_1_ID: EventSinkId = 1;
const CLIENT_2_ID: EventSinkId = 2;
const CLIENT_ERR_ID: EventSinkId = 99;
const PLAYER_1_ID: PlayerId = 1;
const PLAYER_2_ID: PlayerId = 2;

/// Retrieves events from a server or client
macro_rules! events {
    ($source: expr) => {{
        $source.lock().unwrap().battle().history().events()
    }};
}

macro_rules! add_sink {
    ($source: expr, $sink: expr) => {{
        assert_eq!(
            $source
                .lock()
                .unwrap()
                .client_sinks_mut()
                .add_sink(Box::new($sink.clone()))
                .err(),
            None
        );
    }};
}

macro_rules! add_sink_from {
    ($source: expr, $sink: expr, $start: expr) => {{
        assert_eq!(
            $source
                .lock()
                .unwrap()
                .client_sinks_mut()
                .add_sink_from(Box::new($sink.clone()), $start)
                .err(),
            None
        );
    }};
}

battle_rules! {}

#[derive(Clone)]
struct SinkImpl {
    id: EventSinkId,
    disconnections: u32,
    broken: bool,
}

impl SinkImpl {
    fn new(id: EventSinkId) -> Self {
        Self {
            id,
            disconnections: 0,
            broken: false,
        }
    }
}

/// A test `ServerSink` sending events to a local server.
struct TestServerSink<R: BattleRules> {
    sink: Arc<Mutex<SinkImpl>>,
    server: Arc<Mutex<Server<R>>>,
}

impl<R: BattleRules + 'static> TestServerSink<R> {
    fn new(id: EventSinkId, server: Arc<Mutex<Server<R>>>) -> TestServerSink<R> {
        TestServerSink {
            sink: Arc::new(Mutex::new(SinkImpl::new(id))),
            server,
        }
    }
}

impl<R: BattleRules> Clone for TestServerSink<R> {
    fn clone(&self) -> Self {
        TestServerSink {
            sink: self.sink.clone(),
            server: self.server.clone(),
        }
    }
}

impl<R: BattleRules> EventSink for TestServerSink<R> {
    fn id(&self) -> EventSinkId {
        self.sink.lock().unwrap().id
    }

    fn on_disconnect(&mut self) {
        self.sink.lock().unwrap().disconnections += 1;
    }
}

impl<R: BattleRules + 'static> ServerSink<R> for TestServerSink<R> {
    fn send(&mut self, event: &ClientEventPrototype<R>) -> WeaselResult<(), R> {
        if self.sink.lock().unwrap().broken {
            Err(WeaselError::EventSinkError("broken".to_string()))
        } else {
            self.server.lock().unwrap().process_client(event.clone())
        }
    }
}

/// A test `ClientSink` sending events to a local client.
struct TestClientSink<R: BattleRules> {
    sink: Arc<Mutex<SinkImpl>>,
    client: Arc<Mutex<Client<R>>>,
    buffer: Arc<Mutex<Vec<VersionedEventWrapper<R>>>>,
}

impl<R: BattleRules + 'static> TestClientSink<R> {
    fn new(id: EventSinkId, client: Arc<Mutex<Client<R>>>) -> TestClientSink<R> {
        TestClientSink {
            sink: Arc::new(Mutex::new(SinkImpl::new(id))),
            client,
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Dumps all received events into the client.
    /// It's needed because it is not possible to borrow_mut client inside send().
    fn receive(&mut self) -> WeaselResult<(), R> {
        let vec: Vec<_> = self.buffer.lock().unwrap().drain(..).collect();
        for event in vec.into_iter() {
            self.client.lock().unwrap().receive(event)?;
        }
        Ok(())
    }
}

impl<R: BattleRules> Clone for TestClientSink<R> {
    fn clone(&self) -> Self {
        TestClientSink {
            sink: self.sink.clone(),
            client: self.client.clone(),
            buffer: self.buffer.clone(),
        }
    }
}

impl<R: BattleRules> EventSink for TestClientSink<R> {
    fn id(&self) -> EventSinkId {
        self.sink.lock().unwrap().id
    }

    fn on_disconnect(&mut self) {
        self.sink.lock().unwrap().disconnections += 1;
    }
}

impl<R: BattleRules> ClientSink<R> for TestClientSink<R> {
    fn send(&mut self, event: &VersionedEventWrapper<R>) -> WeaselResult<(), R> {
        if self.sink.lock().unwrap().broken {
            Err(WeaselError::EventSinkError("broken".to_string()))
        } else {
            self.buffer.lock().unwrap().push(event.clone());
            Ok(())
        }
    }
}

#[test]
fn send_events() {
    // Create a server.
    let server = Arc::new(Mutex::new(util::server(CustomRules::new())));
    let server_sink = TestServerSink::new(SERVER_1_ID, server.clone());
    // Create a client.
    let client = Arc::new(Mutex::new(util::client(CustomRules::new(), server_sink)));
    // Connect the client to the server.
    let mut client_sink = TestClientSink::new(CLIENT_1_ID, client.clone());
    add_sink!(server, client_sink);
    // Attach an event recorder to the client.
    let event_recorder = TestClientSink::new(CLIENT_1_ID, client.clone());
    add_sink!(client, event_recorder);
    // Send an event from server and one from client.
    util::team(&mut *server.lock().unwrap(), TEAM_1_ID);
    assert_eq!(client_sink.receive().err(), None);
    util::dummy(&mut *client.lock().unwrap());
    assert_eq!(client_sink.receive().err(), None);
    // Verify whether both battles have the same history.
    assert_eq!(events!(server)[0].kind(), EventKind::CreateTeam);
    assert_eq!(events!(client)[0].kind(), EventKind::CreateTeam);
    assert_eq!(events!(server)[1].kind(), EventKind::DummyEvent);
    assert_eq!(events!(client)[1].kind(), EventKind::DummyEvent);
    // Verify that the event recorder stores the same event as the client and server.
    assert_eq!(
        event_recorder.buffer.lock().unwrap()[0].kind(),
        EventKind::CreateTeam
    );
    assert_eq!(
        event_recorder.buffer.lock().unwrap()[1].kind(),
        EventKind::DummyEvent
    );
}

#[test]
fn send_errors() {
    // Create a server.
    let server = Arc::new(Mutex::new(util::server(CustomRules::new())));
    let server_sink = TestServerSink::new(SERVER_1_ID, server.clone());
    // Create two clients.
    let client_1 = Arc::new(Mutex::new(util::client(
        CustomRules::new(),
        server_sink.clone(),
    )));
    let client_2 = Arc::new(Mutex::new(util::client(
        CustomRules::new(),
        server_sink.clone(),
    )));
    // Connects the clients to the server.
    let mut client_sink_1 = TestClientSink::new(CLIENT_1_ID, client_1.clone());
    add_sink!(server, client_sink_1);
    let mut client_sink_2 = TestClientSink::new(CLIENT_2_ID, client_2.clone());
    add_sink!(server, client_sink_2);
    // Send event from one client. One client sink is faulty.
    client_sink_2.sink.lock().unwrap().broken = true;
    util::dummy(&mut *client_1.lock().unwrap());
    assert_eq!(client_sink_1.receive().err(), None);
    assert_eq!(client_sink_2.receive().err(), None);
    // Event should be in the server and in one client.
    assert_eq!(events!(server).len(), 1);
    assert_eq!(events!(client_1).len(), 1);
    assert_eq!(events!(client_2).len(), 0);
    // Check if the faulty sink got disconnected.
    assert_eq!(client_sink_2.sink.lock().unwrap().disconnections, 1);
    assert_eq!(server.lock().unwrap().client_sinks().sinks().count(), 1);
    // Server sink is faulty. Check no new event is added to the server.
    server_sink.sink.lock().unwrap().broken = true;
    assert_eq!(
        DummyEvent::trigger(&mut *client_1.lock().unwrap())
            .fire()
            .err(),
        Some(WeaselError::EventSinkError("broken".to_string()))
    );
    assert_eq!(events!(server).len(), 1);
}

#[test]
fn integrity_checks() {
    // Create a server.
    let server = Arc::new(Mutex::new(util::server(CustomRules::new())));
    let server_sink = TestServerSink::new(SERVER_1_ID, server.clone());
    // Create a client.
    let client = Arc::new(Mutex::new(util::client(
        CustomRules::new(),
        server_sink.clone(),
    )));
    // Fire event.
    util::dummy(&mut *server.lock().unwrap());
    // Connect the client, but not from history start.
    let mut client_sink = TestClientSink::new(CLIENT_1_ID, client.clone());
    add_sink!(server, client_sink);
    // Client should block the next server event.
    util::dummy(&mut *server.lock().unwrap());
    assert_eq!(
        client_sink.receive().err(),
        Some(WeaselError::NonContiguousEventId(1, 0))
    );
    // Reattach client from history start.
    server
        .lock()
        .unwrap()
        .client_sinks_mut()
        .remove_sink(CLIENT_1_ID);
    add_sink_from!(server, client_sink, 0);
    assert_eq!(client_sink.receive().err(), None);
    assert_eq!(events!(server).len(), 2);
    assert_eq!(events!(client).len(), 2);
    // Client should receive the next event.
    util::team(&mut *server.lock().unwrap(), TEAM_1_ID);
    util::creature(&mut *server.lock().unwrap(), CREATURE_1_ID, TEAM_1_ID, ());
    assert_eq!(client_sink.receive().err(), None);
    assert_eq!(events!(server).len(), 4);
    assert_eq!(events!(client).len(), 4);
    // Fire an event in client. It should be processed correctly.
    util::dummy(&mut *client.lock().unwrap());
    assert_eq!(client_sink.receive().err(), None);
    assert_eq!(events!(server).len(), 5);
    assert_eq!(events!(client).len(), 5);
    // Change server.
    assert_eq!(server_sink.sink.lock().unwrap().disconnections, 0);
    let server = Arc::new(Mutex::new(util::server(CustomRules::new())));
    let new_server_sink = TestServerSink::new(SERVER_1_ID, server.clone());
    client
        .lock()
        .unwrap()
        .set_server_sink(Box::new(new_server_sink));
    assert_eq!(server_sink.sink.lock().unwrap().disconnections, 1);
    // Fire another event in the client.
    assert_eq!(
        StartRound::trigger(&mut *client.lock().unwrap(), ENTITY_1_ID)
            .fire()
            .err(),
        Some(WeaselError::EntityNotFound(ENTITY_1_ID))
    );
    // Events should be blocked by the new server.
    assert_eq!(events!(server).len(), 0);
    assert_eq!(events!(client).len(), 5);
}

#[test]
fn check_version() {
    const VERSION_NEW: u32 = 4;
    const VERSION_OLD: u32 = 2;
    // Create a server with newer rules.
    let mut rules = CustomRules::new();
    rules.version = VERSION_NEW;
    let server = Arc::new(Mutex::new(util::server(rules)));
    let server_sink = TestServerSink::new(SERVER_1_ID, server.clone());
    // Create a client with older rules.
    let mut rules = CustomRules::new();
    rules.version = VERSION_OLD;
    let client = Arc::new(Mutex::new(util::client(rules, server_sink)));
    // Connect the client to the server.
    let mut client_sink = TestClientSink::new(CLIENT_1_ID, client.clone());
    add_sink_from!(server, client_sink, 0);
    // Check if events from server are rejected.
    util::dummy(&mut *server.lock().unwrap());
    assert_eq!(
        client_sink.receive().err(),
        Some(WeaselError::IncompatibleVersions(VERSION_OLD, VERSION_NEW))
    );
    // Check if events from client are rejected.
    assert_eq!(
        DummyEvent::trigger(&mut *client.lock().unwrap())
            .fire()
            .err(),
        Some(WeaselError::IncompatibleVersions(VERSION_OLD, VERSION_NEW))
    );
}

#[test]
fn add_client_sink() {
    // Create server.
    let server = Arc::new(Mutex::new(util::server(CustomRules::new())));
    let server_sink = TestServerSink::new(SERVER_1_ID, server.clone());
    // Fire four events.
    for _ in 0..4 {
        util::dummy(&mut *server.lock().unwrap());
    }
    assert_eq!(events!(server).len(), 4);
    // Create client.
    let client = Arc::new(Mutex::new(util::client(CustomRules::new(), server_sink)));
    let mut client_sink = TestClientSink::new(CLIENT_1_ID, client.clone());
    // Add client sink with invalid range.
    let range = Range { start: 5, end: 7 };
    assert_eq!(
        server
            .lock()
            .unwrap()
            .client_sinks_mut()
            .add_sink_range(Box::new(client_sink.clone()), range.clone())
            .err(),
        Some(WeaselError::InvalidEventRange(range, 4))
    );
    let range = Range { start: 0, end: 7 };
    assert_eq!(
        server
            .lock()
            .unwrap()
            .client_sinks_mut()
            .add_sink_range(Box::new(client_sink.clone()), range.clone())
            .err(),
        Some(WeaselError::InvalidEventRange(range, 4))
    );
    // Add the client sink and send the first two events.
    assert_eq!(
        server
            .lock()
            .unwrap()
            .client_sinks_mut()
            .add_sink_range(Box::new(client_sink.clone()), Range { start: 0, end: 2 })
            .err(),
        None
    );
    assert_eq!(client_sink.receive().err(), None);
    assert_eq!(events!(client).len(), 2);
    // Check id is verified when sending events.
    assert_eq!(
        server
            .lock()
            .unwrap()
            .client_sinks_mut()
            .send_range(CLIENT_ERR_ID, Range { start: 0, end: 2 })
            .err(),
        Some(WeaselError::EventSinkNotFound(CLIENT_ERR_ID))
    );
    // Send the other two events.
    assert_eq!(
        server
            .lock()
            .unwrap()
            .client_sinks_mut()
            .send_range(CLIENT_1_ID, Range { start: 2, end: 4 })
            .err(),
        None
    );
    assert_eq!(client_sink.receive().err(), None);
    assert_eq!(events!(client).len(), 4);
}

#[test]
fn rights() {
    // Create a server with auth.
    let server = Server::builder(Battle::builder(CustomRules::new()).build())
        .enforce_authentication()
        .build();
    let server = Arc::new(Mutex::new(server));
    // Create a client with auth.
    let server_sink = TestServerSink::new(SERVER_1_ID, server.clone());
    let client = Client::builder(
        Battle::builder(CustomRules::new()).build(),
        Box::new(server_sink),
    )
    .enable_authentication(PLAYER_1_ID)
    .build();
    let client = Arc::new(Mutex::new(client));
    assert_eq!(client.lock().unwrap().authentication(), true);
    // Connect the client to the server.
    let mut client_sink = TestClientSink::new(CLIENT_1_ID, client.clone());
    add_sink!(server, client_sink);
    // Check that new rights for non existing team are rejected.
    assert_eq!(
        server
            .lock()
            .unwrap()
            .rights_mut()
            .add(PLAYER_1_ID, &TEAM_1_ID)
            .err(),
        Some(WeaselError::TeamNotFound(TEAM_1_ID))
    );
    // Create a team and some rights.
    util::team(&mut *server.lock().unwrap(), TEAM_1_ID);
    util::creature(&mut *server.lock().unwrap(), CREATURE_1_ID, TEAM_1_ID, ());
    assert_eq!(client_sink.receive().err(), None);
    assert_eq!(
        server
            .lock()
            .unwrap()
            .rights_mut()
            .add(PLAYER_2_ID, &TEAM_1_ID)
            .err(),
        None
    );
    // Check that rights are enforced for the wrong client.
    assert_eq!(
        StartRound::trigger(&mut *client.lock().unwrap(), ENTITY_1_ID)
            .fire()
            .err(),
        Some(WeaselError::AuthenticationError(
            Some(PLAYER_1_ID),
            TEAM_1_ID
        ))
    );
    // Create a client without any authentication.
    server
        .lock()
        .unwrap()
        .client_sinks_mut()
        .remove_sink(CLIENT_1_ID);
    let server_sink = TestServerSink::new(SERVER_1_ID, server.clone());
    let client = Arc::new(Mutex::new(util::client(CustomRules::new(), server_sink)));
    // Connect the client to the server.
    let mut client_sink = TestClientSink::new(CLIENT_1_ID, client.clone());
    add_sink_from!(server, client_sink, 0);
    assert_eq!(client_sink.receive().err(), None);
    // Client events should be rejected.
    assert_eq!(
        StartRound::trigger(&mut *client.lock().unwrap(), ENTITY_1_ID)
            .fire()
            .err(),
        Some(WeaselError::MissingAuthentication)
    );
    // Connect a client with correct rights.
    let server_sink = TestServerSink::new(SERVER_1_ID, server.clone());
    let client = Client::builder(
        Battle::builder(CustomRules::new()).build(),
        Box::new(server_sink),
    )
    .enable_authentication(PLAYER_2_ID)
    .build();
    let client = Arc::new(Mutex::new(client));
    // Connect the client to the server.
    let mut client_sink = TestClientSink::new(CLIENT_2_ID, client.clone());
    add_sink_from!(server, client_sink, 0);
    assert_eq!(client_sink.receive().err(), None);
    // Check that the good client can send events.
    assert_eq!(
        StartRound::trigger(&mut *client.lock().unwrap(), ENTITY_1_ID)
            .fire()
            .err(),
        None,
    );
}

#[test]
fn server_only_events() {
    // Create a client and a server.
    let server = Arc::new(Mutex::new(util::server(CustomRules::new())));
    let mut server_sink = TestServerSink::new(SERVER_1_ID, server.clone());
    let client = Arc::new(Mutex::new(util::client(
        CustomRules::new(),
        server_sink.clone(),
    )));
    // Set client sink.
    let client_sink = TestClientSink::new(CLIENT_1_ID, client.clone());
    add_sink!(server, client_sink);
    // Verify that client blocks server-only events.
    assert_eq!(
        CreateTeam::trigger(&mut *client.lock().unwrap(), TEAM_1_ID)
            .fire()
            .err(),
        Some(WeaselError::ServerOnlyEvent)
    );
    // Verify that server blocks server-only events from clients.
    let event = CreateTeam::trigger(&mut *client.lock().unwrap(), TEAM_1_ID)
        .prototype()
        .client_prototype(0, None);
    assert_eq!(
        server_sink.send(&event).err(),
        Some(WeaselError::ServerOnlyEvent)
    );
}

#[cfg(feature = "serialization")]
#[test]
fn client_server_serde() {
    use weasel::creature::RemoveCreature;
    use weasel::round::EndRound;

    const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
    // Create a server.
    let server = Arc::new(Mutex::new(util::server(CustomRules::new())));
    let server_sink = TestServerSink::new(SERVER_1_ID, server.clone());
    // Create a client.
    let client = Arc::new(Mutex::new(util::client(CustomRules::new(), server_sink)));
    // Connect the client to the server.
    let mut client_sink = TestClientSink::new(CLIENT_1_ID, client.clone());
    add_sink!(server, client_sink);
    // Send events from server and from client.
    util::team(&mut *server.lock().unwrap(), TEAM_1_ID);
    util::creature(&mut *server.lock().unwrap(), TEAM_1_ID, CREATURE_1_ID, ());
    assert_eq!(client_sink.receive().err(), None);
    assert_eq!(
        StartRound::trigger(&mut *client.lock().unwrap(), ENTITY_1_ID)
            .fire()
            .err(),
        None
    );
    assert_eq!(client_sink.receive().err(), None);
    // Verify whether both battles have the same history.
    assert_eq!(
        events!(server).iter().map(|e| e.kind()).collect::<Vec<_>>(),
        vec![
            EventKind::CreateTeam,
            EventKind::CreateCreature,
            EventKind::StartRound
        ]
    );
    assert_eq!(
        events!(client).iter().map(|e| e.kind()).collect::<Vec<_>>(),
        vec![
            EventKind::CreateTeam,
            EventKind::CreateCreature,
            EventKind::StartRound
        ]
    );
    // Start a new server and load history.
    let history_json = helper::history_as_json(server.lock().unwrap().battle());
    let server = Arc::new(Mutex::new(util::server(CustomRules::new())));
    let server_sink = TestServerSink::new(SERVER_1_ID, server.clone());
    helper::load_json_history(&mut *server.lock().unwrap(), history_json);
    // Start a new client and load history.
    let history_json = helper::history_as_json(server.lock().unwrap().battle());
    let client = Arc::new(Mutex::new(util::client(CustomRules::new(), server_sink)));
    // Connect the client to the server.
    let mut client_sink = TestClientSink::new(CLIENT_1_ID, client.clone());
    helper::load_json_history(&mut *client.lock().unwrap(), history_json);
    add_sink!(server, client_sink);
    // Fire new events.
    assert_eq!(
        EndRound::trigger(&mut *client.lock().unwrap()).fire().err(),
        None
    );
    assert_eq!(
        RemoveCreature::trigger(&mut *server.lock().unwrap(), CREATURE_1_ID)
            .fire()
            .err(),
        None
    );
    assert_eq!(client_sink.receive().err(), None);
    // Verify whether both battles have the same history.
    assert_eq!(
        events!(server).iter().map(|e| e.kind()).collect::<Vec<_>>(),
        vec![
            EventKind::CreateTeam,
            EventKind::CreateCreature,
            EventKind::StartRound,
            EventKind::EndRound,
            EventKind::RemoveCreature,
        ]
    );
    assert_eq!(
        events!(client).iter().map(|e| e.kind()).collect::<Vec<_>>(),
        vec![
            EventKind::CreateTeam,
            EventKind::CreateCreature,
            EventKind::StartRound,
            EventKind::EndRound,
            EventKind::RemoveCreature,
        ]
    );
}
