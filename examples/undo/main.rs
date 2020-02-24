use crate::rules::*;
use std::io::Read;
use weasel::battle::Battle;
use weasel::creature::{CreateCreature, CreatureId};
use weasel::event::EventTrigger;
use weasel::team::{CreateTeam, TeamId};
use weasel::Server;

mod rules;

static TEAM_ID: TeamId<CustomRules> = 0;
static CREATURE_ID: CreatureId<CustomRules> = 0;

fn main() {
    print_intro();
    // The loop where the game progresses.
    game_loop();
    // When this point is reached, the game has ended.
    println!();
    println!("Goodbye!");
}

fn print_intro() {
    println!("Undo");
    println!();
    println!("Example to demonstrate how to undo/redo player actions with weasel.");
    println!("Move around the soldier on the battlefield.");
    println!();
    println!("  Controls:");
    println!("    w - Move up");
    println!("    s - Move down");
    println!("    d - Move right");
    println!("    a - Move left");
    println!("    u - Undo");
    println!("    r - Redo");
    println!("    q - Quit");
}

fn game_loop() {
    // Create a server.
    let server = create_server();
    println!();
    display_world(&server);
    // Main loop.
    loop {
        // Read a char from stdin.
        let input: Option<char> = std::io::stdin()
            .bytes()
            .next()
            .and_then(|result| result.ok())
            .map(|byte| byte as char);
        // Take an action depending on the user input.
        if let Some(key) = input {
            match key {
                'w' => {
                    display_world(&server);
                }
                's' => {
                    display_world(&server);
                }
                'd' => {
                    display_world(&server);
                }
                'a' => {
                    display_world(&server);
                }
                'u' => {
                    display_world(&server);
                }
                'r' => {
                    display_world(&server);
                }
                'q' => break,
                _ => {}
            }
        }
    }
}

fn display_world(server: &Server<CustomRules>) {
    // Display the space model.
    println!("Battlefield:\n{}", server.battle().space().model());
}

/// Creates a new server.
fn create_server() -> Server<CustomRules> {
    // Create a new server to manage the battle.
    let battle = Battle::builder(CustomRules::new()).build();
    let mut server = Server::builder(battle).build();
    // Create a team and a creature.
    CreateTeam::trigger(&mut server, TEAM_ID).fire().unwrap();
    CreateCreature::trigger(&mut server, CREATURE_ID, TEAM_ID, Square { x: 0, y: 0 })
        .fire()
        .unwrap();
    server
}
