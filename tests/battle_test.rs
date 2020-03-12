use weasel::ability::ActivateAbility;
use weasel::actor::{Action, ActorRules};
use weasel::battle::{BattlePhase, BattleRules, BattleState, EndBattle};
use weasel::battle_rules_with_actor;
use weasel::entity::EntityId;
use weasel::entropy::Entropy;
use weasel::event::{DummyEvent, EventQueue, EventTrigger};
use weasel::metric::WriteMetrics;
use weasel::round::{EndRound, StartRound};
use weasel::rules::empty::EmptyAbility;
use weasel::WeaselError;
use weasel::{battle_rules, rules::empty::*};

const TEAM_1_ID: u32 = 1;
const CREATURE_1_ID: u32 = 1;
const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
const ABILITY_ID: u32 = 1;

#[derive(Default)]
pub struct CustomActorRules {}

impl<R: BattleRules + 'static> ActorRules<R> for CustomActorRules {
    type Ability = EmptyAbility;
    type AbilitiesSeed = u32;
    type Activation = u32;
    type AbilitiesAlteration = ();

    fn generate_abilities(
        &self,
        _: &Option<Self::AbilitiesSeed>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) -> Box<dyn Iterator<Item = Self::Ability>> {
        let v = vec![EmptyAbility { id: ABILITY_ID }];
        Box::new(v.into_iter())
    }

    fn activate(
        &self,
        _state: &BattleState<R>,
        _action: Action<R>,
        mut event_queue: &mut Option<EventQueue<R>>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) {
        DummyEvent::trigger(&mut event_queue).fire();
        EndBattle::trigger(&mut event_queue).fire();
        EndRound::trigger(&mut event_queue).fire();
    }
}

battle_rules_with_actor! { CustomActorRules }

#[test]
fn end_battle() {
    // Create the scenario.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    assert_eq!(server.battle().phase(), BattlePhase::Started);
    // End the battle and checks that new events aren't accepted.
    assert_eq!(EndBattle::trigger(&mut server).fire().err(), None);
    assert_eq!(
        StartRound::trigger(&mut server, ENTITY_1_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::BattleEnded)
    );
    assert_eq!(server.battle().phase(), BattlePhase::Ended);
}

#[test]
fn end_battle_during_events() {
    // Create the scenario.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    assert_eq!(server.battle().phase(), BattlePhase::Started);
    util::start_round(&mut server, &ENTITY_1_ID);
    // Fire an ability that creates a dummy, an endbattle and an endround.
    // Last event should have been rejected.
    assert_eq!(
        ActivateAbility::trigger(&mut server, ENTITY_1_ID, ABILITY_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::BattleEnded)
    );
    assert_eq!(server.battle().phase(), BattlePhase::Ended);
}
