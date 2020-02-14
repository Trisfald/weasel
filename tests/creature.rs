use weasel::actor::ActorRules;
use weasel::battle::BattleRules;
use weasel::character::{AlterStatistics, Character, CharacterRules};
use weasel::creature::{CreateCreature, RemoveCreature};
use weasel::entity::{EntityId, Transmutation};
use weasel::entropy::Entropy;
use weasel::event::EventTrigger;
use weasel::metric::{system::*, WriteMetrics};
use weasel::round::RoundState;
use weasel::rules::empty::{EmptyAbility, EmptyStat};
use weasel::rules::statistic::SimpleStatistic;
use weasel::user::UserMetricId;
use weasel::WeaselError;
use weasel::{battle_rules, rules::empty::*};
use weasel::{battle_rules_with_actor, battle_rules_with_character};

static TEAM_1_ID: u32 = 1;
static TEAM_5_ID: u32 = 5;
static CREATURE_1_ID: u32 = 1;
static CREATURE_5_ID: u32 = 5;

#[test]
fn new_creature() {
    battle_rules! {}
    // Check creature creation.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    for i in 0..2 {
        util::creature(&mut server, i, TEAM_1_ID, ());
        assert!(server.battle().entities().creature(&i).is_some());
    }
    // Check metrics.
    assert_eq!(
        server.battle().metrics().system_u64(CREATURES_CREATED),
        Some(2)
    );
    // Check team exists.
    assert_eq!(
        CreateCreature::trigger(&mut server, CREATURE_5_ID, TEAM_5_ID, ())
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::TeamNotFound(TEAM_5_ID))
    );
    assert!(server
        .battle()
        .entities()
        .creature(&CREATURE_5_ID)
        .is_none());
    // Check creature duplication.
    util::team(&mut server, TEAM_5_ID);
    assert_eq!(
        CreateCreature::trigger(&mut server, 0, TEAM_5_ID, ())
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::DuplicatedCreature(0))
    );
    assert!(server.battle().entities().creature(&0).is_some());
}

#[derive(Default)]
pub struct CustomCharacterRules {}

impl<R: BattleRules + 'static> CharacterRules<R> for CustomCharacterRules {
    type CreatureId = u32;
    type Statistic = EmptyStat;
    type StatisticsSeed = u32;
    type StatisticsAlteration = ();

    fn generate_statistics(
        &self,
        seed: &Option<Self::StatisticsSeed>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) -> Box<dyn Iterator<Item = Self::Statistic>> {
        if let Some(seed) = seed {
            let v = vec![EmptyStat { id: *seed }];
            Box::new(v.into_iter())
        } else {
            Box::new(std::iter::empty())
        }
    }
}

#[derive(Default)]
pub struct CustomActorRules {}

impl<R: BattleRules> ActorRules<R> for CustomActorRules {
    type Ability = EmptyAbility;
    type AbilitiesSeed = u32;
    type Activation = ();
    type AbilitiesAlteration = ();

    fn generate_abilities(
        &self,
        seed: &Option<Self::AbilitiesSeed>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) -> Box<dyn Iterator<Item = Self::Ability>> {
        let seed = seed.unwrap();
        let v = vec![EmptyAbility { id: seed }];
        Box::new(v.into_iter())
    }
}

#[test]
fn statistics_generated() {
    battle_rules_with_character! { CustomCharacterRules }
    static SEED: u32 = 5;
    // Create a new creature.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    let mut trigger = CreateCreature::trigger(&mut server, CREATURE_5_ID, TEAM_1_ID, ());
    let result = trigger.statistics_seed(SEED).fire();
    assert_eq!(result.err(), None);
    // Check that stats are generated correctly.
    let creature = server.battle().entities().creature(&CREATURE_5_ID).unwrap();
    let stats: Vec<_> = creature.statistics().collect();
    assert_eq!(stats, vec![&EmptyStat { id: SEED }]);
}

#[test]
fn abilities_generated() {
    battle_rules_with_actor! { CustomActorRules }
    static SEED: u32 = 5;
    // Create a new creature.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    let mut trigger = CreateCreature::trigger(&mut server, CREATURE_5_ID, TEAM_1_ID, ());
    let result = trigger.abilities_seed(SEED).fire();
    assert_eq!(result.err(), None);
    // Check that stats are generated correctly.
    let creature = server.battle().entities().creature(&CREATURE_5_ID).unwrap();
    let abilities: Vec<_> = creature.abilities().collect();
    assert_eq!(abilities, vec![&EmptyAbility { id: SEED }]);
}

#[derive(Default)]
pub struct CharacterRulesWithMetrics {}

impl<R: BattleRules + 'static> CharacterRules<R> for CharacterRulesWithMetrics
where
    UserMetricId<R>: Default,
{
    type CreatureId = u32;
    type Statistic = SimpleStatistic<u32, u64>;
    type StatisticsSeed = u64;
    type StatisticsAlteration = ();

    fn generate_statistics(
        &self,
        seed: &Option<Self::StatisticsSeed>,
        _entropy: &mut Entropy<R>,
        metrics: &mut WriteMetrics<R>,
    ) -> Box<dyn Iterator<Item = Self::Statistic>> {
        let seed = seed.unwrap();
        let v = vec![SimpleStatistic::new(0, seed)];
        metrics
            .add_user_u64(UserMetricId::<R>::default(), seed)
            .unwrap();
        Box::new(v.into_iter())
    }
}

#[test]
fn user_metrics() {
    // Create a scenario.
    battle_rules_with_character! { CharacterRulesWithMetrics }
    static SEED: u64 = 5;
    static TOTAL_STAT_VALUE: u64 = 5 * 2;
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    // Create two creatures each one with a stat of value 5.
    let mut trigger = CreateCreature::trigger(&mut server, CREATURE_5_ID, TEAM_1_ID, ());
    assert_eq!(trigger.statistics_seed(SEED).fire().err(), None);
    let mut trigger = CreateCreature::trigger(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    assert_eq!(trigger.statistics_seed(SEED).fire().err(), None);
    // Check if metric for total stat value is correct.
    assert_eq!(
        server.battle().metrics().user_u64(0),
        Some(TOTAL_STAT_VALUE)
    );
}

#[test]
fn remove_creature() {
    battle_rules! {}
    static ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
    // Create a battle with one creature.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    // Remove creature should fail if the creature doesn't exist.
    assert_eq!(
        RemoveCreature::trigger(&mut server, CREATURE_5_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::CreatureNotFound(CREATURE_5_ID))
    );
    // Remove the creature.
    assert_eq!(
        RemoveCreature::trigger(&mut server, CREATURE_1_ID)
            .fire()
            .err(),
        None
    );
    // Check that the creature was removed.
    let entities = server.battle().entities();
    assert!(entities.creature(&CREATURE_1_ID).is_none());
    assert!(!entities
        .team(&TEAM_1_ID)
        .unwrap()
        .creatures()
        .any(|e| *e == CREATURE_1_ID));
    // Create another creature and start a round.
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    util::start_round(&mut server, &ENTITY_1_ID);
    // Remove the creature.
    assert_eq!(
        RemoveCreature::trigger(&mut server, CREATURE_1_ID)
            .fire()
            .err(),
        None
    );
    // Check that the creature was removed and the round ended.
    let entities = server.battle().entities();
    assert!(entities.creature(&CREATURE_1_ID).is_none());
    assert!(!entities
        .team(&TEAM_1_ID)
        .unwrap()
        .creatures()
        .any(|e| *e == CREATURE_1_ID));
    assert_eq!(*server.battle().rounds().state(), RoundState::<_>::Ready);
}

#[test]
fn remove_creature_on_alter() {
    #[derive(Default)]
    struct CustomCharacterRules {}

    impl<R: BattleRules + 'static> CharacterRules<R> for CustomCharacterRules {
        type CreatureId = u32;
        type Statistic = EmptyStat;
        type StatisticsSeed = ();
        type StatisticsAlteration = ();

        fn alter(
            &self,
            _character: &mut dyn Character<R>,
            _alteration: &Self::StatisticsAlteration,
            _entropy: &mut Entropy<R>,
            _metrics: &mut WriteMetrics<R>,
        ) -> Option<Transmutation> {
            Some(Transmutation::REMOVAL)
        }
    }

    battle_rules_with_character! { CustomCharacterRules }
    static ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
    // Create a battle with one creature.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    // Fire an alter statistics event.
    assert_eq!(
        AlterStatistics::trigger(&mut server, ENTITY_1_ID, ())
            .fire()
            .err(),
        None
    );
    // Check that the creature was removed.
    let entities = server.battle().entities();
    assert!(entities.creature(&CREATURE_1_ID).is_none());
}
