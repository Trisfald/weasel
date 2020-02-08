//! A battle client.

use crate::battle::{Battle, BattleRules, EventCallback};
use crate::error::WeaselResult;
use crate::event::{
    EventProcessor, EventPrototype, EventReceiver, MultiClientSink, MultiClientSinkHandle,
    MultiClientSinkHandleMut, ServerSink, VersionedEventWrapper,
};
use crate::player::PlayerId;

/// A client event processor.
///
/// Clients can accept any kind of event from a remote server.
/// Local events are sent to the server to which the client is connected.
///
/// One or more client sinks can be connected to a client. Events received from
/// the server are propagated to these sinks.
pub struct Client<R: BattleRules> {
    battle: Battle<R>,
    server_sink: Box<dyn ServerSink<R>>,
    client_sinks: MultiClientSink<R>,
    player: Option<PlayerId>,
}

impl<R: BattleRules + 'static> Client<R> {
    /// Returns a client builder.
    pub fn builder(battle: Battle<R>, server_sink: Box<dyn ServerSink<R>>) -> ClientBuilder<R> {
        ClientBuilder {
            battle,
            server_sink,
            player: None,
        }
    }

    /// Returns a reference to the battle.
    pub fn battle(&self) -> &Battle<R> {
        &self.battle
    }

    /// Returns whether or not client events authentication is enabled.
    pub fn authentication(&self) -> bool {
        self.player.is_some()
    }

    /// Returns the player id associated to this client.
    pub fn player(&self) -> &Option<PlayerId> {
        &self.player
    }

    /// Returns a reference to the server sink to which all event prototypes
    /// initiated by this client are sent.
    #[allow(clippy::borrowed_box)]
    pub fn server_sink(&self) -> &Box<dyn ServerSink<R>> {
        &self.server_sink
    }

    /// Disconnects the current server sink and sets a new one.
    pub fn set_server_sink(&mut self, sink: Box<dyn ServerSink<R>>) {
        self.server_sink.on_disconnect();
        self.server_sink = sink;
    }

    /// Returns a handle to access the client sinks of this client.
    pub fn client_sinks(&self) -> MultiClientSinkHandle<'_, R> {
        MultiClientSinkHandle::new(&self.client_sinks)
    }

    /// Returns a mutable handle to manage the client sinks of this client.
    pub fn client_sinks_mut(&mut self) -> MultiClientSinkHandleMut<'_, R> {
        MultiClientSinkHandleMut::new(&mut self.client_sinks, &self.battle)
    }

    /// Returns the current event callback set to this client's battle.
    pub fn event_callback(&self) -> &Option<EventCallback<R>> {
        &self.battle.event_callback
    }

    /// Sets a new event callback for this client's battle.
    /// The current callback is discarded.
    pub fn set_event_callback(&mut self, callback: Option<EventCallback<R>>) {
        self.battle.event_callback = callback;
    }
}

impl<R: BattleRules + 'static> EventProcessor<R> for Client<R> {
    type ProcessOutput = WeaselResult<(), R>;

    fn process(&mut self, event: EventPrototype<R>) -> Self::ProcessOutput {
        self.battle.verify_prototype(&event)?;
        // Decorate the prototype with additional information.
        let event = event.client_prototype(self.battle().rules().version().clone(), self.player);
        // Send the event to the server.
        self.server_sink.send(&event)
    }
}

impl<R: BattleRules + 'static> EventReceiver<R> for Client<R> {
    fn receive(&mut self, event: VersionedEventWrapper<R>) -> WeaselResult<(), R> {
        // Verify the event.
        self.battle.verify_wrapper(&event)?;
        // Apply the event on the battle.
        self.battle.apply(&event.wrapper(), &mut None);
        // Send the event to all client sinks.
        self.client_sinks.send_all(&event);
        Ok(())
    }
}

/// A builder object to create a client.
pub struct ClientBuilder<R: BattleRules> {
    battle: Battle<R>,
    server_sink: Box<dyn ServerSink<R>>,
    player: Option<PlayerId>,
}

impl<R: BattleRules> ClientBuilder<R> {
    /// Enable authentication on the new client.
    /// All produced events will be authenticated with `player`.
    pub fn enable_authentication(mut self, player: PlayerId) -> ClientBuilder<R> {
        self.player = Some(player);
        self
    }

    /// Creates a new client.
    pub fn build(self) -> Client<R> {
        Client {
            battle: self.battle,
            server_sink: self.server_sink,
            client_sinks: MultiClientSink::new(),
            player: self.player,
        }
    }
}
