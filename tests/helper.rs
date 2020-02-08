#[cfg(feature = "serialization")]
use weasel::battle::{Battle, BattleRules};
#[cfg(feature = "serialization")]
use weasel::event::EventReceiver;
#[cfg(feature = "serialization")]
use weasel::serde::FlatVersionedEvent;

#[cfg(feature = "serialization")]
/// Serializes the history of a battle into a json string.
pub fn history_as_json<R>(battle: &Battle<R>) -> String
where
    R: BattleRules + 'static,
{
    let events: Vec<FlatVersionedEvent<R>> = battle
        .versioned_events(std::ops::Range {
            start: 0,
            end: battle.history().len() as usize,
        })
        .map(|e| e.into())
        .collect();
    serde_json::to_string(&events).unwrap()
}

#[cfg(feature = "serialization")]
/// Loads a history stored as json into an event receiver.
pub fn load_json_history<'a, R, T>(receiver: &'a mut T, json: String)
where
    R: BattleRules + 'static,
    T: EventReceiver<R>,
{
    let events: Vec<FlatVersionedEvent<R>> = serde_json::from_str(&json).unwrap();
    for event in events {
        receiver.receive(event.into()).unwrap();
    }
}
