use weasel::ability::ActivateAbility;
use weasel::actor::{Action, Actor, ActorRules, AlterAbilities};
use weasel::battle::{BattleController, BattleRules, BattleState};
use weasel::character::{AlterStatistics, Character, CharacterRules};
use weasel::entity::{EntityId, Transmutation};
use weasel::entropy::Entropy;
use weasel::event::{EventKind, EventQueue, EventTrigger};
use weasel::fight::{ApplyImpact, FightRules};
use weasel::metric::WriteMetrics;
use weasel::rules::ability::SimpleAbility;
use weasel::rules::statistic::SimpleStatistic;
use weasel::{battle_rules, rules::empty::*};

const TEAM_1_ID: u32 = 1;
const CREATURE_1_ID: u32 = 1;
const CREATURE_2_ID: u32 = 2;
const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
const ENTITY_2_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_2_ID);
const ABILITY_ID: u32 = 1;
const POWER: i32 = 1;
const HEALTH: i32 = 10;
const HEALTH_ID: &str = "health";

#[derive(Default)]
pub struct CustomCharacterRules {}

impl CharacterRules<CustomRules> for CustomCharacterRules {
    type CreatureId = u32;
    type ObjectId = ();
    type Statistic = SimpleStatistic<String, i32>;
    type StatisticsSeed = ();
    type StatisticsAlteration = i32;
    type Status = EmptyStatus;
    type StatusesAlteration = ();

    fn generate_statistics(
        &self,
        _: &Option<Self::StatisticsSeed>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> Box<dyn Iterator<Item = Self::Statistic>> {
        let v = vec![SimpleStatistic::new(HEALTH_ID.to_string(), HEALTH)];
        Box::new(v.into_iter())
    }

    fn alter_statistics(
        &self,
        character: &mut dyn Character<CustomRules>,
        alteration: &Self::StatisticsAlteration,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> Option<Transmutation> {
        let health = character.statistic(&HEALTH_ID.to_string()).unwrap().value();
        character
            .statistic_mut(&HEALTH_ID.to_string())
            .unwrap()
            .set_value(health - *alteration);
        None
    }
}

#[derive(Default)]
pub struct CustomActorRules {}

impl ActorRules<CustomRules> for CustomActorRules {
    type Ability = SimpleAbility<u32, i32>;
    type AbilitiesSeed = ();
    type Activation = ();
    type AbilitiesAlteration = i32;

    fn generate_abilities(
        &self,
        _: &Option<Self::AbilitiesSeed>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> Box<dyn Iterator<Item = Self::Ability>> {
        let v = vec![SimpleAbility::new(ABILITY_ID, POWER)];
        Box::new(v.into_iter())
    }

    fn activate(
        &self,
        _state: &BattleState<CustomRules>,
        action: Action<CustomRules>,
        mut event_queue: &mut Option<EventQueue<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        AlterAbilities::trigger(&mut event_queue, ENTITY_1_ID, 0).fire();
        ApplyImpact::trigger(&mut event_queue, action.ability.power() * 2).fire();
    }

    fn alter_abilities(
        &self,
        actor: &mut dyn Actor<CustomRules>,
        alteration: &Self::AbilitiesAlteration,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        actor
            .ability_mut(&ABILITY_ID)
            .unwrap()
            .set_power(*alteration);
    }
}

#[derive(Default)]
pub struct CustomFightRules {}

impl FightRules<CustomRules> for CustomFightRules {
    type Impact = i32;
    type Potency = ();

    fn apply_impact(
        &self,
        _state: &BattleState<CustomRules>,
        impact: &Self::Impact,
        mut event_queue: &mut Option<EventQueue<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        AlterStatistics::trigger(&mut event_queue, ENTITY_2_ID, *impact * 2).fire();
    }
}

battle_rules! {
    EmptyTeamRules,
    CustomCharacterRules,
    CustomActorRules,
    CustomFightRules,
    EmptyUserRules,
    EmptySpaceRules,
    EmptyRoundsRules,
    EmptyEntropyRules
}

#[test]
fn simple_attack() {
    // Create scenario.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    util::creature(&mut server, CREATURE_2_ID, TEAM_1_ID, ());
    // Start a round.
    util::start_round(&mut server, &ENTITY_1_ID);
    // Fire ability.
    assert_eq!(
        ActivateAbility::trigger(&mut server, ENTITY_1_ID, ABILITY_ID)
            .fire()
            .err(),
        None
    );
    // Check outcome of ability.
    // Attacker should have his ability's power set to zero.
    let creature = server.battle().entities().creature(&CREATURE_1_ID).unwrap();
    assert_eq!(creature.ability(&ABILITY_ID).unwrap().power(), 0);
    // Defender should have received damage equal to twice the impact's power, which is
    // twice the ability's power (in total x4).
    let creature = server.battle().entities().creature(&CREATURE_2_ID).unwrap();
    assert_eq!(
        creature.statistic(&HEALTH_ID.to_string()).unwrap().value(),
        HEALTH - POWER * 4
    );
    // Check events origin.
    let events = server.battle().history().events();
    assert_eq!(events[4].kind(), EventKind::ActivateAbility);
    assert_eq!(events[5].kind(), EventKind::AlterAbilities);
    assert_eq!(events[5].origin(), Some(4));
    assert_eq!(events[6].kind(), EventKind::ApplyImpact);
    assert_eq!(events[6].origin(), Some(4));
    assert_eq!(events[7].kind(), EventKind::AlterStatistics);
    assert_eq!(events[7].origin(), Some(6));
}

#[test]
fn default_works() {
    battle_rules! {}
    let mut server = util::server(CustomRules::new());
    // ApplyImpact with default rules does not return an error.
    assert_eq!(ApplyImpact::trigger(&mut server, ()).fire().err(), None);
}
