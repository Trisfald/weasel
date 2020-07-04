use crate::rules::*;
use weasel::battle::{Battle, BattleController};
use weasel::event::EventTrigger;
use weasel::serde::FlatVersionedEvent;
use weasel::Server;

mod rules;

fn main() {
    // Create a server which will manage the battle.
    let battle = Battle::builder(CustomRules::new()).build();
    let mut server = Server::builder(battle).build();
    // Fire two MakePizza events.
    MakePizza::trigger(&mut server, "margherita".to_string())
        .fire()
        .unwrap();
    MakePizza::trigger(&mut server, "diavola".to_string())
        .fire()
        .unwrap();
    // Check that our custom metric is working correctly.
    assert_eq!(
        server
            .battle()
            .metrics()
            .user_u64(PIZZAS_CREATED_METRIC.to_string()),
        Some(2)
    );
    // Print the serialized history.
    let events: Vec<FlatVersionedEvent<_>> = server
        .battle()
        .versioned_events(std::ops::Range {
            start: 0,
            end: server.battle().history().len() as usize,
        })
        .map(|e| e.into())
        .collect();
    println!("History:\n {}", serde_json::to_string(&events).unwrap());
}
