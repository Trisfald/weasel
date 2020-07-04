use std::cell::RefCell;
use weasel::battle::{Battle, BattleController, BattleRules, BattleState};
use weasel::character::{AlterStatistics, Character, CharacterRules};
use weasel::entity::{EntityId, Transmutation};
use weasel::entropy::Entropy;
use weasel::event::{EventKind, EventQueue, EventTrigger, LinkedQueue};
use weasel::fight::FightRules;
use weasel::metric::WriteMetrics;
use weasel::round::EnvironmentRound;
use weasel::rules::statistic::SimpleStatistic;
use weasel::rules::status::SimpleStatus;
use weasel::status::{
    AlterStatuses, Application, AppliedStatus, ClearStatus, InflictStatus, Potency, Status,
    StatusDuration, StatusId,
};
use weasel::{battle_rules, rules::empty::*, Server, WeaselError};

const TEAM_1_ID: u32 = 1;
const CREATURE_1_ID: u32 = 1;
const CREATURE_ERR_ID: u32 = 99;
const OBJECT_1_ID: u32 = 1;
const OBJECT_2_ID: u32 = 2;
const ENTITY_C1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
const ENTITY_O1_ID: EntityId<CustomRules> = EntityId::Object(OBJECT_1_ID);
const ENTITY_O2_ID: EntityId<CustomRules> = EntityId::Object(OBJECT_2_ID);
const ENTITY_ERR_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_ERR_ID);
const STATISTIC_ID: u32 = 1;
const STATISTIC_VALUE: i32 = 10;
const STATUS_1_ID: u32 = 1;
const STATUS_2_ID: u32 = 2;
const STATUS_ERR_ID: u32 = 99;
const STATUS_INTENSITY: i32 = 5;
const STATUS_DURATION: StatusDuration = 2;

#[derive(Default)]
pub struct CustomCharacterRules {
    unstackable_statuses: RefCell<bool>,
}

impl CharacterRules<CustomRules> for CustomCharacterRules {
    type CreatureId = u32;
    type ObjectId = u32;
    type Statistic = SimpleStatistic<u32, i32>;
    type StatisticsSeed = ();
    type StatisticsAlteration = i32;
    type Status = SimpleStatus<u32, i32>;
    type StatusesAlteration = i32;

    fn generate_statistics(
        &self,
        _seed: &Option<Self::StatisticsSeed>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> Box<dyn Iterator<Item = Self::Statistic>> {
        let v = vec![SimpleStatistic::with_value(
            STATISTIC_ID,
            0,
            STATISTIC_VALUE * 10,
            STATISTIC_VALUE,
        )];
        Box::new(v.into_iter())
    }

    fn alter_statistics(
        &self,
        character: &mut dyn Character<CustomRules>,
        alteration: &Self::StatisticsAlteration,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> Option<Transmutation> {
        character
            .statistic_mut(&STATISTIC_ID)
            .unwrap()
            .set_value(*alteration);
        None
    }

    fn generate_status(
        &self,
        character: &dyn Character<CustomRules>,
        status_id: &StatusId<CustomRules>,
        potency: &Option<Potency<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> Option<Status<CustomRules>> {
        if *self.unstackable_statuses.borrow() && character.status(status_id).is_some() {
            return None;
        }
        let potency = potency.unwrap_or_else(|| (0, 0));
        Some(SimpleStatus::new(*status_id, potency.0, Some(potency.1)))
    }

    fn alter_statuses(
        &self,
        character: &mut dyn Character<CustomRules>,
        alteration: &Self::StatusesAlteration,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        for status in character.statuses_mut() {
            let current = status.effect();
            status.set_effect(current + alteration);
        }
    }
}

#[derive(Default)]
pub struct CustomFightRules {}

impl FightRules<CustomRules> for CustomFightRules {
    type Impact = ();
    // Pair of (intensity, duration).
    type Potency = (i32, StatusDuration);

    fn apply_status(
        &self,
        _state: &BattleState<CustomRules>,
        character: &dyn Character<CustomRules>,
        application: Application<CustomRules>,
        event_queue: &mut Option<EventQueue<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        let delta = match application {
            Application::New(status) => status.effect(),
            Application::Replacement(_, new) => new.effect(),
        };
        AlterStatistics::trigger(event_queue, *character.entity_id(), delta * STATISTIC_VALUE)
            .fire();
    }

    fn update_status(
        &self,
        _state: &BattleState<CustomRules>,
        character: &dyn Character<CustomRules>,
        status: &AppliedStatus<CustomRules>,
        linked_queue: &mut Option<LinkedQueue<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> bool {
        let current_value = character.statistic(&STATISTIC_ID).unwrap().value();
        AlterStatistics::trigger(linked_queue, *character.entity_id(), current_value + 1).fire();
        status.duration() == status.max_duration().unwrap()
    }

    fn delete_status(
        &self,
        _state: &BattleState<CustomRules>,
        character: &dyn Character<CustomRules>,
        _status: &AppliedStatus<CustomRules>,
        event_queue: &mut Option<EventQueue<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        AlterStatistics::trigger(event_queue, *character.entity_id(), STATISTIC_VALUE).fire();
    }
}

battle_rules! {
    EmptyTeamRules,
    CustomCharacterRules,
    EmptyActorRules,
    CustomFightRules,
    EmptyUserRules,
    EmptySpaceRules,
    EmptyRoundsRules,
    EmptyEntropyRules
}

/// Creates a scenario with custom rules, one team, one creature and one object.
macro_rules! scenario {
    () => {{
        // Create the battle.
        let battle = Battle::builder(CustomRules::new()).build();
        let mut server = Server::builder(battle).build();
        // Create a team.
        util::team(&mut server, TEAM_1_ID);
        // Create a creature.
        util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
        // Create an object.
        util::object(&mut server, OBJECT_1_ID, ());
        server
    }};
}

/// Returns the creature with id CREATURE_1_ID.
macro_rules! creature {
    ($server: expr) => {{
        $server
            .battle()
            .entities()
            .creature(&CREATURE_1_ID)
            .unwrap()
    }};
}

/// Returns the object with the given id.
macro_rules! object {
    ($server: expr) => {{
        object!($server, &OBJECT_1_ID)
    }};
    ($server: expr, $id: expr) => {{
        $server.battle().entities().object($id).unwrap()
    }};
}

#[test]
fn status_inflict() {
    let mut server = scenario!();
    // Check that inflict with a wrong entity fails.
    assert_eq!(
        InflictStatus::trigger(&mut server, ENTITY_ERR_ID, STATUS_1_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::EntityNotFound(ENTITY_ERR_ID))
    );
    // Add a new status to the creature.
    assert_eq!(
        InflictStatus::trigger(&mut server, ENTITY_C1_ID, STATUS_1_ID)
            .potency((STATUS_INTENSITY, STATUS_DURATION))
            .fire()
            .err(),
        None
    );
    // Verify that status side effects have been applied.
    assert!(creature!(server).status(&STATUS_1_ID).is_some());
    // The creature should have a new value for the statistic.
    assert_eq!(
        creature!(server).statistic(&STATISTIC_ID).unwrap().value(),
        STATUS_INTENSITY * STATISTIC_VALUE
    );
    // Verify AppliedStatus's origin.
    let inflict_status_event_id = server
        .battle()
        .history()
        .events()
        .iter()
        .find(|e| e.kind() == EventKind::InflictStatus)
        .map(|e| e.id())
        .unwrap();
    assert_eq!(
        creature!(server).status(&STATUS_1_ID).unwrap().origin(),
        Some(inflict_status_event_id)
    );
    // Replace the status effect already present in the creature.
    assert_eq!(
        InflictStatus::trigger(&mut server, ENTITY_C1_ID, STATUS_1_ID)
            .potency((STATUS_INTENSITY * 2, STATUS_DURATION))
            .fire()
            .err(),
        None
    );
    // Verify that status side effects have been updated.
    assert!(creature!(server).status(&STATUS_1_ID).is_some());
    // The creature should have an updated value for the statistic.
    assert_eq!(
        creature!(server).statistic(&STATISTIC_ID).unwrap().value(),
        STATUS_INTENSITY * 2 * STATISTIC_VALUE
    );
}

#[test]
fn status_clear() {
    let mut server = scenario!();
    // Check that clear with a wrong entity fails.
    assert_eq!(
        ClearStatus::trigger(&mut server, ENTITY_ERR_ID, STATUS_1_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::EntityNotFound(ENTITY_ERR_ID))
    );
    // Inflict a status to a creature.
    assert_eq!(
        InflictStatus::trigger(&mut server, ENTITY_C1_ID, STATUS_1_ID)
            .fire()
            .err(),
        None
    );
    // Check that removing non existent statuses fails.
    assert_eq!(
        ClearStatus::trigger(&mut server, ENTITY_C1_ID, STATUS_ERR_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::StatusNotPresent(ENTITY_C1_ID, STATUS_ERR_ID))
    );
    // Remove the status from the creature.
    assert_eq!(
        ClearStatus::trigger(&mut server, ENTITY_C1_ID, STATUS_1_ID)
            .fire()
            .err(),
        None
    );
    // Verify that status side effects have been deleted.
    assert!(creature!(server).status(&STATUS_1_ID).is_none());
    assert_eq!(
        creature!(server).statistic(&STATISTIC_ID).unwrap().value(),
        STATISTIC_VALUE
    );
}

#[test]
fn status_update() {
    let mut server = scenario!();
    // Inflict the status to the creature.
    assert_eq!(
        InflictStatus::trigger(&mut server, ENTITY_C1_ID, STATUS_1_ID)
            .potency((STATUS_INTENSITY, STATUS_DURATION))
            .fire()
            .err(),
        None
    );
    // Check that the status side effects are there.
    assert_eq!(
        creature!(server).statistic(&STATISTIC_ID).unwrap().value(),
        STATUS_INTENSITY * STATISTIC_VALUE
    );
    // Do a round.
    util::start_round(&mut server, &ENTITY_C1_ID);
    util::end_round(&mut server);
    // Check that the statistic's value changed.
    assert_eq!(
        creature!(server).statistic(&STATISTIC_ID).unwrap().value(),
        STATUS_INTENSITY * STATISTIC_VALUE + 1
    );
    // Status' duration should have increased.
    assert_eq!(
        creature!(server).status(&STATUS_1_ID).unwrap().duration(),
        1
    );
    // Verify that the AlterStatistics event triggered by the status update has origin equal to the
    // event id of InflictStatus.
    let events = server.battle().history().events();
    let inflict_id = events
        .iter()
        .find(|e| e.kind() == EventKind::InflictStatus)
        .unwrap()
        .id();
    let alter_origin = events
        .iter()
        .rev()
        .find(|e| e.kind() == EventKind::AlterStatistics)
        .unwrap()
        .origin();
    assert_eq!(alter_origin, Some(inflict_id));
    // Do another round.
    util::start_round(&mut server, &ENTITY_C1_ID);
    util::end_round(&mut server);
    // The status should have been terminated now.
    assert!(creature!(server).status(&STATUS_1_ID).is_none());
    // Check that the side effects have been deleted.
    assert_eq!(
        creature!(server).statistic(&STATISTIC_ID).unwrap().value(),
        STATISTIC_VALUE
    );
}

#[test]
fn status_for_objects() {
    let mut server = scenario!();
    // Create another object.
    util::object(&mut server, OBJECT_2_ID, ());
    // Add a new status to both objects.
    assert_eq!(
        InflictStatus::trigger(&mut server, ENTITY_O1_ID, STATUS_1_ID)
            .potency((STATUS_INTENSITY, STATUS_DURATION))
            .fire()
            .err(),
        None
    );
    assert_eq!(
        InflictStatus::trigger(&mut server, ENTITY_O2_ID, STATUS_1_ID)
            .potency((STATUS_INTENSITY, STATUS_DURATION))
            .fire()
            .err(),
        None
    );
    // Verify that status side effects have been applied.
    assert!(object!(server).status(&STATUS_1_ID).is_some());
    assert_eq!(
        object!(server).statistic(&STATISTIC_ID).unwrap().value(),
        STATUS_INTENSITY * STATISTIC_VALUE
    );
    // Trigger a round for the environment.
    assert_eq!(EnvironmentRound::trigger(&mut server).fire().err(), None);
    // Check that both objects have been altered by the status update.
    assert_eq!(
        object!(server).statistic(&STATISTIC_ID).unwrap().value(),
        STATUS_INTENSITY * STATISTIC_VALUE + 1
    );
    assert_eq!(
        object!(server, &OBJECT_2_ID)
            .statistic(&STATISTIC_ID)
            .unwrap()
            .value(),
        STATUS_INTENSITY * STATISTIC_VALUE + 1
    );
    // Remove the status from the first object.
    assert_eq!(
        ClearStatus::trigger(&mut server, ENTITY_O1_ID, STATUS_1_ID)
            .fire()
            .err(),
        None
    );
    // Verify that status side effects have been deleted.
    assert!(object!(server).status(&STATUS_1_ID).is_none());
    assert_eq!(
        object!(server).statistic(&STATISTIC_ID).unwrap().value(),
        STATISTIC_VALUE
    );
    // Do another round.
    assert_eq!(EnvironmentRound::trigger(&mut server).fire().err(), None);
    // Check that the status expired from the second object.
    assert!(object!(server, &OBJECT_2_ID).status(&STATUS_1_ID).is_none());
    assert_eq!(
        object!(server, &OBJECT_2_ID)
            .statistic(&STATISTIC_ID)
            .unwrap()
            .value(),
        STATISTIC_VALUE
    );
}

#[test]
fn multiple_statuses() {
    let mut server = scenario!();
    // Inflict one status on the creature.
    assert_eq!(
        InflictStatus::trigger(&mut server, ENTITY_C1_ID, STATUS_1_ID)
            .fire()
            .err(),
        None
    );
    // Inflict another status on the same creature.
    assert_eq!(
        InflictStatus::trigger(&mut server, ENTITY_C1_ID, STATUS_2_ID)
            .fire()
            .err(),
        None
    );
    // Verify both statuses are saved.
    assert_eq!(creature!(server).statuses().count(), 2);
}

#[test]
fn status_not_stackable() {
    let mut server = scenario!();
    // Change rules to make statuses not stackable.
    *server
        .battle()
        .rules()
        .character_rules()
        .unstackable_statuses
        .borrow_mut() = true;
    // Inflict the status once.
    assert_eq!(
        InflictStatus::trigger(&mut server, ENTITY_C1_ID, STATUS_1_ID)
            .potency((STATUS_INTENSITY, STATUS_DURATION))
            .fire()
            .err(),
        None
    );
    // Verify that status side effects have been applied.
    assert_eq!(
        creature!(server).statistic(&STATISTIC_ID).unwrap().value(),
        STATUS_INTENSITY * STATISTIC_VALUE
    );
    // Inflict the status another time, with different potency.
    assert_eq!(
        InflictStatus::trigger(&mut server, ENTITY_C1_ID, STATUS_1_ID)
            .potency((STATUS_INTENSITY * 2, STATUS_DURATION))
            .fire()
            .err(),
        None
    );
    // Check that the second application was ignored.
    assert_eq!(
        creature!(server).statistic(&STATISTIC_ID).unwrap().value(),
        STATUS_INTENSITY * STATISTIC_VALUE
    );
}

#[test]
fn statuses_alteration() {
    const ALTERATION: i32 = 10;
    let mut server = scenario!();
    // Inflict a status to both the creature and the object.
    assert_eq!(
        InflictStatus::trigger(&mut server, ENTITY_C1_ID, STATUS_1_ID)
            .potency((STATUS_INTENSITY, STATUS_DURATION))
            .fire()
            .err(),
        None
    );
    assert_eq!(
        InflictStatus::trigger(&mut server, ENTITY_O1_ID, STATUS_1_ID)
            .potency((STATUS_INTENSITY, STATUS_DURATION))
            .fire()
            .err(),
        None
    );
    // Verify alter statuses preconditions.
    assert_eq!(
        AlterStatuses::trigger(&mut server, ENTITY_ERR_ID, ALTERATION)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::EntityNotFound(ENTITY_ERR_ID))
    );
    // Alter the inflicted statuses.
    assert_eq!(
        AlterStatuses::trigger(&mut server, ENTITY_C1_ID, ALTERATION)
            .fire()
            .err(),
        None
    );
    assert_eq!(
        AlterStatuses::trigger(&mut server, ENTITY_O1_ID, ALTERATION)
            .fire()
            .err(),
        None
    );
    // Check alterations.
    assert_eq!(
        creature!(server).status(&STATUS_1_ID).unwrap().effect(),
        STATUS_INTENSITY + ALTERATION
    );
    assert_eq!(
        object!(server).status(&STATUS_1_ID).unwrap().effect(),
        STATUS_INTENSITY + ALTERATION
    );
}
