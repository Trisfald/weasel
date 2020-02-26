use crate::rules::*;
use std::io::Read;
use weasel::ability::ActivateAbility;
use weasel::battle::Battle;
use weasel::creature::{CreateCreature, CreatureId};
use weasel::entity::EntityId;
use weasel::event::{EventKind, EventReceiver, EventTrigger, VersionedEventWrapper};
use weasel::round::{EndRound, StartRound};
use weasel::team::{CreateTeam, TeamId};
use weasel::Server;

mod rules;

static TEAM_ID: TeamId<CustomRules> = 0;
static CREATURE_ID: CreatureId<CustomRules> = 0;
static ENTITY_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_ID);

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
    let mut server = create_game();
    // Create a buffer to keep events for redo.
    let mut event_buffer = Vec::new();
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
                    walk(&mut server, &mut event_buffer, Direction::Up);
                    display_world(&server);
                }
                's' => {
                    walk(&mut server, &mut event_buffer, Direction::Down);
                    display_world(&server);
                }
                'd' => {
                    walk(&mut server, &mut event_buffer, Direction::Right);
                    display_world(&server);
                }
                'a' => {
                    walk(&mut server, &mut event_buffer, Direction::Left);
                    display_world(&server);
                }
                'u' => {
                    server = undo(server, &mut event_buffer);
                    display_world(&server);
                }
                'r' => {
                    redo(&mut server, &mut event_buffer);
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
    // Display the number of steps taken and the space model.
    let steps = server
        .battle()
        .history()
        .events()
        .iter()
        .filter(|e| e.kind() == EventKind::ActivateAbility)
        .count();
    let battlefield = server.battle().space().model();
    println!("Steps: {}\nBattlefield:\n{}", steps, battlefield);
}

/// Creates a new server
fn create_server() -> Server<CustomRules> {
    let battle = Battle::builder(CustomRules::new()).build();
    Server::builder(battle).build()
}

/// Creates a new game: a server with a team and a creature.
fn create_game() -> Server<CustomRules> {
    let mut server = create_server();
    // Create a team and a creature.
    CreateTeam::trigger(&mut server, TEAM_ID).fire().unwrap();
    CreateCreature::trigger(&mut server, CREATURE_ID, TEAM_ID, Square { x: 0, y: 0 })
        .fire()
        .unwrap();
    server
}

/// Moves the creature on step towards the given direction.
fn walk(
    server: &mut Server<CustomRules>,
    event_buffer: &mut Vec<VersionedEventWrapper<CustomRules>>,
    direction: Direction,
) {
    // Clean the buffered events to invalidate the redo action.
    event_buffer.clear();
    // Start a round.
    StartRound::trigger(server, ENTITY_ID).fire().unwrap();
    // Activate the 'walk' ability of the creature.
    let result = ActivateAbility::trigger(server, ENTITY_ID, WALK)
        .activation(direction)
        .fire();
    // We print the error in case the movement is not allowed.
    if let Err(e) = result {
        println!("{:?}", e.unfold());
    }
    // End the round.
    EndRound::trigger(server).fire().unwrap();
}

/// Undo the last action.
fn undo(
    server: Server<CustomRules>,
    event_buffer: &mut Vec<VersionedEventWrapper<CustomRules>>,
) -> Server<CustomRules> {
    // Retrieve the last event of type ActivateAbility.
    let last = server
        .battle()
        .history()
        .events()
        .iter()
        .rev()
        .find(|e| e.kind() == EventKind::ActivateAbility);
    match last {
        Some(last) => {
            // We are gonna undo this round.
            // First save the current history in a buffer, if it's empty.
            if event_buffer.is_empty() {
                let mut events = server
                    .battle()
                    .versioned_events(std::ops::Range {
                        start: 0,
                        end: server.battle().history().len() as usize,
                    })
                    .collect();
                event_buffer.append(&mut events);
            }
            // Create a new server and replay history up to the last ActivateAbility before 'last'.
            // We know the number of events to replay is equal to last.id() - 1 because event id
            // is equal to the index of the event, and we subtract 1 to avoid replaying the
            // StartRound event.
            let mut server = create_server();
            for event in event_buffer.iter().take((last.id() - 1) as usize) {
                server.receive(event.clone()).unwrap();
            }
            server
        }
        None => {
            // No single action was taken yet. We can't undo anything, so return the same server.
            server
        }
    }
}

/// Redo the last undoed action.
fn redo(
    server: &mut Server<CustomRules>,
    event_buffer: &mut Vec<VersionedEventWrapper<CustomRules>>,
) {
    let history_len = server.battle().history().events().len();
    if event_buffer.len() > history_len {
        // There are some events to redo.
        // It's enough to replay the missing events. However, since we want to redo an entire
        // rounds, replay up to the next ActivateAbility + MoveEntity + EndRound.
        let future_events = &event_buffer[history_len..event_buffer.len()];
        let next_activation = future_events
            .iter()
            .position(|e| e.kind() == EventKind::ActivateAbility);
        if let Some(next_activation) = next_activation {
            // Last event to be replayed is next_activation + MoveEntity + EndRound.
            let last = next_activation + 2;
            for event in &future_events[0..=last] {
                server.receive(event.clone()).unwrap();
            }
        }
    }
    // Nothing to redo.
}
