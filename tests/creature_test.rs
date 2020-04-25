use std::cell::RefCell;
use std::collections::HashSet;
use weasel::ability::AbilityId;
use weasel::actor::{Actor, ActorRules, RegenerateAbilities};
use weasel::battle::{BattleRules, BattleState};
use weasel::character::{
    AlterStatistics, Character, CharacterRules, RegenerateStatistics, StatisticId,
};
use weasel::creature::{CreateCreature, RemoveCreature};
use weasel::entity::{EntityId, Transmutation};
use weasel::entropy::Entropy;
use weasel::event::{EventQueue, EventTrigger};
use weasel::metric::{system::*, WriteMetrics};
use weasel::round::{RoundState, RoundsRules};
use weasel::rules::empty::{EmptyAbility, EmptyStat};
use weasel::rules::{ability::SimpleAbility, statistic::SimpleStatistic};
use weasel::space::{PositionClaim, SpaceRules};
use weasel::user::UserMetricId;
use weasel::{
    battle_rules, battle_rules_with_actor, battle_rules_with_character, rules::empty::*,
    WeaselError, WeaselResult,
};

const TEAM_1_ID: u32 = 1;
const TEAM_5_ID: u32 = 5;
const CREATURE_1_ID: u32 = 1;
const CREATURE_5_ID: u32 = 5;
const CREATURE_ERR_ID: u32 = 99;

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

#[test]
fn statistics_generated() {
    #[derive(Default)]
    pub struct CustomCharacterRules {}

    impl<R: BattleRules + 'static> CharacterRules<R> for CustomCharacterRules {
        type CreatureId = u32;
        type ObjectId = ();
        type Statistic = EmptyStat;
        type StatisticsSeed = u32;
        type StatisticsAlteration = ();
        type Status = EmptyStatus;
        type StatusesAlteration = ();

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

    battle_rules_with_character! { CustomCharacterRules }
    const SEED: u32 = 5;
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
fn regenerate_statistics() {
    #[derive(Default)]
    pub struct CustomCharacterRules {}

    impl<R: BattleRules + 'static> CharacterRules<R> for CustomCharacterRules {
        type CreatureId = u32;
        type ObjectId = ();
        type Statistic = SimpleStatistic<u32, u32>;
        // Vec with pair (id, value).
        type StatisticsSeed = Vec<(u32, u32)>;
        type StatisticsAlteration = ();
        type Status = EmptyStatus;
        type StatusesAlteration = ();

        fn generate_statistics(
            &self,
            seed: &Option<Self::StatisticsSeed>,
            _entropy: &mut Entropy<R>,
            _metrics: &mut WriteMetrics<R>,
        ) -> Box<dyn Iterator<Item = Self::Statistic>> {
            if let Some(seed) = seed {
                let mut v = Vec::new();
                for (id, value) in seed {
                    v.push(SimpleStatistic::new(*id, *value));
                }
                Box::new(v.into_iter())
            } else {
                Box::new(std::iter::empty())
            }
        }
    }

    battle_rules_with_character! { CustomCharacterRules }

    static STAT_1_ID: StatisticId<CustomRules> = 1;
    static STAT_2_ID: StatisticId<CustomRules> = 2;
    static STAT_3_ID: StatisticId<CustomRules> = 3;
    const STAT_VALUE: u32 = 10;
    const STAT_ERR_VALUE: u32 = 0;
    const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
    const ENTITY_ERR_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_ERR_ID);
    // Create a new creature with two statistics.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    assert_eq!(
        CreateCreature::trigger(&mut server, CREATURE_1_ID, TEAM_1_ID, ())
            .statistics_seed(vec![(STAT_1_ID, STAT_VALUE), (STAT_2_ID, STAT_VALUE)])
            .fire()
            .err(),
        None
    );
    assert_eq!(
        server
            .battle()
            .entities()
            .character(&ENTITY_1_ID)
            .unwrap()
            .statistics()
            .count(),
        2
    );
    // Regenerate should fail for non existing entities.
    assert_eq!(
        RegenerateStatistics::trigger(&mut server, ENTITY_ERR_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::EntityNotFound(ENTITY_ERR_ID))
    );
    // Regenerate statistics.
    assert_eq!(
        RegenerateStatistics::trigger(&mut server, ENTITY_1_ID)
            .seed(vec![(STAT_1_ID, STAT_ERR_VALUE), (STAT_3_ID, STAT_VALUE)])
            .fire()
            .err(),
        None
    );
    let creature = server.battle().entities().character(&ENTITY_1_ID).unwrap();
    assert_eq!(creature.statistics().count(), 2);
    // Verify that one statistic was left untouched.
    assert_eq!(
        creature.statistic(&STAT_1_ID),
        Some(&SimpleStatistic::new(STAT_1_ID, STAT_VALUE))
    );
    // Verify that one statistic was removed.
    assert!(creature.statistic(&STAT_2_ID).is_none());
    // Verify that one statistic was added.
    assert_eq!(
        creature.statistic(&STAT_3_ID),
        Some(&SimpleStatistic::new(STAT_3_ID, STAT_VALUE))
    );
}

#[test]
fn abilities_generated() {
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

    battle_rules_with_actor! { CustomActorRules }
    const SEED: u32 = 5;
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

#[test]
fn regenerate_abilities() {
    #[derive(Default)]
    pub struct CustomActorRules {}

    impl<R: BattleRules> ActorRules<R> for CustomActorRules {
        type Ability = SimpleAbility<u32, u32>;
        // Vec with pair (id, value).
        type AbilitiesSeed = Vec<(u32, u32)>;
        type Activation = ();
        type AbilitiesAlteration = ();

        fn generate_abilities(
            &self,
            seed: &Option<Self::AbilitiesSeed>,
            _entropy: &mut Entropy<R>,
            _metrics: &mut WriteMetrics<R>,
        ) -> Box<dyn Iterator<Item = Self::Ability>> {
            if let Some(seed) = seed {
                let mut v = Vec::new();
                for (id, value) in seed {
                    v.push(SimpleAbility::new(*id, *value));
                }
                Box::new(v.into_iter())
            } else {
                Box::new(std::iter::empty())
            }
        }
    }

    battle_rules_with_actor! { CustomActorRules }

    static ABILITY_1_ID: AbilityId<CustomRules> = 1;
    static ABILITY_2_ID: AbilityId<CustomRules> = 2;
    static ABILITY_3_ID: AbilityId<CustomRules> = 3;
    const ABILITY_VALUE: u32 = 10;
    const ABILITY_ERR_VALUE: u32 = 0;
    const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
    const ENTITY_ERR_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_ERR_ID);
    // Create a new creature with two abilities.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    assert_eq!(
        CreateCreature::trigger(&mut server, CREATURE_1_ID, TEAM_1_ID, ())
            .abilities_seed(vec![
                (ABILITY_1_ID, ABILITY_VALUE),
                (ABILITY_2_ID, ABILITY_VALUE)
            ])
            .fire()
            .err(),
        None
    );
    assert_eq!(
        server
            .battle()
            .entities()
            .actor(&ENTITY_1_ID)
            .unwrap()
            .abilities()
            .count(),
        2
    );
    // Regenerate should fail for non existing entities.
    assert_eq!(
        RegenerateAbilities::trigger(&mut server, ENTITY_ERR_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::EntityNotFound(ENTITY_ERR_ID))
    );
    // Regenerate abilities.
    assert_eq!(
        RegenerateAbilities::trigger(&mut server, ENTITY_1_ID)
            .seed(vec![
                (ABILITY_1_ID, ABILITY_ERR_VALUE),
                (ABILITY_3_ID, ABILITY_VALUE)
            ])
            .fire()
            .err(),
        None
    );
    let creature = server.battle().entities().actor(&ENTITY_1_ID).unwrap();
    assert_eq!(creature.abilities().count(), 2);
    // Verify that one ability was left untouched.
    assert_eq!(
        creature.ability(&ABILITY_1_ID),
        Some(&SimpleAbility::new(ABILITY_1_ID, ABILITY_VALUE))
    );
    // Verify that one ability was removed.
    assert!(creature.ability(&ABILITY_2_ID).is_none());
    // Verify that one ability was added.
    assert_eq!(
        creature.ability(&ABILITY_3_ID),
        Some(&SimpleAbility::new(ABILITY_3_ID, ABILITY_VALUE))
    );
}

#[test]
fn user_metrics() {
    #[derive(Default)]
    pub struct CharacterRulesWithMetrics {}

    impl<R: BattleRules + 'static> CharacterRules<R> for CharacterRulesWithMetrics
    where
        UserMetricId<R>: Default,
    {
        type CreatureId = u32;
        type ObjectId = ();
        type Statistic = SimpleStatistic<u32, u64>;
        type StatisticsSeed = u64;
        type StatisticsAlteration = ();
        type Status = EmptyStatus;
        type StatusesAlteration = ();

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

    battle_rules_with_character! { CharacterRulesWithMetrics }
    const SEED: u64 = 5;
    const TOTAL_STAT_VALUE: u64 = 5 * 2;
    // Create a battle.
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
    #[derive(Default)]
    pub struct CustomRoundsRules {}

    impl RoundsRules<CustomRules> for CustomRoundsRules {
        type RoundsSeed = ();
        // Collection to keep track of how many times on_actor_removed is called
        // and with what entity_id.
        type RoundsModel = Vec<EntityId<CustomRules>>;

        fn generate_model(&self, _: &Option<Self::RoundsSeed>) -> Self::RoundsModel {
            Vec::new()
        }

        fn on_actor_removed(
            &self,
            model: &mut Self::RoundsModel,
            actor: &dyn Actor<CustomRules>,
            _: &mut Entropy<CustomRules>,
            _: &mut WriteMetrics<CustomRules>,
        ) {
            model.push(actor.entity_id().clone());
        }
    }

    #[derive(Default)]
    struct CustomSpaceRules {}

    impl SpaceRules<CustomRules> for CustomSpaceRules {
        type Position = u32;
        type SpaceSeed = ();
        type SpaceModel = HashSet<Self::Position>;
        type SpaceAlteration = ();

        fn generate_model(&self, _: &Option<Self::SpaceSeed>) -> Self::SpaceModel {
            HashSet::new()
        }

        fn check_move<'a>(
            &self,
            model: &Self::SpaceModel,
            _claim: PositionClaim<'a, CustomRules>,
            position: &Self::Position,
        ) -> WeaselResult<(), CustomRules> {
            if !model.contains(position) {
                Ok(())
            } else {
                Err(WeaselError::GenericError)
            }
        }

        fn move_entity<'a>(
            &self,
            model: &mut Self::SpaceModel,
            claim: PositionClaim<'a, CustomRules>,
            position: Option<&Self::Position>,
            _metrics: &mut WriteMetrics<CustomRules>,
        ) {
            if let Some(position) = position {
                if let PositionClaim::Movement(entity) = claim {
                    model.remove(entity.position());
                }
                model.insert(*position);
            } else if let PositionClaim::Movement(entity) = claim {
                model.remove(entity.position());
            }
        }
    }

    battle_rules! {
        EmptyTeamRules,
        EmptyCharacterRules,
        EmptyActorRules,
        EmptyFightRules,
        EmptyUserRules,
        CustomSpaceRules,
        CustomRoundsRules,
        EmptyEntropyRules
    }
    const POSITION_1: u32 = 1;
    const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
    // Create a battle with one creature.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, POSITION_1);
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
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, POSITION_1);
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
    // Position must have been freed.
    assert!(!server.battle().space().model().contains(&POSITION_1));
    // Creature with id 1 must have been removed twice from the round model.
    assert_eq!(
        server.battle().rounds().model(),
        &vec![
            EntityId::Creature(CREATURE_1_ID),
            EntityId::Creature(CREATURE_1_ID)
        ]
    );
}

#[test]
fn remove_creature_on_alter() {
    #[derive(Default)]
    struct CustomCharacterRules {}

    impl<R: BattleRules + 'static> CharacterRules<R> for CustomCharacterRules {
        type CreatureId = u32;
        type ObjectId = ();
        type Statistic = EmptyStat;
        type StatisticsSeed = ();
        type StatisticsAlteration = ();
        type Status = EmptyStatus;
        type StatusesAlteration = ();

        fn alter_statistics(
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
    const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
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

#[test]
fn character_existence_callbacks() {
    #[derive(Default)]
    pub struct CustomCharacterRules {
        added: RefCell<u32>,
        removed: RefCell<u32>,
    }

    impl<R: BattleRules + 'static> CharacterRules<R> for CustomCharacterRules {
        type CreatureId = u32;
        type ObjectId = ();
        type Statistic = EmptyStat;
        type StatisticsSeed = ();
        type StatisticsAlteration = ();
        type Status = EmptyStatus;
        type StatusesAlteration = ();

        fn on_character_added(
            &self,
            _state: &BattleState<R>,
            _character: &dyn Character<R>,
            _event_queue: &mut Option<EventQueue<R>>,
            _entropy: &mut Entropy<R>,
            _metrics: &mut WriteMetrics<R>,
        ) {
            *self.added.borrow_mut() += 1;
        }

        fn on_character_transmuted(
            &self,
            _state: &BattleState<R>,
            _character: &dyn Character<R>,
            _transmutation: Transmutation,
            _event_queue: &mut Option<EventQueue<R>>,
            _entropy: &mut Entropy<R>,
            _metrics: &mut WriteMetrics<R>,
        ) {
            *self.removed.borrow_mut() += 1;
        }
    }

    battle_rules_with_character! { CustomCharacterRules }
    // Create a new creature.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    // Check that the callback was invoked.
    assert_eq!(*server.battle().rules().character_rules().added.borrow(), 1);
    // Remove the creature.
    assert_eq!(
        RemoveCreature::trigger(&mut server, CREATURE_1_ID)
            .fire()
            .err(),
        None
    );
    // Check that the callback was invoked.
    assert_eq!(
        *server.battle().rules().character_rules().removed.borrow(),
        1
    );
}
