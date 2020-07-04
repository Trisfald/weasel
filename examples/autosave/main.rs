use crate::sink::AutosaveSink;
use std::convert::TryInto;
use std::fs::File;
use std::{env, io::BufRead, io::BufReader, io::Read};
use weasel::battle::{Battle, BattleController, BattleRules};
use weasel::creature::CreateCreature;
use weasel::event::{EventReceiver, EventSinkId, EventTrigger};
use weasel::serde::FlatVersionedEvent;
use weasel::team::{CreateTeam, TeamId};
use weasel::{battle_rules, rules::empty::*, Server};

mod sink;

// It's not a real game so we can use generic no-op battle rules.
battle_rules! {}

static TEAM_ID: TeamId<CustomRules> = 0;
const AUTOSAVE_NAME: &str = "autosave";
const SINK_ID: EventSinkId = 0;

fn main() {
    print_intro();
    // The loop where the game progresses.
    game_loop();
    // When this point is reached, the game has ended.
    println!();
    println!("Goodbye!");
}

fn print_intro() {
    println!("Autosave");
    println!();
    println!("Example to demonstrate how to use an event sink to create autosaves with weasel.");
    println!("Create soldiers and exit whenever you want.");
    println!("Next time you launch the game it will resume from the latest progress!");
    println!();
    println!("  Controls:");
    println!("    c - Create a new soldier");
    println!("    q - Quit");
}

fn game_loop() {
    // Create a server.
    let mut server = create_server();
    println!();
    print_soldiers_count(&server);
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
                'c' => {
                    create_soldier(&mut server);
                    print_soldiers_count(&server);
                }
                'q' => break,
                _ => {}
            }
        }
    }
}

/// Retrieves how many creatures are in the battle.
fn get_soldiers_count(server: &Server<CustomRules>) -> usize {
    server.battle().entities().creatures().count()
}

fn print_soldiers_count(server: &Server<CustomRules>) {
    println!("Current number of soldiers: {}", get_soldiers_count(server));
}

/// Creates a new 'soldier' creature.
fn create_soldier(server: &mut Server<CustomRules>) {
    let next_id = get_soldiers_count(server).try_into().unwrap();
    CreateCreature::trigger(server, next_id, TEAM_ID, ())
        .fire()
        .unwrap();
}

/// Creates a new server. The battle state will be loaded from the autosave, if found.
fn create_server() -> Server<CustomRules> {
    // Create a new server to manage the battle.
    let battle = Battle::builder(CustomRules::new()).build();
    let mut server = Server::builder(battle).build();
    // Read the json stored in a temporary file.
    let mut path = env::temp_dir();
    path.push(AUTOSAVE_NAME);
    let file = File::open(path);
    match file {
        Ok(file) => {
            let mut reader = BufReader::new(file);
            // Deserialize all events, one at a time, because we append them in sequence.
            loop {
                let mut buffer = Vec::new();
                // We use a delimiter to separate the different json objects.
                let result = reader.read_until(b'#', &mut buffer).unwrap();
                if result > 0 {
                    // Remove the delimiter.
                    buffer.truncate(buffer.len() - 1);
                    // Replay the event in the server.
                    let event: FlatVersionedEvent<_> = serde_json::from_slice(&buffer).unwrap();
                    server.receive(event.into()).unwrap()
                } else {
                    // End of file.
                    break;
                }
            }
            attach_sink(&mut server);
            // Return the server with the restored autosave.
            server
        }
        Err(_) => {
            // No autosave, so setup a fresh battle.
            attach_sink(&mut server);
            // Create a team where we will put all soldiers.
            CreateTeam::trigger(&mut server, TEAM_ID).fire().unwrap();
            server
        }
    }
}

/// Attaches a sink to the server to dump events into a file.
fn attach_sink(server: &mut Server<CustomRules>) {
    let sink = AutosaveSink::new(SINK_ID, AUTOSAVE_NAME);
    server.client_sinks_mut().add_sink(Box::new(sink)).unwrap();
}
