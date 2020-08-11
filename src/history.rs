//! History of events.

use crate::battle::BattleRules;
use crate::error::{WeaselError, WeaselResult};
use crate::event::EventId;
use crate::event::EventWrapper;
use std::convert::TryInto;

/// History is the place where all events are kept, in a way such that they
/// construct a single, consistent timeline.
pub struct History<R: BattleRules> {
    events: Vec<EventWrapper<R>>,
}

impl<R: BattleRules> History<R> {
    /// Creates a new History.
    pub(crate) fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Returns all events inside this timeline.
    pub fn events(&self) -> &[EventWrapper<R>] {
        &self.events
    }

    /// Stores a new event in the history logs.
    pub(crate) fn archive(&mut self, event: &EventWrapper<R>) {
        assert_eq!(event.id() as usize, self.events.len());
        self.events.push(event.clone());
    }

    /// Verifies if an event has an id compatible with the current timeline.
    /// Timeline only accepts monotonically increasing ids with no gaps.
    pub(crate) fn verify_event(&self, event: &EventWrapper<R>) -> WeaselResult<(), R> {
        if event.id() as usize != self.events.len() {
            return Err(WeaselError::NonContiguousEventId(
                event.id(),
                self.events.len().try_into().unwrap(),
            ));
        }
        Ok(())
    }

    /// Returns the id for the next event.
    pub(crate) fn next_id(&self) -> EventId {
        self.events.len().try_into().unwrap()
    }

    /// Returns the number of events in this history.
    pub fn len(&self) -> EventId {
        self.events.len().try_into().unwrap()
    }

    /// Returns whether this history is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{DummyEvent, EventTrigger};
    use crate::{battle_rules, rules::empty::*};

    #[test]
    fn verify_id() {
        battle_rules! {}
        let mut history = History::<CustomRules>::new();
        let mut try_archive = |id| -> WeaselResult<(), _> {
            let event = EventWrapper::new(id, None, DummyEvent::trigger(&mut ()).event());
            history.verify_event(&event)?;
            history.archive(&event);
            Ok(())
        };
        assert!(try_archive(3).is_err());
        assert!(try_archive(0).is_ok());
        assert!(try_archive(2).is_err());
        assert!(try_archive(1).is_ok());
        assert!(try_archive(1).is_err());
        assert!(try_archive(0).is_err());
    }
}
