//! Module to manage powers.

use crate::battle::{Battle, BattleRules};
use crate::error::{WeaselError, WeaselResult};
use crate::event::{Event, EventKind, EventProcessor, EventQueue, EventRights, EventTrigger};
use crate::round::TurnStateType;
use crate::team::{Call, TeamId, TeamRules};
use crate::util::Id;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::any::Any;

/// Type to represent a special power of a team.
///
/// Powers are both a statistic and an ability. Thus, they can be used to give a team
/// a certain property and/or an activable skill.
pub type Power<R> = <<R as BattleRules>::TR as TeamRules<R>>::Power;

/// Alias for `Power<R>::Id`.
pub type PowerId<R> = <Power<R> as Id>::Id;

/// Type to drive the generation of the powers for a given team.
pub type PowersSeed<R> = <<R as BattleRules>::TR as TeamRules<R>>::PowersSeed;

/// Type to customize in which way a power is invoked.
pub type Invocation<R> = <<R as BattleRules>::TR as TeamRules<R>>::Invocation;

/// Encapsulates the data used to describe an alteration of one or more powers.
pub type PowersAlteration<R> = <<R as BattleRules>::TR as TeamRules<R>>::PowersAlteration;

/// Event to make a team invoke a power.
///
/// A team can invoke a power in between actor turns or during turns of actors
/// that belongs to itself.
///
/// # Examples
/// ```
/// use weasel::{
///     battle_rules, rules::empty::*, Battle, BattleRules, CreateTeam, EntityId,
///     EventTrigger, InvokePower, Server,
/// };
///
/// battle_rules! {}
///
/// let battle = Battle::builder(CustomRules::new()).build();
/// let mut server = Server::builder(battle).build();
///
/// let team_id = 1;
/// CreateTeam::trigger(&mut server, team_id).fire().unwrap();
///
/// let power_id = 99;
/// let result = InvokePower::trigger(&mut server, team_id, power_id).fire();
/// // We get an error because the team doesn't possess this power.
/// // The team's powers must defined in 'TeamRules'.
/// assert!(result.is_err());
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct InvokePower<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "TeamId<R>: Serialize",
            deserialize = "TeamId<R>: Deserialize<'de>"
        ))
    )]
    team_id: TeamId<R>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "PowerId<R>: Serialize",
            deserialize = "PowerId<R>: Deserialize<'de>"
        ))
    )]
    power_id: PowerId<R>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Option<Invocation<R>>: Serialize",
            deserialize = "Option<Invocation<R>>: Deserialize<'de>"
        ))
    )]
    invocation: Option<Invocation<R>>,
}

impl<R: BattleRules> InvokePower<R> {
    /// Returns a trigger for this event.
    pub fn trigger<P: EventProcessor<R>>(
        processor: &mut P,
        team_id: TeamId<R>,
        power_id: PowerId<R>,
    ) -> InvokePowerTrigger<R, P> {
        InvokePowerTrigger {
            processor,
            team_id,
            power_id,
            invocation: None,
        }
    }

    /// Returns the id of the team that is invoking the power.
    pub fn team_id(&self) -> &TeamId<R> {
        &self.team_id
    }

    /// Returns the id of the power to be invoked.
    pub fn power_id(&self) -> &PowerId<R> {
        &self.power_id
    }

    /// Returns the invocation profile for the power.
    pub fn invocation(&self) -> &Option<Invocation<R>> {
        &self.invocation
    }
}

impl<R: BattleRules> std::fmt::Debug for InvokePower<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "InvokePower {{ team_id: {:?}, power_id: {:?}, invocation: {:?} }}",
            self.team_id, self.power_id, self.invocation
        )
    }
}

impl<R: BattleRules> Clone for InvokePower<R> {
    fn clone(&self) -> Self {
        InvokePower {
            team_id: self.team_id.clone(),
            power_id: self.power_id.clone(),
            invocation: self.invocation.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for InvokePower<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        // Verify that the team exists.
        if let Some(team) = battle.entities().team(&self.team_id) {
            // Verify that the team can invoke a power at this stage.
            let ready = match battle.state.rounds.state() {
                TurnStateType::Ready => true,
                TurnStateType::Started(entities) => entities.iter().any(|e| {
                    battle
                        .state
                        .entities
                        .actor(&e)
                        .map(|a| *a.team_id() == self.team_id)
                        .unwrap_or(false)
                }),
            };
            if !ready {
                return Err(WeaselError::TeamNotReady(self.team_id.clone()));
            }
            // Verify that the team possesses this power.
            if let Some(power) = team.power(&self.power_id) {
                // Verify if this power can be activated.
                battle
                    .rules
                    .team_rules()
                    .invocable(&battle.state, Call::new(team, power, &self.invocation))
                    .map_err(|err| {
                        WeaselError::PowerNotInvocable(
                            self.team_id.clone(),
                            self.power_id.clone(),
                            Box::new(err),
                        )
                    })
            } else {
                Err(WeaselError::PowerNotKnown(
                    self.team_id.clone(),
                    self.power_id.clone(),
                ))
            }
        } else {
            Err(WeaselError::TeamNotFound(self.team_id.clone()))
        }
    }

    fn apply(&self, battle: &mut Battle<R>, event_queue: &mut Option<EventQueue<R>>) {
        let team = battle
            .state
            .entities
            .team(&self.team_id)
            .unwrap_or_else(|| panic!("constraint violated: team {:?} not found", self.team_id));
        let power = team.power(&self.power_id).unwrap_or_else(|| {
            panic!(
                "constraint violated: power {:?} not found in team {:?}",
                self.power_id, self.team_id
            )
        });
        battle.rules.team_rules().invoke(
            &battle.state,
            Call::new(team, power, &self.invocation),
            event_queue,
            &mut battle.entropy,
            &mut battle.metrics.write_handle(),
        );
    }

    fn kind(&self) -> EventKind {
        EventKind::InvokePower
    }

    fn box_clone(&self) -> Box<dyn Event<R> + Send> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn rights<'a>(&'a self, _: &'a Battle<R>) -> EventRights<'a, R> {
        EventRights::Team(&self.team_id)
    }
}

/// Trigger to build and fire an `InvokePower` event.
pub struct InvokePowerTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    team_id: TeamId<R>,
    power_id: PowerId<R>,
    invocation: Option<Invocation<R>>,
}

impl<'a, R, P> InvokePowerTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    /// Adds an invocation profile to customize this power instance.
    pub fn invocation(&'a mut self, invocation: Invocation<R>) -> &'a mut Self {
        self.invocation = Some(invocation);
        self
    }
}

impl<'a, R, P> EventTrigger<'a, R, P> for InvokePowerTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns an `InvokePower` event.
    fn event(&self) -> Box<dyn Event<R> + Send> {
        Box::new(InvokePower {
            team_id: self.team_id.clone(),
            power_id: self.power_id.clone(),
            invocation: self.invocation.clone(),
        })
    }
}
