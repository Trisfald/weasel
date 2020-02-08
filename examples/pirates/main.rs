use crate::game::Game;
use std::io::Read;

mod game;
mod rules;

fn main() {
    print_intro();
    // The loop where the game progresses.
    game_loop();
    // When this point is reached, the game has ended.
    println!("");
    println!("Goodbye!");
}

fn print_intro() {
    println!("Welcome to Pirates!");
    println!("");
    println!("Sink the enemy's ship by shooting your cannons!");
    println!("");
    print_controls();
}

fn print_controls() {
    println!("  Controls:");
    println!("    1 - Fire cannonballs");
    println!("    2 - Fire grapeshots");
    println!("    h - Display the controls");
    println!("    s - Save the game state");
    println!("    l - Load the savegame");
    println!("    q - Quit");
}

fn print_separator() {
    println!("--------------------------------------------------------------------------------");
}

fn game_loop() {
    // Create a game instance.
    let mut game = Game::new();
    turn_header(&game);
    // In this loop we process user input and dispatch to the corresponding method.
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
                // When the player takes a move we first fire an ability, then let the enemy act.
                // Remember to always check if there's a winner.
                '1' => {
                    print_separator();
                    game.fire_cannonball();
                    if game.check_winner() {
                        break;
                    }
                    game.enemy_round();
                    if game.check_winner() {
                        break;
                    }
                    turn_header(&game);
                }
                '2' => {
                    print_separator();
                    game.fire_grapeshot();
                    if game.check_winner() {
                        break;
                    }
                    game.enemy_round();
                    if game.check_winner() {
                        break;
                    }
                    turn_header(&game);
                }
                'h' => print_controls(),
                's' => game.save(),
                'l' => {
                    game.load();
                    if game.check_winner() {
                        break;
                    }
                    turn_header(&game);
                }
                'q' => break,
                _ => {}
            }
        }
    }
}

/// Displays the player's and enemy's statistics.
fn turn_header(game: &Game) {
    let (player_ship_hull, player_ship_crew) = game.player_stats();
    let (enemy_ship_hull, enemy_ship_crew) = game.enemy_stats();
    let stat_to_string = |stat| {
        let i = (stat as usize + 5 - 1) / 5; // ceiling
        std::iter::repeat("=")
            .take(i)
            .chain(std::iter::repeat(" ").take(20 - i))
            .collect::<String>()
    };
    print_separator();
    println!("---            PLAYER SHIP                                ENEMY SHIP         ---");
    println!(
        "---   HULL [{}]                HULL [{}] ---",
        stat_to_string(player_ship_hull),
        stat_to_string(enemy_ship_hull)
    );
    println!(
        "---   CREW [{}]                CREW [{}] ---",
        stat_to_string(player_ship_crew),
        stat_to_string(enemy_ship_crew)
    );
    print_separator();
}
