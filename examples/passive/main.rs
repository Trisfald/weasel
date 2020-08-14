use crate::rules::*;
use weasel::creature::CreatureId;
use weasel::team::TeamId;
use weasel::{
    Actor, Battle, BattleController, CreateCreature, CreateTeam, EndTurn, Entity, EntityId,
    EventTrigger, Server, StartTurn,
};

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
    println!(
        "Now doing three rounds of combat. Notice how Creature (2) punches get more powerful!"
    );
    println!();
    // Carry out three round.
    for i in 0..3 {
        round(&mut server, i);
    }
}

/// Does a round, containing a turn for each creatures.
fn round(server: &mut Server<CustomRules>, turn: u32) {
    // Display in which round we are.
    println!("Round {}", turn + 1);
    println!();
    print_power(server, CREATURE_1_ID);
    print_power(server, CREATURE_2_ID);
    // Start and end a turn for the first creature.
    println!("Turn of Creature (1)...");
    StartTurn::trigger(server, ENTITY_1_ID).fire().unwrap();
    EndTurn::trigger(server).fire().unwrap();
    // Start and end a turn for the second creature.
    println!("Turn of Creature (2)...");
    StartTurn::trigger(server, ENTITY_2_ID).fire().unwrap();
    EndTurn::trigger(server).fire().unwrap();
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
