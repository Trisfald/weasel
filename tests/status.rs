use weasel::battle::{Battle, BattleRules, BattleState};
use weasel::character::{Character, CharacterRules};
use weasel::entity::EntityId;
use weasel::entropy::Entropy;
use weasel::event::{EventQueue, EventTrigger};
use weasel::fight::FightRules;
use weasel::metric::WriteMetrics;
use weasel::rules::status::SimpleStatus;
use weasel::status::{ClearStatus, InflictStatus, Potency, Status, StatusId};
use weasel::{battle_rules, rules::empty::*, Server, WeaselError};

const TEAM_1_ID: u32 = 1;
const CREATURE_1_ID: u32 = 1;
const CREATURE_ERR_ID: u32 = 99;
const OBJECT_1_ID: u32 = 1;
const ENTITY_C1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
const ENTITY_O1_ID: EntityId<CustomRules> = EntityId::Object(OBJECT_1_ID);
const ENTITY_ERR_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_ERR_ID);
const STATUS_1_ID: u32 = 1;
const STATUS_2_ID: u32 = 2;
const STATUS_ERR_ID: u32 = 99;
const STATUS_INTENSITY: u32 = 5;
const STATUS_DURATION: u16 = 1;

#[derive(Default)]
pub struct CustomCharacterRules {}

impl CharacterRules<CustomRules> for CustomCharacterRules {
    type CreatureId = u32;
    type ObjectId = u32;
    type Statistic = EmptyStat;
    type StatisticsSeed = ();
    type StatisticsAlteration = ();

    fn generate_status(
        &self,
        _character: &dyn Character<CustomRules>,
        status_id: &StatusId<CustomRules>,
        potency: &Option<Potency<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> Option<Status<CustomRules>> {
        let potency = potency.unwrap_or_else(|| (0, 0));
        Some(SimpleStatus::new(*status_id, potency.0, Some(potency.1)))
    }
}

#[derive(Default)]
pub struct CustomFightRules {}

impl FightRules<CustomRules> for CustomFightRules {
    type Impact = ();
    type Status = SimpleStatus<u32, u32>;
    /// Pair of (intensity, duration);
    type Potency = (u32, u16);

    fn apply_status(
        &self,
        _state: &BattleState<CustomRules>,
        _character: &dyn Character<CustomRules>,
        _status: &Status<CustomRules>,
        _event_queue: &mut Option<EventQueue<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
    }

    fn update_status(
        &self,
        _state: &BattleState<CustomRules>,
        _character: &dyn Character<CustomRules>,
        _status: &Status<CustomRules>,
        _event_queue: &mut Option<EventQueue<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> bool {
        false
    }

    fn delete_status(
        &self,
        _state: &BattleState<CustomRules>,
        _character: &dyn Character<CustomRules>,
        _status: &Status<CustomRules>,
        _event_queue: &mut Option<EventQueue<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
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
    assert!(server
        .battle()
        .entities()
        .creature(&CREATURE_1_ID)
        .unwrap()
        .status(&STATUS_1_ID)
        .is_some());

    // Replace the status effect already present in the creature.

    // Verify that status side effects have been applied???. How to handle this?
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

    // Verify that status side effects have been deleted.

    // Verify that status side effects have been deleted.
}

#[test]
fn status_update() {
    // let mut server = scenario!();
}

#[test]
fn status_for_objects() {
    let mut server = scenario!();
    // Add a new status to the object.
    assert_eq!(
        InflictStatus::trigger(&mut server, ENTITY_O1_ID, STATUS_1_ID)
            .potency((STATUS_INTENSITY, STATUS_DURATION))
            .fire()
            .err(),
        None
    );
    // Verify that status side effects have been applied.

    // Remove the status from the object.

    // Verify that status side effects have been deleted.
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
    assert_eq!(
        server
            .battle()
            .entities()
            .creature(&CREATURE_1_ID)
            .unwrap()
            .statuses()
            .count(),
        2
    );
}
