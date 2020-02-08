//! A battle server.

use crate::battle::{Battle, BattleRules, EventCallback};
use crate::error::{WeaselError, WeaselResult};
use crate::event::{
    ClientEventPrototype, EventProcessor, EventPrototype, EventQueue, EventReceiver, EventRights,
    EventServer, EventWrapper, MultiClientSink, MultiClientSinkHandle, MultiClientSinkHandleMut,
    VersionedEventWrapper,
};
use crate::player::{RightsHandle, RightsHandleMut};
use crate::team::TeamId;

/// The server is the main object used to orchestrate a battle.
///
/// A server owns all data of the battle and it can also process events. Events are the only way in
/// which a battle can be evolved; all battle data can be retrieved through immutable references.\
/// Exactly one server is required in order to start a game.
///
/// One or more client sinks can be connected to a server, to receive verified events.
pub struct Server<R: BattleRules> {
    pub(crate) battle: Battle<R>,
    client_sinks: MultiClientSink<R>,
    authentication: bool,
}

impl<R: BattleRules + 'static> Server<R> {
    /// Returns a server builder.
    pub fn builder(battle: Battle<R>) -> ServerBuilder<R> {
        ServerBuilder {
            battle,
            authentication: false,
        }
    }

    /// Returns a reference to the battle.
    pub fn battle(&self) -> &Battle<R> {
        &self.battle
    }

    /// Returns true if the client events authentication is enforced.
    pub fn authentication(&self) -> bool {
        self.authentication
    }

    /// Returns a handle to access the players' rights to control one or more teams.
    pub fn rights(&self) -> RightsHandle<R> {
        self.battle.rights()
    }

    /// Returns a mutable handle to manage the players' rights to control one or more teams.
    pub fn rights_mut<'a>(&'a mut self) -> RightsHandleMut<R, impl Iterator<Item = &'a TeamId<R>>> {
        self.battle.rights_mut()
    }

    /// Returns a handle to access the client sinks of this server.
    pub fn client_sinks(&self) -> MultiClientSinkHandle<'_, R> {
        MultiClientSinkHandle::new(&self.client_sinks)
    }

    /// Returns a mutable handle to manage the client sinks of this server.
    pub fn client_sinks_mut(&mut self) -> MultiClientSinkHandleMut<'_, R> {
        MultiClientSinkHandleMut::new(&mut self.client_sinks, &self.battle)
    }

    /// Returns the current event callback set to this server's battle.
    pub fn event_callback(&self) -> &Option<EventCallback<R>> {
        &self.battle.event_callback
    }

    /// Sets a new event callback for this server's battle.
    /// The current callback is discarded.
    pub fn set_event_callback(&mut self, callback: Option<EventCallback<R>>) {
        self.battle.event_callback = callback;
    }

    /// Applies an event. The event must be valid.
    fn apply_event(&mut self, event: EventWrapper<R>) -> WeaselResult<(), R> {
        let mut event_queue = Some(EventQueue::<R>::new());
        // Apply the event on the battle.
        self.battle.apply(&event, &mut event_queue);
        // Send the event to all client sinks.
        self.client_sinks
            .send_all(&event.clone().version(self.battle.rules().version().clone()));
        // Recursively process derived events.
        let mut errors = Vec::new();
        if let Some(event_queue) = event_queue {
            for mut prototype in event_queue {
                // Set origin id in derived event.
                prototype.origin = Some(event.id);
                let result = self.process(prototype);
                if let Err(error) = result {
                    errors.push(error);
                }
            }
        }
        // If there is an error, return it.
        // In the case of multiple errors, wrap them into a multi error.
        match errors.len() {
            1 => Err(errors.swap_remove(0)),
            x if x > 1 => Err(WeaselError::MultiError(errors)),
            _ => Ok(()),
        }
    }
}

impl<R: BattleRules + 'static> EventProcessor<R> for Server<R> {
    type ProcessOutput = WeaselResult<(), R>;

    fn process(&mut self, event: EventPrototype<R>) -> Self::ProcessOutput {
        // Verify this event.
        self.battle
            .verify_prototype(&event)
            .map_err(|e| WeaselError::InvalidEvent(event.event().clone(), e.into()))?;
        // Promote verified event.
        let event = self.battle.promote(event);
        // Apply it.
        self.apply_event(event)
    }
}

impl<R: BattleRules + 'static> EventServer<R> for Server<R> {
    fn process_client(&mut self, event: ClientEventPrototype<R>) -> WeaselResult<(), R> {
        // Verify this event.
        self.battle.verify_client(&event)?;
        // Verify event's rights.
        match event.rights(&self.battle) {
            EventRights::Server => {
                return Err(WeaselError::ServerOnlyEvent);
            }
            EventRights::Team(team_id) => {
                if self.authentication {
                    if let Some(player) = event.player() {
                        // Player id is present. Check if it matches the event's rights.
                        if !self.rights().check(player, team_id) {
                            return Err(WeaselError::AuthenticationError(
                                Some(player),
                                team_id.clone(),
                            ));
                        }
                    } else {
                        // No player id present.
                        return Err(WeaselError::MissingAuthentication);
                    }
                }
            }
            EventRights::None => {}
        }
        // Promote verified event.
        let event = self.battle.promote(event.prototype());
        // Apply it.
        self.apply_event(event)
    }
}

impl<R: BattleRules + 'static> EventReceiver<R> for Server<R> {
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

/// A builder object to create a server.
pub struct ServerBuilder<R: BattleRules> {
    battle: Battle<R>,
    authentication: bool,
}

impl<R: BattleRules> ServerBuilder<R> {
    /// Enforce authentication on all events sent by clients.
    /// Clients must present a valid `PlayerId` each time they want to send an event.
    pub fn enforce_authentication(mut self) -> ServerBuilder<R> {
        self.authentication = true;
        self
    }

    /// Creates a new server.
    pub fn build(self) -> Server<R> {
        Server {
            battle: self.battle,
            client_sinks: MultiClientSink::new(),
            authentication: self.authentication,
        }
    }
}
