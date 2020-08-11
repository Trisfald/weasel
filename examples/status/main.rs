use crate::rules::*;
use weasel::creature::CreatureId;
use weasel::object::ObjectId;
use weasel::team::TeamId;
use weasel::{
    Battle, BattleController, ClearStatus, CreateCreature, CreateObject, CreateTeam, EndRound,
    EntityId, EnvironmentRound, EventKind, EventTrigger, Id, InflictStatus, Server, StartRound,
};

mod rules;

static TEAM_ID: TeamId<CustomRules> = 1;
static CREATURE_ID: CreatureId<CustomRules> = 1;
static OBJECT_ID: ObjectId<CustomRules> = 2;
static ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_ID);
static ENTITY_2_ID: EntityId<CustomRules> = EntityId::Object(OBJECT_ID);

fn main() {
    // Create a server to manage the battle.
    let battle = Battle::builder(CustomRules::new()).build();
    let mut server = Server::builder(battle).build();
    // Create a team.
    CreateTeam::trigger(&mut server, TEAM_ID).fire().unwrap();
    // Spawn a creature and an object, both with 50 HEALTH.
    println!("Spawning a creature...");
    CreateCreature::trigger(&mut server, CREATURE_ID, TEAM_ID, ())
        .statistics_seed(50)
        .fire()
        .unwrap();
    println!("Spawning an object...");
    CreateObject::trigger(&mut server, OBJECT_ID, ())
        .statistics_seed(50)
        .fire()
        .unwrap();
    // Display the entities' state.
    print_state(&server);
    // Inflict a power-up status effect on the creature, with no time limit.
    println!("Inflicting a power-up on the creature...");
    InflictStatus::trigger(&mut server, ENTITY_1_ID, VIGOR)
        .potency((50, None))
        .fire()
        .unwrap();
    // Inflict a DoT status effect on the object, for two rounds.
    println!("Inflicting a DoT on the object...");
    InflictStatus::trigger(&mut server, ENTITY_2_ID, DOT)
        .potency((10, Some(2)))
        .fire()
        .unwrap();
    // Display the entities' state.
    print_state(&server);
    // Do two full turns.
    for i in 1..=2 {
        turn(&mut server, i);
    }
    // The DoT should have been cleared automatically.
    // Remove the power-up manually.
    println!("Removing the creature power-up...");
    ClearStatus::trigger(&mut server, ENTITY_1_ID, VIGOR)
        .fire()
        .unwrap();
    print_state(&server);
    // Display the link between the DoT status and the effects it created.
    print_dot_effects(&server);
}

/// Performs a turn.
fn turn(server: &mut Server<CustomRules>, turn: u32) {
    // Display in which turn we are.
    println!("Turn {}", turn);
    println!();
    // Start and end a round for the creature.
    println!("Round of Creature (1)...");
    StartRound::trigger(server, ENTITY_1_ID).fire().unwrap();
    EndRound::trigger(server).fire().unwrap();
    // Do a round for all non-actor entities, to update their statuses.
    println!("Round of environment...");
    EnvironmentRound::trigger(server).fire().unwrap();
    // Display the entities' state.
    print_state(server);
}

/// Displays briefly the state of all entities.
fn print_state(server: &Server<CustomRules>) {
    println!();
    println!("------------------------- Entities -------------------------");
    for character in server.battle().entities().characters() {
        let statuses: Vec<_> = character
            .statuses()
            .map(|status| match *status.id() {
                VIGOR => "vigor",
                DOT => "DoT",
                _ => unimplemented!(),
            })
            .collect();
        println!(
            "{:?} => health: {}, statuses: {:?}",
            character.entity_id(),
            character.statistic(&HEALTH).unwrap().value(),
            statuses
        );
    }
    println!();
}

fn print_dot_effects(server: &Server<CustomRules>) {
    println!("Event derived from the DOT status:");
    // We want to show the chain of events derived from the DOT status.
    // First find the event that put the DOT on the object.
    // We know it's first InflictStatus iterating in reverse order.
    let events = server.battle().history().events();
    let inflict_event = events
        .iter()
        .rev()
        .find(|e| e.kind() == EventKind::InflictStatus)
        .unwrap();
    println!("{:?}", inflict_event.event());
    // Get all events with inflict_event as origin.
    for event in events {
        if event.origin() == Some(inflict_event.id()) {
            println!("+-- {:?}", event.event());
        }
    }
}
