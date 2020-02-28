use weasel::actor::{Actor, ActorRules};
use weasel::battle::{BattleRules, BattleState};
use weasel::battle_rules_with_actor;
use weasel::entity::EntityId;
use weasel::entropy::Entropy;
use weasel::event::{EventKind, EventQueue, EventTrigger};
use weasel::metric::WriteMetrics;
use weasel::rules::empty::EmptyAbility;
use weasel::space::MoveEntity;
use weasel::{battle_rules, rules::empty::*};

const TEAM_1_ID: u32 = 1;
const CREATURE_1_ID: u32 = 1;
const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);

#[derive(Default)]
pub struct CustomActorRules {}

impl<R: BattleRules + 'static> ActorRules<R> for CustomActorRules {
    type Ability = EmptyAbility;
    type AbilitiesSeed = u32;
    type Activation = u32;
    type AbilitiesAlteration = ();

    fn on_round_start(
        &self,
        _state: &BattleState<R>,
        actor: &dyn Actor<R>,
        mut event_queue: &mut Option<EventQueue<R>>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) {
        MoveEntity::trigger(
            &mut event_queue,
            actor.entity_id().clone(),
            actor.position().clone(),
        )
        .fire();
    }

    fn on_round_end(
        &self,
        _state: &BattleState<R>,
        actor: &dyn Actor<R>,
        mut event_queue: &mut Option<EventQueue<R>>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) {
        MoveEntity::trigger(
            &mut event_queue,
            actor.entity_id().clone(),
            actor.position().clone(),
        )
        .fire();
    }
}

battle_rules_with_actor! { CustomActorRules }

#[test]
fn round_start_and_end() {
    // Create a new creature.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    // Start a round, by the rules a move entity event should have been spawned.
    util::start_round(&mut server, &ENTITY_1_ID);
    {
        let events = server.battle().history().events();
        assert_eq!(events[2].kind(), EventKind::StartRound);
        assert_eq!(events[3].kind(), EventKind::MoveEntity);
    }
    // End the round, by the rules another move entity event should have been spawned.
    util::end_round(&mut server);
    {
        let events = server.battle().history().events();
        assert_eq!(events[4].kind(), EventKind::EndRound);
        assert_eq!(events[5].kind(), EventKind::MoveEntity);
    }
}
