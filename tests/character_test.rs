use weasel::battle::BattleRules;
use weasel::character::{AlterStatistics, Character};
use weasel::entity::EntityId;
use weasel::event::EventTrigger;
use weasel::status::InflictStatus;
use weasel::{battle_rules, rules::empty::*};

const TEAM_1_ID: u32 = 1;
const CREATURE_1_ID: u32 = 1;
const STATUS_1_ID: u32 = 1;

#[test]
fn default_works() {
    battle_rules! {}
    // Create a server with a creature.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    // Empty AlterStatistics with default rules does not return an error.
    assert_eq!(
        AlterStatistics::trigger(&mut server, EntityId::Creature(CREATURE_1_ID), ())
            .fire()
            .err(),
        None
    );
    // Empty rules don't add any status.
    assert_eq!(
        InflictStatus::trigger(&mut server, EntityId::Creature(CREATURE_1_ID), STATUS_1_ID)
            .fire()
            .err(),
        None
    );
    assert_eq!(
        server
            .battle()
            .entities()
            .creature(&CREATURE_1_ID)
            .unwrap()
            .statuses()
            .count(),
        0
    );
}
