//! Collection of utilities.

use indexmap::IndexMap;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::hash::Hash;

/// Trait for an object that can provide an Id for itself.
pub trait Id {
    #[cfg(not(feature = "serialization"))]
    /// Type of the id value.
    type Id: Hash + Eq + Clone + Debug + Send;
    #[cfg(feature = "serialization")]
    /// Type of the id value.
    type Id: Hash + Eq + Clone + Debug + Send + Serialize + for<'a> Deserialize<'a>;

    /// Returns a reference to the current id.
    fn id(&self) -> &Self::Id;
}

/// Collects an iterator into an indexmap.
/// Subsequent values with same key are ignored.
pub(crate) fn collect_from_iter<I>(
    it: I,
) -> IndexMap<<<I as Iterator>::Item as Id>::Id, <I as Iterator>::Item>
where
    I: Iterator,
    <I as Iterator>::Item: Id,
{
    let mut map = IndexMap::new();
    for e in it {
        if !map.contains_key(e.id()) {
            map.insert(e.id().clone(), e);
        }
    }
    map
}

/// Creates a server from the given battlerules.
#[cfg(test)]
pub(crate) mod tests {
    use crate::battle::{Battle, BattleRules};
    use crate::creature::{CreateCreature, CreatureId};
    use crate::event::{DefaultOutput, DummyEvent, EventProcessor, EventTrigger};
    use crate::object::{CreateObject, ObjectId};
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

    /// Creates an object with default arguments.
    pub(crate) fn object<'a, R: BattleRules + 'static>(
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

    /// Dummy event.
    pub(crate) fn dummy<R, P>(processor: &mut P)
    where
        R: BattleRules + 'static,
        P: EventProcessor<R>,
    {
        assert_eq!(DummyEvent::trigger(processor).fire().err(), None);
    }
}
