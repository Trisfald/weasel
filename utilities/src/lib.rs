use weasel::battle::{Battle, BattleRules};
use weasel::client::Client;
use weasel::creature::{CreateCreature, CreatureId};
use weasel::entity::EntityId;
use weasel::event::{DefaultOutput, DummyEvent, EventProcessor, EventTrigger, ServerSink};
use weasel::round::{EndRound, StartRound};
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
    S: ServerSink<R> + 'static,
{
    let battle = Battle::builder(rules).build();
    Client::builder(battle, Box::new(server_sink)).build()
}

/// Creates a team with default arguments.
pub fn team<'a, R, P>(processor: &'a mut P, id: TeamId<R>)
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

/// Starts a round with the given entity.
pub fn start_round<'a, R, P>(processor: &'a mut P, id: &EntityId<R>)
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    assert_eq!(
        StartRound::trigger(processor, id.clone()).fire().err(),
        None
    );
}

/// Ends the round.
pub fn end_round<'a, R, P>(processor: &'a mut P)
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    assert_eq!(EndRound::trigger(processor).fire().err(), None);
}

/// Dummy event.
pub fn dummy<'a, R, P>(processor: &'a mut P)
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    assert_eq!(DummyEvent::trigger(processor).fire().err(), None);
}
