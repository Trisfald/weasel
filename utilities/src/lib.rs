use weasel::battle::{Battle, BattleRules};
use weasel::client::Client;
use weasel::creature::{CreateCreature, CreatureId};
use weasel::entity::EntityId;
use weasel::event::{DefaultOutput, DummyEvent, EventProcessor, EventTrigger, ServerSink};
use weasel::object::{CreateObject, ObjectId};
use weasel::round::{EndTurn, StartTurn};
use weasel::server::Server;
use weasel::space::Position;
use weasel::team::{CreateTeam, TeamId};

/// Creates a server from the given battlerules.
pub fn server<R: BattleRules + 'static>(rules: R) -> Server<R> {
    let battle = Battle::builder(rules).build();
    Server::builder(battle).build()
}

/// Creates a client from the given battlerules.
pub fn client<R, S>(rules: R, server_sink: S) -> Client<R>
where
    R: BattleRules + 'static,
    S: ServerSink<R> + 'static + Send,
{
    let battle = Battle::builder(rules).build();
    Client::builder(battle, Box::new(server_sink)).build()
}

/// Creates a team with default arguments.
pub fn team<R, P>(processor: &mut P, id: TeamId<R>)
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    assert_eq!(CreateTeam::trigger(processor, id).fire().err(), None);
}

/// Creates a creature with default arguments.
pub fn creature<'a, R, P>(
    processor: &'a mut P,
    creature_id: CreatureId<R>,
    team_id: TeamId<R>,
    position: Position<R>,
) where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    assert_eq!(
        CreateCreature::trigger(processor, creature_id, team_id, position)
            .fire()
            .err(),
        None
    );
}

/// Creates an object with default arguments.
pub fn object<'a, R: BattleRules + 'static>(
    server: &'a mut Server<R>,
    object_id: ObjectId<R>,
    position: Position<R>,
) {
    assert_eq!(
        CreateObject::trigger(server, object_id, position)
            .fire()
            .err(),
        None
    );
}

/// Starts a turn with the given entity.
pub fn start_turn<'a, R, P>(processor: &'a mut P, id: &EntityId<R>)
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    assert_eq!(StartTurn::trigger(processor, id.clone()).fire().err(), None);
}

/// Ends the turn.
pub fn end_turn<R, P>(processor: &mut P)
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    assert_eq!(EndTurn::trigger(processor).fire().err(), None);
}

/// Dummy event.
pub fn dummy<R, P>(processor: &mut P)
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    assert_eq!(DummyEvent::trigger(processor).fire().err(), None);
}
