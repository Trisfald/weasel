use crate::rules::{CustomRules, CARD_VALUE_STAT, PLAY_CARD_ABILITY};
use crate::tcp::{TcpClient, TcpServer};
use rand::{seq::SliceRandom, thread_rng};
use std::convert::TryInto;
use std::sync::{Arc, Mutex};
use std::{io::Read, thread, time};
use weasel::round::TurnsCount;
use weasel::team::TeamId;
use weasel::{
    ActivateAbility, Actor, Battle, BattleController, BattleState, Character, CreateCreature,
    CreateTeam, Creature, EndRound, EndTurn, EntityId, EventKind, EventProcessor, EventQueue,
    EventTrigger, EventWrapper, Id, RemoveEntity, ResetObjectives, Server, StartTurn,
};

mod rules;
mod tcp;

fn main() {
    // Get the server's address from command line args. If empty, it means we are also the server.
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 {
        // A tcp client contains a weasel Client. It forwards events registered in the latter
        // to the remote server and automatically dumps events coming from the server into the client.
        let client = TcpClient::new(&args[1]);
        // Attach the event callback.
        client
            .game_client
            .lock()
            .unwrap()
            .set_event_callback(Some(Box::new(event_callback)));
        print_intro();
        // Run the main game loop.
        client_game_loop(client);
    } else {
        // Create a battle object with our game rules.
        let battle = Battle::builder(CustomRules::new())
            .event_callback(Box::new(event_callback))
            .build();
        // Create a server to handle the game state.
        let mut game_server = Server::builder(battle).build();
        // Initialize the game.
        initialize_game(&mut game_server);
        // Tcp server contains a weasel Server. It sends local events to all clients and also
        // receives and validates their events.
        let server = TcpServer::new(game_server);
        print_intro();
        // Run the main game loop.
        server_game_loop(server);
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
            false,                 // the card starts in the player's hand
        )
        .statistics_seed(*card)
        .fire()
        .unwrap();
    }
}

/// Game loop for the server.
fn server_game_loop(mut server: TcpServer) {
    let id = 0;
    game_status(server.game_server.lock().unwrap().battle(), id);
    loop {
        // Check for the game's end.
        if completed_rounds(&server.game_server) == 3 {
            println!("The game has ended!");
            return;
        }
        if let Some(key) = read_input() {
            match key {
                '1' | '2' | '3' => {
                    if play_card(&mut server.game_server, key.to_digit(10).unwrap(), id) {
                        server_end_turn(&mut server.game_server);
                        game_status(server.game_server.lock().unwrap().battle(), id);
                    }
                }
                'h' => print_controls(),
                'q' => break,
                _ => {}
            }
        }
    }
}

/// Game loop for the client.
fn client_game_loop(mut client: TcpClient) {
    game_status(client.game_client.lock().unwrap().battle(), client.id);
    loop {
        let round = completed_rounds(&client.game_client);
        // Check for the game's end.
        if round == 3 {
            println!("The game has ended!");
            return;
        }
        if let Some(key) = read_input() {
            match key {
                '1' | '2' | '3' => {
                    if play_card(
                        &mut client.game_client,
                        key.to_digit(10).unwrap(),
                        client.id,
                    ) {
                        // Wait for the round completion.
                        wait(|| {
                            client
                                .game_client
                                .lock()
                                .unwrap()
                                .battle()
                                .rounds()
                                .completed_rounds()
                                > round
                        });
                        game_status(client.game_client.lock().unwrap().battle(), client.id);
                    }
                }
                'h' => print_controls(),
                'q' => break,
                _ => {}
            }
        }
    }
}

/// Checks and prints the game status.
fn game_status(battle: &Battle<CustomRules>, id: TeamId<CustomRules>) {
    // Print the game state.
    print_separator();
    for team in battle.entities().teams() {
        println!("Player {} points: {}", team.id() + 1, team.objectives());
    }
    // Print the player's hand.
    println!("\nYour hand:");
    for (i, creature_id) in battle.entities().team(&id).unwrap().creatures().enumerate() {
        let creature = battle.entities().creature(creature_id).unwrap();
        println!(
            "  {} - Card of value {}",
            i + 1,
            creature.statistic(&CARD_VALUE_STAT).unwrap().value()
        );
    }
    print_separator();
}

/// Method to make the player play the card in the given slot.
fn play_card<T>(controller: &mut Arc<Mutex<T>>, card_index: u32, id: TeamId<CustomRules>) -> bool
where
    T: BattleController<CustomRules> + EventProcessor<CustomRules>,
    T: EventProcessor<CustomRules, ProcessOutput = weasel::WeaselResult<(), CustomRules>>,
{
    // Retrieve the id of the card we want to play.
    // card_index contains the 'index' of the selected card in our hand, we have to retrive the id.
    let card_id = match controller
        .lock()
        .unwrap()
        .battle()
        .entities()
        .team(&id)
        .unwrap()
        .creatures()
        .nth(card_index as usize - 1)
        .map(|e| EntityId::Creature(*e))
    {
        Some(id) => id,
        None => {
            println!("Invalid command!");
            return false;
        }
    };
    println!("Waiting for all players to make their move...");
    // Wait for our turn.
    wait(|| {
        // We have defined the rounds model to be equal to the id of the player who should act.
        *controller.lock().unwrap().battle().rounds().model() == id
    });
    // Perform the play.
    // Everything is server based, so we can't just fire events one after the other because
    // TcpClient and TcpServer are asynchronous. The quick and dirty solution is to wait for
    // each event to be acknowledged. A proper solution would be to have the server sending
    // messages to the clients and the clients themselves having a state machine.
    let last_event = controller.lock().unwrap().battle().history().len();
    StartTurn::trigger(&mut *controller.lock().unwrap(), card_id)
        .fire()
        .unwrap();
    // Wait to receive the StartTurn event validation.
    wait(|| controller.lock().unwrap().battle().history().len() > last_event);
    let last_event = controller.lock().unwrap().battle().history().len();
    ActivateAbility::trigger(&mut *controller.lock().unwrap(), card_id, PLAY_CARD_ABILITY)
        .fire()
        .unwrap();
    // Wait to receive the ActivateAbility and MoveEntity events validation.
    wait(|| controller.lock().unwrap().battle().history().len() > last_event + 1);
    EndTurn::trigger(&mut *controller.lock().unwrap())
        .fire()
        .unwrap();
    true
}

fn server_end_turn(server: &mut Arc<Mutex<Server<CustomRules>>>) {
    wait(|| server.lock().unwrap().battle().rounds().completed_rounds() % 3 == 0);
    // Decide the winner.
    let winner = winner(server);
    // Update the winner's score.
    let new_score = server
        .lock()
        .unwrap()
        .battle()
        .entities()
        .team(&winner)
        .unwrap()
        .objectives()
        + 1;
    ResetObjectives::trigger(&mut *server.lock().unwrap(), winner)
        .seed(new_score)
        .fire()
        .unwrap();
    // Remove all played cards.
    let cards = *server.lock().unwrap().battle().space().model();
    for card in cards.iter() {
        RemoveEntity::trigger(&mut *server.lock().unwrap(), card.unwrap())
            .fire()
            .unwrap();
    }
    // Close the round.
    EndRound::trigger(&mut *server.lock().unwrap())
        .fire()
        .unwrap();
}

/// Returns the number of completed rounds.
fn completed_rounds<T>(controller: &Arc<Mutex<T>>) -> TurnsCount
where
    T: BattleController<CustomRules> + EventProcessor<CustomRules>,
{
    controller
        .lock()
        .unwrap()
        .battle()
        .rounds()
        .completed_rounds()
}

/// Method to decide who won a turn.
fn winner<T>(controller: &mut Arc<Mutex<T>>) -> TeamId<CustomRules>
where
    T: BattleController<CustomRules> + EventProcessor<CustomRules>,
{
    let controller = controller.lock().unwrap();
    let table = controller.battle().space().model();
    let mut highest: Option<&Creature<CustomRules>> = None;
    for card_id in table.iter() {
        let card_id = card_id.unwrap().creature().unwrap();
        let card = controller.battle().entities().creature(&card_id).unwrap();
        let card_value = card.statistic(&CARD_VALUE_STAT).unwrap().value();
        if let Some(a) = highest {
            if card_value > a.statistic(&CARD_VALUE_STAT).unwrap().value() {
                highest = Some(card);
            }
        } else {
            highest = Some(card);
        }
    }
    *highest.unwrap().team_id()
}

/// Event callback that prints the result of the battle.
fn event_callback(
    event: &EventWrapper<CustomRules>,
    _: &BattleState<CustomRules>,
    _: &mut Option<EventQueue<CustomRules>>,
) {
    if let EventKind::ResetObjectives = event.kind() {
        let event: &ResetObjectives<CustomRules> =
            match event.as_any().downcast_ref::<ResetObjectives<_>>() {
                Some(e) => e,
                None => panic!("incorrect cast!"),
            };
        println!("Player {} won a turn!", event.id() + 1);
    }
}

// Helper methods.

/// Read a char from stdin.
fn read_input() -> Option<char> {
    std::io::stdin()
        .bytes()
        .next()
        .and_then(|result| result.ok())
        .map(|byte| byte as char)
}

/// Blocks until `func` returns true.
fn wait<F>(func: F)
where
    F: Fn() -> bool,
{
    loop {
        if func() {
            break;
        }
        thread::sleep(time::Duration::from_millis(10));
    }
}

fn print_intro() {
    println!("\nWelcome to King of the hill!\n");
    println!("You are given three cards numbered from 1 to 9.");
    println!("Each turn all players play one card, the highest wins!\n");
    print_controls();
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
