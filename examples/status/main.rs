use crate::rules::*;
use weasel::actor::Actor;
use weasel::battle::Battle;
use weasel::creature::{CreateCreature, CreatureId};
use weasel::entity::{Entity, EntityId};
use weasel::event::EventTrigger;
use weasel::object::{CreateObject, ObjectId};
use weasel::round::{EndRound, EnvironmentRound, StartRound};
use weasel::status::{ClearStatus, InflictStatus};
use weasel::team::{CreateTeam, TeamId};
use weasel::Server;

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
    // Spawn a creature and an object, both with 100 HEALTH.
    println!("Spawning a creature...");
    CreateCreature::trigger(&mut server, CREATURE_ID, TEAM_ID, ())
        .statistics_seed(100)
        .fire()
        .unwrap();
    println!("Spawning an object...");
    CreateObject::trigger(&mut server, OBJECT_ID, ())
        .statistics_seed(100)
        .fire()
        .unwrap();
    println!();
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
    display_state(&server);
    println!();
    // Do two full turns.
    for i in 1..=2 {
        turn(&mut server, i);
        turn(&mut server, i);
    }
    // The DoT should be gone automatically.
    // Remove the power-up manually.
    println!("Removing the creature power-up...");
    ClearStatus::trigger(&mut server, ENTITY_1_ID, VIGOR)
        .fire()
        .unwrap();
    display_state(&server);
    println!();
    // Display the link between the DoT status and the effects it created.
    display_dot_effects(&server);
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
    println!();
    // Display the entities' state.
    display_state(server);
    println!();
}

/// Displays briefly the state of all entities.
fn display_state(server: &Server<CustomRules>) {
    // TODO
}

fn display_dot_effects(server: &Server<CustomRules>) {
    // TODO
}
