use crate::rules::PiratesRules;
use crate::rules::*;
use rand::Rng;
use std::fs::{self, File};
use std::time::SystemTime;
use std::{env, io::Read};
use weasel::ability::ActivateAbility;
use weasel::battle::{Battle, BattleState, EndBattle};
use weasel::character::{AlterStatistics, Character};
use weasel::creature::{CreateCreature, CreatureId, RemoveCreature};
use weasel::entity::EntityId;
use weasel::entropy::ResetEntropy;
use weasel::event::{EventKind, EventQueue, EventReceiver, EventTrigger, EventWrapper};
use weasel::round::{EndRound, StartRound};
use weasel::serde::FlatVersionedEvent;
use weasel::team::{CreateTeam, TeamId};
use weasel::Server;

// Constants to identify teams.
static PLAYER_TEAM: &str = "player";
static ENEMY_TEAM: &str = "enemy";
// Constants to identify creatures (ships).
static PLAYER_SHIP: CreatureId<PiratesRules> = 0;
static ENEMY_SHIP: CreatureId<PiratesRules> = 1;

pub struct Game {
    server: Server<PiratesRules>,
}

impl Game {
    pub fn new() -> Game {
        // Create a battle object with our game rules.
        // We attach a callback to the battle, so that we can display a brief commentary
        // when certain events happen!
        let battle = Battle::builder(PiratesRules::new())
            .event_callback(Box::new(commentary))
            .build();
        // Create a server to orchestrate the game.
        let mut server = Server::builder(battle).build();
        // Reset entropy with a 'random enough' seed.
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        ResetEntropy::trigger(&mut server)
            .seed(time.as_secs())
            .fire()
            .unwrap();
        // Create a team for the player.
        // Changes to the battle are always performed through events. To fire an event
        // just create a trigger object from the event itself, then call fire() on it.
        //
        // You must always give a valid 'EventProcessor' to a trigger, which can be either a
        // server or a client.
        CreateTeam::trigger(&mut server, PLAYER_TEAM.to_string())
            // The objective of this team is to defeat ENEMY_TEAM.
            .objectives_seed(ENEMY_TEAM.to_string())
            .fire()
            .unwrap();
        // Now create one ship for the player.
        CreateCreature::trigger(&mut server, PLAYER_SHIP, PLAYER_TEAM.to_string(), ())
            .fire()
            .unwrap();
        // Do the same for the enemy.
        CreateTeam::trigger(&mut server, ENEMY_TEAM.to_string())
            .objectives_seed(PLAYER_TEAM.to_string())
            .fire()
            .unwrap();
        CreateCreature::trigger(&mut server, ENEMY_SHIP, ENEMY_TEAM.to_string(), ())
            .fire()
            .unwrap();
        // Return a game object.
        Game { server }
    }

    pub fn fire_cannonball(&mut self) {
        // Before our ship can attack we must start a player round.
        StartRound::trigger(&mut self.server, EntityId::Creature(PLAYER_SHIP))
            .fire()
            .unwrap();
        // Now activate the 'cannonball' ability of the player's ship.
        // To do so we use an ActivateAbility event. We must specify the entity doing the ability,
        // the id of the ability itself and as 'activation' who is the target.
        ActivateAbility::trigger(
            &mut self.server,
            EntityId::Creature(PLAYER_SHIP),
            ABILITY_CANNONBALL.to_string(),
        )
        .activation(EntityId::Creature(ENEMY_SHIP))
        .fire()
        .unwrap();
        // After the ship's attack we just end the player round.
        EndRound::trigger(&mut self.server).fire().unwrap();
    }

    pub fn fire_grapeshot(&mut self) {
        // Some logic as fire_cannonball, but fire another ability.
        StartRound::trigger(&mut self.server, EntityId::Creature(PLAYER_SHIP))
            .fire()
            .unwrap();
        ActivateAbility::trigger(
            &mut self.server,
            EntityId::Creature(PLAYER_SHIP),
            ABILITY_GRAPESHOT.to_string(),
        )
        .activation(EntityId::Creature(ENEMY_SHIP))
        .fire()
        .unwrap();
        EndRound::trigger(&mut self.server).fire().unwrap();
    }

    pub fn enemy_round(&mut self) {
        // Before the enemy ship can attack we must start an enemy round.
        StartRound::trigger(&mut self.server, EntityId::Creature(ENEMY_SHIP))
            .fire()
            .unwrap();
        // Fire a random ability.
        let mut rng = rand::thread_rng();
        let rng_number = rng.gen_range(0, 2);
        let ability = if rng_number == 0 {
            ABILITY_CANNONBALL
        } else {
            ABILITY_GRAPESHOT
        };
        ActivateAbility::trigger(
            &mut self.server,
            EntityId::Creature(ENEMY_SHIP),
            ability.to_string(),
        )
        .activation(EntityId::Creature(PLAYER_SHIP))
        .fire()
        .unwrap();
        // After the ship's attack we just end the enemy round.
        EndRound::trigger(&mut self.server).fire().unwrap();
    }

    /// Saves the battle's history as json in a temporary file.
    pub fn save(&mut self) {
        // Collect all events in a serializable format.
        let events: Vec<FlatVersionedEvent<_>> = self
            .server
            .battle()
            .versioned_events(std::ops::Range {
                start: 0,
                end: self.server.battle().history().len() as usize,
            })
            .map(|e| e.into())
            .collect();
        // Serialize as json.
        let json = serde_json::to_string(&events).unwrap();
        // Write the json into a temporary file.
        let mut path = env::temp_dir();
        path.push("savegame");
        fs::write(path, json).unwrap();
        println!("game saved!");
    }

    /// Restores the battle's history from a json temporary file.
    pub fn load(&mut self) {
        // Read the json stored in a temporary file.
        let mut json = String::new();
        let mut path = env::temp_dir();
        path.push("savegame");
        let file = File::open(path);
        match file {
            Ok(mut file) => {
                file.read_to_string(&mut json).unwrap();
                // Deserialize all events.
                let events: Vec<FlatVersionedEvent<_>> = serde_json::from_str(&json).unwrap();
                // Replay all events in a new instance of server.
                let battle = Battle::builder(PiratesRules::new()).build();
                self.server = Server::builder(battle).build();
                for event in events {
                    self.server.receive(event.into()).unwrap();
                }
                // Attach the callback now to avoid invoking it while the events in history
                // are replayed.
                self.server.set_event_callback(Some(Box::new(commentary)));
                println!("savegame loaded!");
            }
            Err(_) => println!("no savegame found!"),
        }
    }

    // Returns the statistics of a ship.
    fn ship_stats(&self, id: CreatureId<PiratesRules>) -> (i16, i16) {
        // Retrieve the creature for the list of entities.
        let creature = self.server.battle().entities().creature(&id);
        // Be careful because the ship might have been destroyed.
        match creature {
            Some(creature) => (
                creature.statistic(&STAT_HULL).unwrap().value(),
                creature.statistic(&STAT_CREW).unwrap().value(),
            ),
            None => (0, 0),
        }
    }

    pub fn player_stats(&self) -> (i16, i16) {
        self.ship_stats(PLAYER_SHIP)
    }

    pub fn enemy_stats(&self) -> (i16, i16) {
        self.ship_stats(ENEMY_SHIP)
    }

    pub fn check_winner(&mut self) -> bool {
        // We want to return whether or not there's a winner.
        let winner: Vec<_> = self.server.battle().entities().victorious_id().collect();
        if !winner.is_empty() {
            println!("{} won!", pretty_team_id(&winner[0]));
            // End the battle as well.
            EndBattle::trigger(&mut self.server).fire().unwrap();
            true
        } else {
            false
        }
    }
}

/// Event callback that prints some commentary out of the events happening in the battle.
fn commentary(
    event: &EventWrapper<PiratesRules>,
    _: &BattleState<PiratesRules>,
    _: &mut Option<EventQueue<PiratesRules>>,
) {
    match event.kind() {
        EventKind::AlterStatistics => {
            let event: &AlterStatistics<PiratesRules> =
                match event.as_any().downcast_ref::<AlterStatistics<_>>() {
                    Some(e) => e,
                    None => panic!("incorrect cast!"),
                };
            let (hull_damage, crew_damage) = event.alteration();
            if *hull_damage != 0 {
                println!(
                    "{} took {} hull damage!",
                    pretty_creature_id(&event.id().creature().unwrap()),
                    hull_damage
                );
            }
            if *crew_damage != 0 {
                println!(
                    "{} took {} crew damage!",
                    pretty_creature_id(&event.id().creature().unwrap()),
                    crew_damage
                );
            }
        }
        EventKind::RemoveCreature => {
            let event: &RemoveCreature<PiratesRules> =
                match event.as_any().downcast_ref::<RemoveCreature<_>>() {
                    Some(e) => e,
                    None => panic!("incorrect cast!"),
                };
            println!("{} destroyed!", pretty_creature_id(event.id()));
        }
        _ => {} // Do nothing.
    }
}

fn pretty_creature_id(id: &CreatureId<PiratesRules>) -> &'static str {
    if *id == PLAYER_SHIP {
        "Player ship"
    } else {
        "Enemy ship"
    }
}

fn pretty_team_id(id: &TeamId<PiratesRules>) -> &'static str {
    if id == PLAYER_TEAM {
        "Player"
    } else {
        "Enemy"
    }
}
