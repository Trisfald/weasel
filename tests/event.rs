#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::marker::PhantomData;
use weasel::ability::ActivateAbility;
use weasel::actor::{Action, Actor, ActorRules, AlterAbilities, RegenerateAbilities};
use weasel::battle::{Battle, BattleRules, BattleState, EndBattle};
use weasel::character::{AlterStatistics, RegenerateStatistics};
use weasel::creature::{ConvertCreature, CreateCreature, RemoveCreature};
use weasel::entity::EntityId;
use weasel::entropy::{Entropy, ResetEntropy};
use weasel::event::{
    Conditional, DummyEvent, Event, EventKind, EventProcessor, EventQueue, EventTrigger,
};
use weasel::fight::ApplyImpact;
use weasel::metric::WriteMetrics;
use weasel::object::{CreateObject, RemoveObject};
use weasel::round::{EndRound, ResetRounds, StartRound};
use weasel::rules::ability::SimpleAbility;
#[cfg(feature = "serialization")]
use weasel::serde::FlatEvent;
use weasel::space::{AlterSpace, MoveEntity, ResetSpace};
use weasel::team::{
    ConcludeObjectives, Conclusion, CreateTeam, Relation, RemoveTeam, ResetObjectives, SetRelations,
};
#[cfg(feature = "serialization")]
use weasel::user::UserEventPacker;
use weasel::user::{UserMetricId, UserRules};
use weasel::{battle_rules, battle_rules_with_actor, battle_rules_with_user, rules::empty::*};
use weasel::{WeaselError, WeaselResult};

#[cfg(feature = "serialization")]
mod helper;

const TEAM_1_ID: u32 = 1;
const CREATURE_1_ID: u32 = 1;

/// Declare an user event.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct MyEvent<R> {
    data: String,
    #[cfg_attr(feature = "serialization", serde(skip))]
    _phantom: PhantomData<R>,
}

impl<R: BattleRules> MyEvent<R> {
    /// Returns a trigger for this event.
    pub fn trigger<P: EventProcessor<R>>(processor: &mut P, data: String) -> MyEventTrigger<R, P> {
        MyEventTrigger {
            processor,
            data,
            _phantom: PhantomData,
        }
    }
}

impl<R> std::fmt::Debug for MyEvent<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MyEvent {{ data: {:?} }}", self.data)
    }
}

impl<R> Clone for MyEvent<R> {
    fn clone(&self) -> Self {
        MyEvent {
            data: self.data.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for MyEvent<R>
where
    UserMetricId<R>: Default,
{
    fn verify(&self, _battle: &Battle<R>) -> WeaselResult<(), R> {
        Ok(())
    }

    fn apply(&self, battle: &mut Battle<R>, _event_queue: &mut Option<EventQueue<R>>) {
        battle
            .metrics_mut()
            .add_user_u64(UserMetricId::<R>::default(), 1)
            .unwrap();
    }

    fn kind(&self) -> EventKind {
        EventKind::UserEvent(0)
    }

    fn box_clone(&self) -> Box<dyn Event<R>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire an `MyEvent` event.
pub struct MyEventTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    data: String,
    _phantom: PhantomData<R>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for MyEventTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
    UserMetricId<R>: Default,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `MyEvent` event.
    fn event(&self) -> Box<dyn Event<R>> {
        Box::new(MyEvent {
            data: self.data.clone(),
            _phantom: self._phantom,
        })
    }
}

#[test]
fn conditional() {
    #[derive(Default)]
    pub struct CustomActorRules {}

    impl ActorRules<CustomRules> for CustomActorRules {
        type Ability = SimpleAbility<u32, u32>;
        type AbilitiesSeed = ();
        type Activation = u32;
        type AbilitiesAlteration = u32;

        fn generate_abilities(
            &self,
            _: &Option<Self::AbilitiesSeed>,
            _entropy: &mut Entropy<CustomRules>,
            _metrics: &mut WriteMetrics<CustomRules>,
        ) -> Box<dyn Iterator<Item = Self::Ability>> {
            let v = vec![SimpleAbility::new(ABILITY_ID, POWER)];
            Box::new(v.into_iter())
        }

        fn alter(
            &self,
            actor: &mut dyn Actor<CustomRules>,
            alteration: &Self::AbilitiesAlteration,
            _entropy: &mut Entropy<CustomRules>,
            _metrics: &mut WriteMetrics<CustomRules>,
        ) {
            actor
                .ability_mut(&ABILITY_ID)
                .unwrap()
                .set_power(*alteration);
        }

        fn activate(
            &self,
            _state: &BattleState<CustomRules>,
            action: Action<CustomRules>,
            mut event_queue: &mut Option<EventQueue<CustomRules>>,
            _entropy: &mut Entropy<CustomRules>,
            _metrics: &mut WriteMetrics<CustomRules>,
        ) {
            AlterAbilities::trigger(&mut event_queue, ENTITY_1_ID, action.activation.unwrap())
                .fire();
            Conditional::new(
                DummyEvent::trigger(&mut event_queue),
                std::rc::Rc::new(|state: &BattleState<CustomRules>| {
                    state
                        .entities()
                        .actor(&ENTITY_1_ID)
                        .unwrap()
                        .ability(&ABILITY_ID)
                        .unwrap()
                        .power()
                        == POWER * 2
                }),
            )
            .fire();
        }
    }

    const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
    const ABILITY_ID: u32 = 1;
    const POWER: u32 = 10;

    battle_rules_with_actor! { CustomActorRules }

    // Create a battle with one creature.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    // Start round.
    util::start_round(&mut server, &ENTITY_1_ID);
    // Fire an ability to alter the creature's ability, which will also fire
    // a dummy event if the power is twice the original.
    assert_eq!(
        ActivateAbility::trigger(&mut server, ENTITY_1_ID, ABILITY_ID)
            .activation(POWER)
            .fire()
            .or_else(|err| err.filter(|err| {
                if let WeaselError::ConditionUnsatisfied = err {
                    false
                } else {
                    true
                }
            }))
            .err(),
        None
    );
    // Check that a dummy event is not created because power is not the original one * 2.
    let events = &server.battle().history().events();
    assert_eq!(events[events.len() - 1].kind(), EventKind::AlterAbilities);
    // Refire ability with activation of original power * 2.
    assert_eq!(
        ActivateAbility::trigger(&mut server, ENTITY_1_ID, ABILITY_ID)
            .activation(POWER * 2)
            .fire()
            .err(),
        None
    );
    // Check that this time the dummy event is there.
    let events = &server.battle().history().events();
    assert_eq!(events[events.len() - 1].kind(), EventKind::DummyEvent);
}

macro_rules! user_event_check {
    ($server: expr, $data: expr) => {{
        let event = &$server.battle().history().events()[0];
        let my_event: &MyEvent<CustomRules> =
            match event.as_any().downcast_ref::<MyEvent<CustomRules>>() {
                Some(b) => b,
                None => panic!("incorrect cast!"),
            };
        assert_eq!(my_event.data, $data);
    }};
}

#[test]
fn user_event() {
    // Define custom user rules.
    #[derive(Default)]
    struct CustomUserRules {}

    impl UserRules<CustomRules> for CustomUserRules {
        type UserMetricId = u32;
        #[cfg(feature = "serialization")]
        type UserEventPackage = ();
    }

    battle_rules_with_user! { CustomUserRules }
    // Create a server.
    let mut server = util::server(CustomRules::new());
    // Fire an user event.
    let data = "my event!".to_string();
    assert_eq!(
        MyEvent::trigger(&mut server, data.clone()).fire().err(),
        None
    );
    // Check that the user event is correct.
    user_event_check!(server, data);
    // Check that the user metric was increased.
    assert_eq!(
        server
            .battle()
            .metrics()
            .user_u64(UserMetricId::<CustomRules>::default()),
        Some(1)
    );
    // Verify if Debug works for a boxed user event.
    let event = MyEvent::trigger(&mut server, data.clone()).event();
    assert!(
        format!("{:?}", event).contains(&format!("{:?}", data)),
        "{:?} does not contain {:?}",
        event,
        data
    );
}

#[cfg(feature = "serialization")]
#[test]
fn user_event_serde() {
    // Define a package for the user events.
    #[derive(Serialize, Deserialize)]
    enum Package {
        MyEvent(MyEvent<CustomRules>),
    }

    impl UserEventPacker<CustomRules> for Package {
        fn boxed(self) -> WeaselResult<Box<dyn Event<CustomRules>>, CustomRules> {
            let event = match self {
                Package::MyEvent(event) => (Box::new(event) as Box<dyn Event<CustomRules>>),
            };
            Ok(event)
        }

        fn flattened(event: Box<dyn Event<CustomRules>>) -> WeaselResult<Self, CustomRules> {
            match event.as_any().downcast_ref::<MyEvent<CustomRules>>() {
                Some(event) => Ok(Package::MyEvent(event.clone())),
                None => Err(WeaselError::UserEventPackingError(
                    event.clone(),
                    "bad cast".into(),
                )),
            }
        }
    }

    // Define custom user rules.
    #[derive(Default)]
    struct CustomUserRules {}

    impl UserRules<CustomRules> for CustomUserRules {
        type UserMetricId = u32;
        type UserEventPackage = Package;
    }

    battle_rules_with_user! { CustomUserRules }
    // Create a server.
    let mut server = util::server(CustomRules::new());
    // Fire an user event.
    let data = "my event!".to_string();
    assert_eq!(
        MyEvent::trigger(&mut server, data.clone()).fire().err(),
        None
    );
    // Save the battle.
    let history_json = helper::history_as_json(server.battle());
    // Restore the battle.
    let mut server = util::server(CustomRules::new());
    helper::load_json_history(&mut server, history_json);
    // Check that the user event is correct.
    user_event_check!(server, data);
}

/// Returns a vector containig an instance of all possible events.
macro_rules! events_vec {
    () => {{
        battle_rules! {}
        const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
        const ABILITY_1_ID: u32 = 1;
        const OBJECT_1_ID: u32 = 1;
        // Collect all events into a vector.
        let mut events: Vec<Box<dyn Event<CustomRules>>> = Vec::new();
        events.push(DummyEvent::trigger(&mut ()).event());
        events.push(CreateTeam::trigger(&mut (), TEAM_1_ID).event());
        events.push(CreateCreature::trigger(&mut (), TEAM_1_ID, CREATURE_1_ID, ()).event());
        events.push(CreateObject::trigger(&mut (), OBJECT_1_ID, ()).event());
        events.push(MoveEntity::trigger(&mut (), ENTITY_1_ID, ()).event());
        events.push(StartRound::trigger(&mut (), ENTITY_1_ID).event());
        events.push(EndRound::trigger(&mut ()).event());
        events.push(ActivateAbility::trigger(&mut (), ENTITY_1_ID, ABILITY_1_ID).event());
        events.push(ApplyImpact::trigger(&mut (), ()).event());
        events.push(AlterStatistics::trigger(&mut (), ENTITY_1_ID, ()).event());
        events.push(AlterAbilities::trigger(&mut (), ENTITY_1_ID, ()).event());
        events.push(RegenerateStatistics::trigger(&mut (), ENTITY_1_ID.clone()).event());
        events.push(RegenerateAbilities::trigger(&mut (), ENTITY_1_ID.clone()).event());
        events.push(ConvertCreature::trigger(&mut (), CREATURE_1_ID, TEAM_1_ID).event());
        events.push(
            SetRelations::trigger(&mut (), &[(TEAM_1_ID, TEAM_1_ID, Relation::Ally)]).event(),
        );
        events.push(ConcludeObjectives::trigger(&mut (), TEAM_1_ID, Conclusion::Victory).event());
        events.push(RemoveCreature::trigger(&mut (), CREATURE_1_ID).event());
        events.push(RemoveObject::trigger(&mut (), OBJECT_1_ID).event());
        events.push(RemoveTeam::trigger(&mut (), TEAM_1_ID).event());
        events.push(AlterSpace::trigger(&mut (), ()).event());
        events.push(ResetEntropy::trigger(&mut ()).event());
        events.push(ResetObjectives::trigger(&mut (), TEAM_1_ID).event());
        events.push(ResetRounds::trigger(&mut ()).event());
        events.push(ResetSpace::trigger(&mut ()).event());
        events.push(EndBattle::trigger(&mut ()).event());
        events
    }};
}

#[test]
fn events_debug() {
    let events = events_vec!();
    for event in events {
        assert!(
            format!("{:?}", event).contains(&format!("{:?}", event.kind())),
            "{:?} does not contain {:?}",
            event,
            event.kind()
        );
    }
}

#[cfg(feature = "serialization")]
#[test]
fn events_serde() {
    let events = events_vec!();
    // Serialize all events.
    let flat_events: Vec<_> = events
        .iter()
        .cloned()
        .map(|e| FlatEvent::flattened(e))
        .collect();
    let json = serde_json::to_string(&flat_events).unwrap();
    // Deserialize all events.
    let deserialized_flat_events: Vec<FlatEvent<_>> = serde_json::from_str(&json).unwrap();
    let deserialized_events: Vec<_> = deserialized_flat_events
        .into_iter()
        .map(|e| e.boxed())
        .collect();
    // Ser/de should work without any error.
    // Events should 'roughly' be the same as before (Eq for Events checks only the event kind).
    assert_eq!(deserialized_events, events);
}
