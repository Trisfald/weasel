use crate::rules::*;
use std::time::SystemTime;
use weasel::team::TeamId;
use weasel::{
    Battle, BattleController, CreateCreature, CreateTeam, EndTurn, EventTrigger, RemoveCreature,
    ResetEntropy, Server, StartTurn,
};

mod rules;

static TEAM_ID: TeamId<CustomRules> = 1;

fn main() {
    // Create a server to manage the battle.
    let battle = Battle::builder(CustomRules::new()).build();
    let mut server = Server::builder(battle).build();
    // Reset entropy with a 'random enough' seed.
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    ResetEntropy::trigger(&mut server)
        .seed(time.as_secs())
        .fire()
        .unwrap();
    // Spawn five creatures.
    CreateTeam::trigger(&mut server, TEAM_ID).fire().unwrap();
    for i in 1..=5 {
        CreateCreature::trigger(&mut server, i, TEAM_ID, ())
            // The speed of the creature is equal to id * 2 + 10.
            .statistics_seed((i * 2 + 10).into())
            .fire()
            .unwrap();
    }
    // Carry out five turn.
    for _ in 0..5 {
        turn(&mut server);
    }
    // Remove one creature.
    println!("Creature (1) removed!");
    println!();
    RemoveCreature::trigger(&mut server, 1).fire().unwrap();
    // Do a final turn.
    turn(&mut server);
}

fn turn(server: &mut Server<CustomRules>) {
    // Display in which turn we are.
    println!(
        "Turn {} - Initiative table:",
        server.battle().rounds().completed_turns() + 1
    );
    println!();
    // Display the order of initiative.
    let initiative = server.battle().rounds().model();
    println!("{}", initiative);
    // Find out who should act.
    // In this game it's always the creature at the top of the initiative table.
    let actor_id = initiative.top();
    println!("It's the turn of: {}", actor_id);
    println!();
    // Start the turn.
    StartTurn::trigger(server, actor_id).fire().unwrap();
    // Since this's an example, creatures do nothing and immediately end the turn.
    EndTurn::trigger(server).fire().unwrap();
}
