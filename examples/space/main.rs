use crate::rules::*;
use rules::BattlefieldSeed;
use weasel::battle::Battle;
use weasel::creature::{CreateCreature, CreatureId};
use weasel::event::EventTrigger;
use weasel::space::{AlterSpace, ResetSpace};
use weasel::team::{CreateTeam, TeamId};
use weasel::Server;

mod rules;

static TEAM_ID: TeamId<CustomRules> = 1;
static CREATURE_1: CreatureId<CustomRules> = 1;
static CREATURE_2: CreatureId<CustomRules> = 2;
static CREATURE_3: CreatureId<CustomRules> = 3;

fn main() {
    // Create a server to manage the battle.
    let battle = Battle::builder(CustomRules::new()).build();
    let mut server = Server::builder(battle).build();
    // Set the space model to be a 2D battlefield of squares.
    ResetSpace::trigger(&mut server)
        .seed(BattlefieldSeed::TwoDimensions)
        .fire()
        .unwrap();
    // Display the space model.
    println!("Battlefield:\n{}", server.battle().space().model());
    // Spawn three creatures.
    println!("Spawning three creatures...");
    CreateTeam::trigger(&mut server, TEAM_ID).fire().unwrap();
    // First creature goes in [1;0].
    CreateCreature::trigger(&mut server, CREATURE_1, TEAM_ID, Square { x: 1, y: 0 })
        .fire()
        .unwrap();
    // Second creature goes in [3;3].
    CreateCreature::trigger(&mut server, CREATURE_2, TEAM_ID, Square { x: 3, y: 3 })
        .fire()
        .unwrap();
    // Third creature goes in [4;3].
    CreateCreature::trigger(&mut server, CREATURE_3, TEAM_ID, Square { x: 4, y: 3 })
        .fire()
        .unwrap();
    println!();
    // Display the space model and the creatures.
    println!("Battlefield:\n{}", server.battle().space().model());
    // Put traps on the squares across the diagonals.
    println!("Placing traps on the diagonals!");
    AlterSpace::trigger(
        &mut server,
        vec![
            Square { x: 0, y: 0 },
            Square { x: 1, y: 1 },
            Square { x: 2, y: 2 },
            Square { x: 3, y: 3 },
            Square { x: 4, y: 4 },
            Square { x: 0, y: 4 },
            Square { x: 1, y: 3 },
            Square { x: 3, y: 1 },
            Square { x: 4, y: 0 },
        ],
    )
    .fire()
    .unwrap();
    assert_eq!(server.battle().entities().entities().count(), 2);
    println!();
    // Display the space model and the creatures. Some of them died!
    println!("Battlefield:\n{}", server.battle().space().model());
    // Now completely reset the space model, dropping one dimension.
    println!("Removing the y-axis!");
    ResetSpace::trigger(&mut server)
        .seed(BattlefieldSeed::OneDimension)
        .fire()
        .unwrap();
    println!();
    // Display the space model and the creatures. Their positions have been adapted!
    println!("Battlefield:\n{}", server.battle().space().model());
}
