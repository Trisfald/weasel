use weasel::battle::BattleRules;
use weasel::entity::{Entity, EntityId};
use weasel::{battle_rules, rules::empty::*, WeaselError};

static TEAM_1_ID: u32 = 1;
static CREATURE_1_ID: u32 = 1;
static CREATURE_2_ID: u32 = 2;
static OBJECT_1_ID: u32 = 1;
static OBJECT_2_ID: u32 = 2;
static ENTITY_C1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
static ENTITY_C2_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_2_ID);
static ENTITY_O1_ID: EntityId<CustomRules> = EntityId::Object(OBJECT_1_ID);
static ENTITY_O2_ID: EntityId<CustomRules> = EntityId::Object(OBJECT_2_ID);

battle_rules! {}

#[test]
fn entity_id_methods() {
    // Create the battle.
    let mut server = util::server(CustomRules::new());
    // Create a team.
    util::team(&mut server, TEAM_1_ID);
    // Create two creatures.
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    util::creature(&mut server, CREATURE_2_ID, TEAM_1_ID, ());
    // Create two objects.
    util::object(&mut server, OBJECT_1_ID, ());
    util::object(&mut server, OBJECT_2_ID, ());
    // Test is_* methods.
    let creature_id = server
        .battle()
        .entities()
        .creature(&CREATURE_1_ID)
        .unwrap()
        .entity_id();
    let object_id = server
        .battle()
        .entities()
        .object(&OBJECT_1_ID)
        .unwrap()
        .entity_id();
    assert!(creature_id.is_character());
    assert!(object_id.is_character());
    assert!(creature_id.is_actor());
    assert!(!object_id.is_actor());
    // Test extracting the concrete id.
    assert_eq!(creature_id.creature(), Ok(CREATURE_1_ID));
    assert_eq!(
        creature_id.object().err(),
        Some(WeaselError::NotAnObject(ENTITY_C1_ID))
    );
    assert_eq!(
        object_id.creature().err(),
        Some(WeaselError::NotACreature(ENTITY_O1_ID))
    );
    assert_eq!(object_id.object(), Ok(OBJECT_1_ID));
}

#[test]
fn entity_id_equality() {
    assert_eq!(ENTITY_C1_ID, ENTITY_C1_ID);
    assert_ne!(ENTITY_C1_ID, ENTITY_C2_ID);
    assert_eq!(ENTITY_O1_ID, ENTITY_O1_ID);
    assert_ne!(ENTITY_O1_ID, ENTITY_O2_ID);
    assert_ne!(ENTITY_O1_ID, ENTITY_C1_ID);
}
