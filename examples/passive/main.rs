use crate::rules::*;
use weasel::actor::Actor;
use weasel::battle::Battle;
use weasel::creature::{CreateCreature, CreatureId};
use weasel::entity::{Entity, EntityId};
use weasel::event::EventTrigger;
use weasel::round::{EndRound, StartRound};
use weasel::team::{CreateTeam, TeamId};
use weasel::Server;

mod rules;

static TEAM_ID: TeamId<CustomRules> = 1;
static CREATURE_1_ID: CreatureId<CustomRules> = 1;
static CREATURE_2_ID: CreatureId<CustomRules> = 2;
static ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
static ENTITY_2_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_2_ID);

fn main() {
    // Create a server to manage the battle.
    let battle = Battle::builder(CustomRules::new()).build();
    let mut server = Server::builder(battle).build();
    // Create a team.
    CreateTeam::trigger(&mut server, TEAM_ID).fire().unwrap();
    println!("Spawning two creatures...");
    println!();
    // Spawn a creature with a single ability: PUNCH.
    CreateCreature::trigger(&mut server, CREATURE_1_ID, TEAM_ID, ())
        .abilities_seed(vec![PUNCH])
        .fire()
        .unwrap();
    // Spawn a creature with two abilities: PUNCH and POWER_UP.
    CreateCreature::trigger(&mut server, CREATURE_2_ID, TEAM_ID, ())
        .abilities_seed(vec![PUNCH, POWER_UP])
        .fire()
        .unwrap();
    println!("Now doing three turns of combat. Notice how Creature (2) punches get more powerful!");
    println!();
    // Carry out three turns.
    for i in 0..3 {
        turn(&mut server, i);
    }
}

/// Does a turn, containing a round for each creatures.
fn turn(server: &mut Server<CustomRules>, turn: u32) {
    // Display in which turn we are.
    println!("Turn {}", turn + 1);
    println!();
    print_power(server, CREATURE_1_ID);
    print_power(server, CREATURE_2_ID);
    // Start and end a round for the first creature.
    println!("Round of Creature (1)...");
    StartRound::trigger(server, ENTITY_1_ID).fire().unwrap();
    EndRound::trigger(server).fire().unwrap();
    // Start and end a round for the second creature.
    println!("Round of Creature (2)...");
    StartRound::trigger(server, ENTITY_2_ID).fire().unwrap();
    EndRound::trigger(server).fire().unwrap();
    print_power(server, CREATURE_1_ID);
    print_power(server, CREATURE_2_ID);
    println!();
}

/// Displays the punch power of a creature.
fn print_power(server: &Server<CustomRules>, id: CreatureId<CustomRules>) {
    let creature = server.battle().entities().creature(&id).unwrap();
    println!(
        "{} punch power: {:?}",
        creature.entity_id(),
        creature.ability(&PUNCH).unwrap().power()
    );
}
