use serde::{Deserialize, Serialize};
use std::any::Any;
use weasel::battle::{Battle, BattleRules};
use weasel::event::{Event, EventKind, EventProcessor, EventQueue, EventTrigger};
use weasel::user::{UserEventPacker, UserRules};
use weasel::{battle_rules, battle_rules_with_user, rules::empty::*, WeaselError, WeaselResult};

pub(crate) const PIZZAS_CREATED_METRIC: &str = "pizzas_created";

// It's not a real game so we can use generic no-op battle rules.
// We still want to override the UserRules to define how to serialize our custom event and to
// add custom metrics.
battle_rules_with_user! { CustomUserRules }

// Define our own user rules in order to have custom metrics and custom events.
#[derive(Default)]
pub struct CustomUserRules {}

impl UserRules<CustomRules> for CustomUserRules {
    // For our metrics we'll use a String id.
    type UserMetricId = String;
    // The type we will use to serialize and deserialize all user events.
    type UserEventPackage = EventPackage;
}

/// An user defined event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MakePizza {
    // A simple data field containing the pizza's name.
    name: String,
}

impl MakePizza {
    /// Returns a trigger for this event.
    /// Triggers are not strictly required, but they offer a convenient way to fire events.
    pub(crate) fn trigger<P: EventProcessor<CustomRules>>(
        processor: &mut P,
        name: String,
    ) -> MakePizzaTrigger<P> {
        MakePizzaTrigger { processor, name }
    }
}

impl Event<CustomRules> for MakePizza {
    fn verify(&self, _battle: &Battle<CustomRules>) -> WeaselResult<(), CustomRules> {
        // You should put here all the logic needed to verify if the event can be applied or not.
        // For the sake of the example the event is always accepted.
        Ok(())
    }

    fn apply(
        &self,
        battle: &mut Battle<CustomRules>,
        _event_queue: &mut Option<EventQueue<CustomRules>>,
    ) {
        // In this method you can modify the battle state or even fire other events.
        // In this example the event does nothing except increasing a metric.
        let mut writer = battle.metrics_mut();
        writer
            .add_user_u64(PIZZAS_CREATED_METRIC.to_string(), 1)
            .unwrap();
    }

    fn kind(&self) -> EventKind {
        // This user event has id 0. If you add a second user event, it should have another id.
        EventKind::UserEvent(0)
    }

    fn box_clone(&self) -> Box<dyn Event<CustomRules>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `MakePizza` event.
pub(crate) struct MakePizzaTrigger<'a, P>
where
    P: EventProcessor<CustomRules>,
{
    processor: &'a mut P,
    name: String,
}

impl<'a, P> EventTrigger<'a, CustomRules, P> for MakePizzaTrigger<'a, P>
where
    P: EventProcessor<CustomRules>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `MakePizza` event.
    fn event(&self) -> Box<dyn Event<CustomRules>> {
        Box::new(MakePizza {
            name: self.name.clone(),
        })
    }
}

/// Type to serialize and deserialize user event.
#[derive(Serialize, Deserialize)]
pub(crate) enum EventPackage {
    MakePizza(MakePizza),
}

impl UserEventPacker<CustomRules> for EventPackage {
    /// In this method we extract an event trait object out of a packaged user event.
    fn boxed(self) -> WeaselResult<Box<dyn Event<CustomRules>>, CustomRules> {
        let event = match self {
            EventPackage::MakePizza(event) => (Box::new(event) as Box<dyn Event<CustomRules>>),
        };
        Ok(event)
    }

    /// This method packages a boxed user event into an instance of EventPackage.
    fn flattened(event: Box<dyn Event<CustomRules>>) -> WeaselResult<Self, CustomRules> {
        match event.as_any().downcast_ref::<MakePizza>() {
            Some(event) => Ok(EventPackage::MakePizza(event.clone())),
            None => Err(WeaselError::UserEventPackingError(
                event.clone(),
                "bad cast".into(),
            )),
        }
    }
}
