use std::convert::TryInto;
use weasel::battle::BattleRules;
use weasel::entropy::ResetEntropy;
use weasel::event::{EventId, EventKind, EventTrigger};
use weasel::round::EndRound;
use weasel::{battle_rules, rules::empty::*};

const TEAM_1_ID: u32 = 1;
const CREATURE_1_ID: u32 = 1;

battle_rules! {}

#[test]
fn timeline_populated() {
    // Create a server with a creature.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    // Create some more faulty events.
    assert!(EndRound::trigger(&mut server).fire().is_err());
    // Create some more good events.
    assert_eq!(ResetEntropy::trigger(&mut server).fire().err(), None);
    // Verify if the events are in the timeline.
    let events = server.battle().history().events();
    let len: EventId = events.len().try_into().unwrap();
    assert_eq!(len, 3);
    assert_eq!(server.battle().history().len(), 3);
    assert_eq!(events[0].kind(), EventKind::CreateTeam);
    assert_eq!(events[1].kind(), EventKind::CreateCreature);
    assert_eq!(events[2].kind(), EventKind::ResetEntropy);
    assert_eq!(events[2].id(), len - 1);
}
