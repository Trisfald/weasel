//! Module to handle combat.

use crate::battle::{Battle, BattleRules, BattleState};
use crate::character::Character;
use crate::entropy::Entropy;
use crate::error::WeaselResult;
use crate::event::{Event, EventKind, EventProcessor, EventQueue, EventTrigger};
use crate::metric::WriteMetrics;
use crate::status::{Application, AppliedStatus};
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::Debug;

/// Rules to determine how combat works. They manage the damage dealt,
/// accuracy of attacks and, more in general, how to apply consequences of abilities.
pub trait FightRules<R: BattleRules> {
    #[cfg(not(feature = "serialization"))]
    /// See [Impact](type.Impact.html).
    type Impact: Clone + Debug;
    #[cfg(feature = "serialization")]
    /// See [Impact](type.Impact.html).
    type Impact: Clone + Debug + Serialize + for<'a> Deserialize<'a>;

    #[cfg(not(feature = "serialization"))]
    /// See [Potency](../status/type.Potency.html).
    type Potency: Clone + Debug;
    #[cfg(feature = "serialization")]
    /// See [Potency](../status/type.Potency.html).
    type Potency: Clone + Debug + Serialize + for<'a> Deserialize<'a>;

    /// Takes an impact and generates one or more events to change the state of creatures or
    /// other objects.
    ///
    /// The provided implementation does nothing.
    fn apply_impact(
        &self,
        _state: &BattleState<R>,
        _impact: &Self::Impact,
        _event_queue: &mut Option<EventQueue<R>>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) {
    }

    /// Applies the side effects of a status when it's inflicted upon a character.
    /// `application` contains the context in which the status is applied.
    ///
    /// The status is automatically registered on the character before the call to this method.
    ///
    /// The provided implementation does nothing.
    fn apply_status(
        &self,
        _state: &BattleState<R>,
        _character: &dyn Character<R>,
        _application: Application<R>,
        _event_queue: &mut Option<EventQueue<R>>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) {
    }

    /// Applies the periodic side effects of a status.
    /// Returns `true` if the status should end after this update.
    ///
    /// For actors status updates happen at the start of their round.\
    /// For non-actor characters status updates happen when the event `EnvironmentRound` is fired.
    ///
    /// The provided implementation does nothing and never terminates statuses.
    fn update_status(
        &self,
        _state: &BattleState<R>,
        _character: &dyn Character<R>,
        _status: &AppliedStatus<R>,
        _event_queue: &mut Option<EventQueue<R>>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) -> bool {
        false
    }

    /// Removes the side effects of a status when the latter is removed from a character.
    ///
    /// The character is guaranteed to be affected by `status`.
    /// The status will be automatically dropped immediately after this method.
    ///
    /// The provided implementation does nothing.
    fn delete_status(
        &self,
        _state: &BattleState<R>,
        _character: &dyn Character<R>,
        _status: &AppliedStatus<R>,
        _event_queue: &mut Option<EventQueue<R>>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) {
    }
}

/// Impacts encapsulate information about which creatures or areas are affected
/// and what force is applied to them.
///
/// More specifically, an impact should contain
/// the necessary data to generate altering events on creatures or other objects.\
/// It's important to understand that an impact is an indirection between an ability's output
/// and its effect on the world. For instance, throwing a bomb could be considered the
/// ability while the bomb's explosion would be the impact; the explosion might then
/// cause damage to one or more creatures.
pub type Impact<R> = <<R as BattleRules>::FR as FightRules<R>>::Impact;

/// An event to apply an impact on the game world.
///
/// # Examples
/// ```
/// use weasel::battle::{Battle, BattleRules};
/// use weasel::event::{EventTrigger, EventKind};
/// use weasel::fight::ApplyImpact;
/// use weasel::{Server, battle_rules, rules::empty::*};
///
/// battle_rules! {}
///
/// let battle = Battle::builder(CustomRules::new()).build();
/// let mut server = Server::builder(battle).build();
///
/// let impact = ();
/// ApplyImpact::trigger(&mut server, impact).fire().unwrap();
/// assert_eq!(
///     server.battle().history().events()[0].kind(),
///     EventKind::ApplyImpact
/// );
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ApplyImpact<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Impact<R>: Serialize",
            deserialize = "Impact<R>: Deserialize<'de>"
        ))
    )]
    impact: Impact<R>,
}

impl<R: BattleRules> ApplyImpact<R> {
    /// Returns a trigger for this event.
    pub fn trigger<'a, P: EventProcessor<R>>(
        processor: &'a mut P,
        impact: Impact<R>,
    ) -> ApplyImpactTrigger<'a, R, P> {
        ApplyImpactTrigger { processor, impact }
    }

    /// Returns the impact inside this event.
    pub fn impact(&self) -> &Impact<R> {
        &self.impact
    }
}

impl<R: BattleRules> std::fmt::Debug for ApplyImpact<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ApplyImpact {{ impact: {:?} }}", self.impact)
    }
}

impl<R: BattleRules> Clone for ApplyImpact<R> {
    fn clone(&self) -> Self {
        ApplyImpact {
            impact: self.impact.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for ApplyImpact<R> {
    fn verify(&self, _: &Battle<R>) -> WeaselResult<(), R> {
        // For simplicity, don't verify an impact.
        // Trust the server to generate *processable* impacts.
        // `apply` should take care of generating correct events in all cases.
        Ok(())
    }

    fn apply(&self, battle: &mut Battle<R>, event_queue: &mut Option<EventQueue<R>>) {
        battle.rules.fight_rules().apply_impact(
            &battle.state,
            &self.impact,
            event_queue,
            &mut battle.entropy,
            &mut battle.metrics.write_handle(),
        );
    }

    fn kind(&self) -> EventKind {
        EventKind::ApplyImpact
    }

    fn box_clone(&self) -> Box<dyn Event<R>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire an `ApplyImpact` event.
pub struct ApplyImpactTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    impact: Impact<R>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for ApplyImpactTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns an `ApplyImpact` event.
    fn event(&self) -> Box<dyn Event<R>> {
        Box::new(ApplyImpact {
            impact: self.impact.clone(),
        })
    }
}
