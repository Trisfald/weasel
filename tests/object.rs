use std::collections::HashSet;
use weasel::ability::ActivateAbility;
use weasel::battle::BattleRules;
use weasel::character::{
    AlterStatistics, Character, CharacterRules, RegenerateStatistics, StatisticId,
};
use weasel::entity::{EntityId, Transmutation};
use weasel::entropy::Entropy;
use weasel::event::EventTrigger;
use weasel::metric::{system::*, WriteMetrics};
use weasel::object::{CreateObject, RemoveObject};
use weasel::round::StartRound;
use weasel::rules::empty::EmptyStat;
use weasel::rules::statistic::SimpleStatistic;
use weasel::space::{PositionClaim, SpaceRules};
use weasel::{
    battle_rules, battle_rules_with_character, battle_rules_with_space, rules::empty::*,
    WeaselError, WeaselResult,
};

const OBJECT_1_ID: u32 = 1;
const OBJECT_2_ID: u32 = 2;
const OBJECT_ERR_ID: u32 = 99;

#[test]
fn new_object() {
    battle_rules! {}
    // Check object creation.
    let mut server = util::server(CustomRules::new());
    for i in 0..2 {
        util::object(&mut server, i, ());
        assert!(server.battle().entities().object(&i).is_some());
    }
    // Check metrics.
    assert_eq!(
        server.battle().metrics().system_u64(OBJECTS_CREATED),
        Some(2)
    );
    // Check object duplication.
    assert_eq!(
        CreateObject::trigger(&mut server, 0, ())
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::DuplicatedObject(0))
    );
    assert!(server.battle().entities().object(&0).is_some());
}

#[test]
fn object_cannot_act() {
    battle_rules! {}
    const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Object(OBJECT_1_ID);
    const ABILITY_1_ID: u32 = 1;
    // Create a battle with one object.
    let mut server = util::server(CustomRules::new());
    util::object(&mut server, OBJECT_1_ID, ());
    // Verify that objects can't start rounds.
    assert_eq!(
        StartRound::trigger(&mut server, ENTITY_1_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::NotAnActor(ENTITY_1_ID))
    );
    // Verify that objects can't activate abilities.
    assert_eq!(
        ActivateAbility::trigger(&mut server, ENTITY_1_ID, ABILITY_1_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::NotAnActor(ENTITY_1_ID))
    );
}

#[test]
fn statistics_generated() {
    #[derive(Default)]
    pub struct CustomCharacterRules {}

    impl<R: BattleRules + 'static> CharacterRules<R> for CustomCharacterRules {
        type CreatureId = ();
        type ObjectId = u32;
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

    battle_rules_with_character! { CustomCharacterRules }
    const SEED: u32 = 5;
    // Create a new object.
    let mut server = util::server(CustomRules::new());
    assert_eq!(
        CreateObject::trigger(&mut server, OBJECT_2_ID, ())
            .statistics_seed(SEED)
            .fire()
            .err(),
        None
    );
    // Check that stats are generated correctly.
    let object = server.battle().entities().object(&OBJECT_2_ID).unwrap();
    let stats: Vec<_> = object.statistics().collect();
    assert_eq!(stats, vec![&EmptyStat { id: SEED }]);
}

#[test]
fn regenerate_statistics() {
    #[derive(Default)]
    pub struct CustomCharacterRules {}

    impl<R: BattleRules + 'static> CharacterRules<R> for CustomCharacterRules {
        type CreatureId = ();
        type ObjectId = u32;
        type Statistic = SimpleStatistic<u32, u32>;
        // Vec with pair (id, value).
        type StatisticsSeed = Vec<(u32, u32)>;
        type StatisticsAlteration = ();

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

    const STAT_1_ID: StatisticId<CustomRules> = 1;
    const STAT_2_ID: StatisticId<CustomRules> = 2;
    const STAT_3_ID: StatisticId<CustomRules> = 3;
    const STAT_VALUE: u32 = 10;
    const STAT_ERR_VALUE: u32 = 0;
    const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Object(OBJECT_1_ID);
    const ENTITY_ERR_ID: EntityId<CustomRules> = EntityId::Object(OBJECT_ERR_ID);
    // Create a new object with two statistics.
    let mut server = util::server(CustomRules::new());
    assert_eq!(
        CreateObject::trigger(&mut server, OBJECT_1_ID, ())
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
    let object = server.battle().entities().character(&ENTITY_1_ID).unwrap();
    assert_eq!(object.statistics().count(), 2);
    // Verify that one statistic was left untouched.
    assert_eq!(
        object.statistic(&STAT_1_ID),
        Some(&SimpleStatistic::new(STAT_1_ID, STAT_VALUE))
    );
    // Verify that one statistic was removed.
    assert!(object.statistic(&STAT_2_ID).is_none());
    // Verify that one statistic was added.
    assert_eq!(
        object.statistic(&STAT_3_ID),
        Some(&SimpleStatistic::new(STAT_3_ID, STAT_VALUE))
    );
}

#[test]
fn remove_object() {
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

    battle_rules_with_space! { CustomSpaceRules }
    const POSITION_1: u32 = 1;
    // Create a battle with one object.
    let mut server = util::server(CustomRules::new());
    util::object(&mut server, OBJECT_1_ID, POSITION_1);
    // Remove object should fail if the object doesn't exist.
    assert_eq!(
        RemoveObject::trigger(&mut server, OBJECT_2_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::ObjectNotFound(OBJECT_2_ID))
    );
    // Remove the object.
    assert_eq!(
        RemoveObject::trigger(&mut server, OBJECT_1_ID).fire().err(),
        None
    );
    // Check that the object was removed.
    let entities = server.battle().entities();
    assert!(entities.object(&OBJECT_1_ID).is_none());
    // Position must have been freed.
    assert!(!server.battle().space().model().contains(&POSITION_1));
}

#[test]
fn remove_object_on_alter() {
    #[derive(Default)]
    struct CustomCharacterRules {}

    impl<R: BattleRules + 'static> CharacterRules<R> for CustomCharacterRules {
        type CreatureId = ();
        type ObjectId = u32;
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
    const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Object(OBJECT_1_ID);
    // Create a battle with one object.
    let mut server = util::server(CustomRules::new());
    util::object(&mut server, OBJECT_1_ID, ());
    // Fire an alter statistics event.
    assert_eq!(
        AlterStatistics::trigger(&mut server, ENTITY_1_ID, ())
            .fire()
            .err(),
        None
    );
    // Check that the object was removed.
    let entities = server.battle().entities();
    assert!(entities.object(&OBJECT_1_ID).is_none());
}
