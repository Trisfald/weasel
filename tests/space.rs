use std::collections::HashSet;
use weasel::battle::BattleRules;
use weasel::battle_rules_with_space;
use weasel::creature::CreateCreature;
use weasel::entity::{Entity, EntityId};
use weasel::event::{EventQueue, EventTrigger};
use weasel::metric::WriteMetrics;
use weasel::server::Server;
use weasel::space::{MoveEntity, ResetSpace, SpaceRules};
use weasel::WeaselError;
use weasel::{battle_rules, rules::empty::*};

static TEAM_1_ID: u32 = 1;
static CREATURE_1_ID: u32 = 1;
static ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
static CREATURE_2_ID: u32 = 2;
static POSITION_1: u32 = 1;
static POSITION_2: u32 = 2;
static POSITION_T: u32 = 99;

#[derive(Default)]
struct CustomSpaceRules {}

impl SpaceRules<CustomRules> for CustomSpaceRules {
    type Position = u32;
    type SpaceSeed = ();
    type SpaceModel = HashSet<Self::Position>;

    fn generate_model(&self, _: &Option<Self::SpaceSeed>) -> Self::SpaceModel {
        HashSet::new()
    }

    fn check_move(
        &self,
        model: &Self::SpaceModel,
        _entity: Option<&dyn Entity<CustomRules>>,
        position: &Self::Position,
    ) -> bool {
        !model.contains(position)
    }

    fn move_entity(
        &self,
        model: &mut Self::SpaceModel,
        entity: Option<&dyn Entity<CustomRules>>,
        position: &Self::Position,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        if let Some(entity) = entity {
            model.remove(entity.position());
        }
        model.insert(*position);
    }

    fn translate_entity(
        &self,
        _model: &Self::SpaceModel,
        new_model: &mut Self::SpaceModel,
        entity: &mut dyn Entity<CustomRules>,
        _event_queue: &mut Option<EventQueue<CustomRules>>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        // All entities go to POSITION_T when changing from one space to another.
        new_model.insert(POSITION_T);
        entity.set_position(POSITION_T);
    }
}

battle_rules_with_space! { CustomSpaceRules }

fn init_custom_game() -> Server<CustomRules> {
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    // Create a first creature in position 0.
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, POSITION_1);
    assert!(server
        .battle()
        .entities()
        .creature(&CREATURE_1_ID)
        .is_some());
    server
}

#[test]
fn position_verified() {
    let mut server = init_custom_game();
    // Try to create a second creature again in position 0.
    assert_eq!(
        CreateCreature::trigger(&mut server, CREATURE_2_ID, TEAM_1_ID, POSITION_1)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::PositionError(None, POSITION_1))
    );
    assert!(server
        .battle()
        .entities()
        .creature(&CREATURE_2_ID)
        .is_none());
}

#[test]
fn move_entity() {
    let mut server = init_custom_game();
    // Move the creature into invalid position.
    assert_eq!(
        MoveEntity::trigger(&mut server, ENTITY_1_ID.clone(), POSITION_1)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::PositionError(Some(POSITION_1), POSITION_1))
    );
    assert_eq!(
        *server
            .battle()
            .entities()
            .entity(&ENTITY_1_ID)
            .unwrap()
            .position(),
        POSITION_1
    );
    // Move the creature into a valid position.
    assert_eq!(
        MoveEntity::trigger(&mut server, ENTITY_1_ID.clone(), POSITION_2)
            .fire()
            .err(),
        None
    );
    assert_eq!(
        *server
            .battle()
            .entities()
            .entity(&ENTITY_1_ID)
            .unwrap()
            .position(),
        POSITION_2
    );
    assert_eq!(server.battle().space().model().len(), 1);
}

#[test]
fn reset_space() {
    // Create a scenario.
    let mut server = init_custom_game();
    // Change the space model.
    assert_eq!(ResetSpace::trigger(&mut server).fire().err(), None);
    // Check that entity's position changed.
    assert_eq!(
        *server
            .battle()
            .entities()
            .entity(&ENTITY_1_ID)
            .unwrap()
            .position(),
        POSITION_T
    );
    assert_eq!(server.battle().space().model().len(), 1);
}
