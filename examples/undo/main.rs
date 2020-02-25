use crate::rules::*;
use std::io::Read;
use weasel::ability::ActivateAbility;
use weasel::battle::Battle;
use weasel::creature::{CreateCreature, CreatureId};
use weasel::entity::EntityId;
use weasel::event::EventTrigger;
use weasel::round::{EndRound, StartRound};
use weasel::team::{CreateTeam, TeamId};
use weasel::Server;

mod rules;

const TEAM_ID: TeamId<CustomRules> = 0;
const CREATURE_ID: CreatureId<CustomRules> = 0;
const ENTITY_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_ID);

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
    print_controls();
}

fn print_controls() {
    println!("  Controls:");
    println!("    w - Move up");
    println!("    s - Move down");
    println!("    d - Move right");
    println!("    a - Move left");
    println!("    u - Undo");
    println!("    r - Redo");
    println!("    h - Display the controls");
    println!("    q - Quit");
}

fn game_loop() {
    // Create a server.
    let mut server = create_server();
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
                    walk(&mut server, Direction::Up);
                    display_world(&server);
                }
                's' => {
                    walk(&mut server, Direction::Down);
                    display_world(&server);
                }
                'd' => {
                    walk(&mut server, Direction::Right);
                    display_world(&server);
                }
                'a' => {
                    walk(&mut server, Direction::Left);
                    display_world(&server);
                }
                'u' => {
                    server = undo(server);
                    display_world(&server);
                }
                'r' => {
                    server = redo(server);
                    display_world(&server);
                }
                'h' => print_controls(),
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

/// Moves the creature on step towards the given direction.
fn walk(server: &mut Server<CustomRules>, direction: Direction) {
    // Start a round.
    StartRound::trigger(server, ENTITY_ID).fire().unwrap();
    // Activate the 'walk' ability of the creature.
    let result = ActivateAbility::trigger(server, ENTITY_ID, WALK)
        .activation(direction)
        .fire();
    // We print an error in case the movement is not allowed.
    if result.is_err() {
        println!("Movement not allowed!");
    }
    // End the round.
    EndRound::trigger(server).fire().unwrap();
}

/// Undo the last action.
fn undo(server: Server<CustomRules>) -> Server<CustomRules> {
    server
}

/// Redo the last undoed action.
fn redo(server: Server<CustomRules>) -> Server<CustomRules> {
    server
}
