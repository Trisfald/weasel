//! Collection of utilities.

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::hash::Hash;

/// Trait for an object that can provide an Id for itself.
pub trait Id {
    #[cfg(not(feature = "serialization"))]
    /// Type of the id value.
    type Id: Hash + Eq + Clone + Debug;
    #[cfg(feature = "serialization")]
    /// Type of the id value.
    type Id: Hash + Eq + Clone + Debug + Serialize + for<'a> Deserialize<'a>;

    /// Returns a reference to the current id.
    fn id(&self) -> &Self::Id;
}

/// Creates a server from the given battlerules.
#[cfg(test)]
pub(crate) mod tests {
    use crate::battle::{Battle, BattleRules};
    use crate::creature::{CreateCreature, CreatureId};
    use crate::event::{DefaultOutput, DummyEvent, EventProcessor, EventTrigger};
    use crate::server::Server;
    use crate::space::Position;
    use crate::team::{CreateTeam, TeamId};

    pub(crate) fn server<R: BattleRules + 'static>(rules: R) -> Server<R> {
        let battle = Battle::builder(rules).build();
        Server::builder(battle).build()
    }

    /// Creates a team with default arguments.
    pub(crate) fn team<'a, R: BattleRules + 'static>(server: &'a mut Server<R>, id: TeamId<R>) {
        assert_eq!(CreateTeam::trigger(server, id).fire().err(), None);
    }

    /// Creates a creature with default arguments.
    pub(crate) fn creature<'a, R: BattleRules + 'static>(
        server: &'a mut Server<R>,
        creature_id: CreatureId<R>,
        team_id: TeamId<R>,
        position: Position<R>,
    ) {
        assert_eq!(
            CreateCreature::trigger(server, creature_id, team_id, position)
                .fire()
                .err(),
            None
        );
    }

    /// Dummy event.
    pub(crate) fn dummy<'a, R, P>(processor: &'a mut P)
    where
        R: BattleRules + 'static,
        P: EventProcessor<R>,
    {
        assert_eq!(DummyEvent::trigger(processor).fire().err(), None);
    }
}
