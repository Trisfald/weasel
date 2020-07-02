use crate::rules::CustomRules;
use crate::tcp::{TcpClient, TcpServer};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::convert::TryInto;
use std::io::Read;
use weasel::battle::Battle;
use weasel::creature::CreateCreature;
use weasel::event::EventTrigger;
use weasel::team::CreateTeam;
use weasel::Server;

mod rules;
mod tcp;

fn main() {
    print_intro();
    // Get the server's address from command line args. If empty, it means we are also the server.
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 {
        let client = TcpClient::new(&args[1]);
        print_intro();
        game_loop();
    } else {
        // Create a battle object with our game rules.
        let battle = Battle::builder(CustomRules::new()).build();
        // Create a server to handle the game state.
        let mut game_server = Server::builder(battle).build();
        // Initialize the game.
        initialize_game(&mut game_server);
        // Run the tcp server.
        let server = TcpServer::new(game_server);
        print_intro();
        game_loop();
    }
    // When this point is reached, the game has ended.
    println!("\nGoodbye!");
}

// Here we initialize the deck and players.
fn initialize_game(server: &mut Server<CustomRules>) {
    // First of all create three teams. Each player will take control of a team.
    for n in 0..=2 {
        CreateTeam::trigger(server, n).fire().unwrap();
    }
    // Prepare a deck of nine cards, shuffled randomly.
    let mut cards: [u8; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 9];
    let mut rng = thread_rng();
    cards.shuffle(&mut rng);
    // Then give three cards to each team. We'll use weasel 'Creature' to represent cards.
    for (i, card) in cards.iter().enumerate() {
        CreateCreature::trigger(
            server,
            i.try_into().unwrap(), // for the card's id we use the incrementing counter
            (i % 3).try_into().unwrap(), // assign team in round-robin fashion
            (),
        )
        .statistics_seed(*card)
        .fire()
        .unwrap();
    }
}

/// The real game takes place in this loop.
fn game_loop() {
    loop {
        // Check the game status.
        game_status();
        // Read a char from stdin.
        let input: Option<char> = std::io::stdin()
            .bytes()
            .next()
            .and_then(|result| result.ok())
            .map(|byte| byte as char);
        // Take an action depending on the user input.
        if let Some(key) = input {
            match key {
                '1' => play_card(1),
                '2' => play_card(2),
                '3' => play_card(3),
                'h' => print_controls(),
                'q' => break,
                _ => {}
            }
        }
    }
}

/// Checks and prints the game status.
fn game_status() {
    // Check for game's end.
    // TODO
    // Print the game state.
    // TODO
}

/// Method to make the player play the card in the given slot
fn play_card(card_number: u8) {
    // TODO
    // Wait for the turn completion.
    // TODO
    // Print the game state.
    // TODO
}

// The following are few methods to display output to the user.

fn print_intro() {
    println!("\nWelcome to King of the hill!\n");
    println!("You are given three cards numbered from 1 to 9.");
    println!("Each round all players play one card, the highest wins!\n");
    print_controls();
    print_separator();
}

fn print_controls() {
    println!("  Controls:");
    println!("    1,2,3 - Play the corresponding card");
    println!("    h - Display the controls");
    println!("    q - Quit");
}

fn print_separator() {
    println!("--------------------------------------------------------------------------------");
}
