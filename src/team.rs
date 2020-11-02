//! Teams of entities.

use crate::battle::{Battle, BattleRules, BattleState};
use crate::creature::{Creature, CreatureId};
use crate::entropy::Entropy;
use crate::error::{WeaselError, WeaselResult};
use crate::event::{Event, EventKind, EventProcessor, EventQueue, EventTrigger};
use crate::metric::system::*;
use crate::metric::{ReadMetrics, WriteMetrics};
use crate::power::{Invocation, Power, PowerId, PowersAlteration, PowersSeed};
use crate::util::{collect_from_iter, Id};
use indexmap::IndexMap;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter, Result};
use std::hash::{Hash, Hasher};
use std::{any::Any, iter};

type Powers<R> = IndexMap<
    <<<R as BattleRules>::TR as TeamRules<R>>::Power as Id>::Id,
    <<R as BattleRules>::TR as TeamRules<R>>::Power,
>;

/// A team is an alliance of entities.
///
/// A team represents the unit of control of a player. Teams must achieve their objectives in
/// order to win the battle.
pub struct Team<R: BattleRules> {
    /// The id of this team.
    id: TeamId<R>,
    /// Ids of all creatures which are currently part of this team.
    creatures: Vec<CreatureId<R>>,
    /// All the team's powers.
    powers: Powers<R>,
    /// `Conclusion`, if any, reached by this team.
    conclusion: Option<Conclusion>,
    /// Team objectives.
    objectives: Objectives<R>,
}

impl<R: BattleRules> Team<R> {
    /// Returns an iterator over creatures.
    pub fn creatures(&self) -> impl Iterator<Item = &CreatureId<R>> {
        Box::new(self.creatures.iter())
    }

    pub(crate) fn creatures_mut(&mut self) -> &mut Vec<CreatureId<R>> {
        &mut self.creatures
    }

    /// Returns an iterator over powers.
    pub fn powers(&self) -> impl Iterator<Item = &Power<R>> {
        Box::new(self.powers.values())
    }

    /// Returns a mutable iterator over powers.
    pub fn powers_mut(&mut self) -> impl Iterator<Item = &mut Power<R>> {
        Box::new(self.powers.values_mut())
    }

    /// Returns the power with the given id.
    pub fn power(&self, id: &PowerId<R>) -> Option<&Power<R>> {
        self.powers.get(id)
    }

    /// Returns a mutable reference to the power with the given id.
    pub fn power_mut(&mut self, id: &PowerId<R>) -> Option<&mut Power<R>> {
        self.powers.get_mut(id)
    }

    /// Adds a new power. Replaces an existing power with the same id.
    /// Returns the replaced power, if present.
    pub fn add_power(&mut self, power: Power<R>) -> Option<Power<R>> {
        self.powers.insert(power.id().clone(), power)
    }

    /// Removes a power.
    /// Returns the removed power, if present.
    pub fn remove_power(&mut self, id: &PowerId<R>) -> Option<Power<R>> {
        self.powers.remove(id)
    }

    /// Returns the conclusion reached by this team, if any.
    pub fn conclusion(&self) -> Option<Conclusion> {
        self.conclusion
    }

    /// Returns the team's objectives.
    pub fn objectives(&self) -> &Objectives<R> {
        &self.objectives
    }

    /// Removes a creature id from this team.
    ///
    /// # Panics
    ///
    /// Panics if the given creature id is not part of the team.
    ///
    pub(crate) fn remove_creature(&mut self, creature: &CreatureId<R>) {
        let index = self.creatures.iter().position(|x| x == creature).expect(
            "constraint violated: creature is in a team, \
             but such team doesn't contain the creature",
        );
        self.creatures.remove(index);
    }
}

impl<R: BattleRules> Id for Team<R> {
    type Id = TeamId<R>;

    fn id(&self) -> &TeamId<R> {
        &self.id
    }
}

/// Collection of rules to manage teams of creatures.
pub trait TeamRules<R: BattleRules> {
    #[cfg(not(feature = "serialization"))]
    /// See [TeamId](type.TeamId.html).
    type Id: Hash + Eq + PartialOrd + Clone + Debug + Send;
    #[cfg(feature = "serialization")]
    /// See [TeamId](type.TeamId.html).
    type Id: Hash + Eq + PartialOrd + Clone + Debug + Send + Serialize + for<'a> Deserialize<'a>;

    /// See [Power](../power/type.Power.html).
    type Power: Id + 'static;

    #[cfg(not(feature = "serialization"))]
    /// See [PowersSeed](../power/type.PowersSeed.html).
    type PowersSeed: Clone + Debug + Send;
    #[cfg(feature = "serialization")]
    /// See [PowersSeed](../power/type.PowersSeed.html).
    type PowersSeed: Clone + Debug + Send + Serialize + for<'a> Deserialize<'a>;

    #[cfg(not(feature = "serialization"))]
    /// See [Invocation](../power/type.Invocation.html).
    type Invocation: Clone + Debug + Send;
    #[cfg(feature = "serialization")]
    /// See [Invocation](../power/type.Invocation.html).
    type Invocation: Clone + Debug + Send + Serialize + for<'a> Deserialize<'a>;

    #[cfg(not(feature = "serialization"))]
    /// See [PowersAlteration](../power/type.PowersAlteration.html).
    type PowersAlteration: Clone + Debug + Send;
    #[cfg(feature = "serialization")]
    /// See [PowersAlteration](../power/type.PowersAlteration.html).
    type PowersAlteration: Clone + Debug + Send + Serialize + for<'a> Deserialize<'a>;

    /// See [Objectives](type.Objectives.html).
    type Objectives: Default;

    #[cfg(not(feature = "serialization"))]
    /// See [ObjectivesSeed](type.ObjectivesSeed.html).
    type ObjectivesSeed: Clone + Debug + Send;
    #[cfg(feature = "serialization")]
    /// See [ObjectivesSeed](type.ObjectivesSeed.html).
    type ObjectivesSeed: Clone + Debug + Send + Serialize + for<'a> Deserialize<'a>;

    /// Checks if the addition of a new entity in the given team is allowed.
    ///
    /// The provided implementation accepts any new entity.
    fn allow_new_entity(
        &self,
        _state: &BattleState<R>,
        _team: &Team<R>,
        _type: EntityAddition<R>,
    ) -> WeaselResult<(), R> {
        Ok(())
    }

    /// Generates all powers of a team.
    /// Powers should have unique ids, otherwise only the last entry will be persisted.
    ///
    /// The provided implementation generates an empty set of powers.
    fn generate_powers(
        &self,
        _seed: &Option<Self::PowersSeed>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) -> Box<dyn Iterator<Item = Self::Power>> {
        Box::new(std::iter::empty())
    }

    /// Returns `Ok` if `call.team` can invoke `call.power` with `call.invocation`,
    /// otherwise returns an error describing the issue preventing the invocation.\
    /// The power is guaranteed to be known by the team.
    ///
    /// The provided implementation accepts any invocation.
    fn invocable(&self, _state: &BattleState<R>, _call: Call<R>) -> WeaselResult<(), R> {
        Ok(())
    }

    /// Invokes a power.
    /// `call.power` is guaranteed to be known by `call.team`.\
    /// In order to change the state of the world, powers should insert
    /// event prototypes in `event_queue`.
    ///
    /// The provided implementation does nothing.
    fn invoke(
        &self,
        _state: &BattleState<R>,
        _call: Call<R>,
        _event_queue: &mut Option<EventQueue<R>>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) {
    }

    /// Alters one or more powers starting from the given alteration object.
    ///
    /// The provided implementation does nothing.
    fn alter_powers(
        &self,
        _team: &mut Team<R>,
        _alteration: &Self::PowersAlteration,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) {
    }

    /// Generate the objectives for a team.
    ///
    /// The provided implementation returns `Objectives::default()`.\
    /// If you set team `Conclusion` manually, you may avoid implementing this method.
    fn generate_objectives(&self, _seed: &Option<Self::ObjectivesSeed>) -> Self::Objectives {
        Self::Objectives::default()
    }

    /// Checks if the team has completed its objectives.
    /// This check is called after every event.
    ///
    /// The provided implementation does not return any conclusion.\
    /// If you set team `Conclusion` manually, you may avoid implementing this method.
    ///
    /// Returns the `Conclusion` for this team, or none if it did not reach any.
    fn check_objectives_on_event(
        &self,
        _state: &BattleState<R>,
        _team: &Team<R>,
        _metrics: &ReadMetrics<R>,
    ) -> Option<Conclusion> {
        None
    }

    /// Checks if the team has completed its objectives.
    /// This check is called every time a turn ends.
    ///
    /// The provided implementation does not return any conclusion.\
    /// If you set team `Conclusion` manually, you may avoid implementing this method.
    ///
    /// Returns the `Conclusion` for this team, or none if it did not reach any.
    fn check_objectives_on_turn(
        &self,
        _state: &BattleState<R>,
        _team: &Team<R>,
        _metrics: &ReadMetrics<R>,
    ) -> Option<Conclusion> {
        None
    }
}

/// Type to drive the generation of the objectives for a given team.
///
/// For instance, a seed might contain the identifiers of all enemies who must be defeated.
pub type ObjectivesSeed<R> = <<R as BattleRules>::TR as TeamRules<R>>::ObjectivesSeed;

/// Type to store all information about the objectives of a team.
///
/// The objectives can be checked during the battle to know whether or not a team is victorious.
pub type Objectives<R> = <<R as BattleRules>::TR as TeamRules<R>>::Objectives;

/// Describes the different scenarios in which an entity might be added to a team.
pub enum EntityAddition<'a, R: BattleRules> {
    /// Spawn a new creature.
    CreatureSpawn,
    /// Take a creature from another team.
    CreatureConversion(&'a Creature<R>),
}

/// Type to uniquely identify teams.
pub type TeamId<R> = <<R as BattleRules>::TR as TeamRules<R>>::Id;

/// A call is comprised by a team that invokes a power with a given invocation profile.
pub struct Call<'a, R: BattleRules> {
    /// The team that is invoking the power.
    pub team: &'a Team<R>,
    /// The power.
    pub power: &'a Power<R>,
    /// The invocation profile for the power.
    pub invocation: &'a Option<Invocation<R>>,
}

impl<'a, R: BattleRules> Call<'a, R> {
    /// Creates a new call.
    pub fn new(
        team: &'a Team<R>,
        power: &'a Power<R>,
        invocation: &'a Option<Invocation<R>>,
    ) -> Self {
        Self {
            team,
            power,
            invocation,
        }
    }
}

/// Event to create a new team.
///
/// # Examples
/// ```
/// use weasel::{
///     battle_rules, rules::empty::*, Battle, BattleController, BattleRules, CreateTeam,
///     EventTrigger, Server,
/// };
///
/// battle_rules! {}
///
/// let battle = Battle::builder(CustomRules::new()).build();
/// let mut server = Server::builder(battle).build();
///
/// let team_id = 1;
/// CreateTeam::trigger(&mut server, team_id).fire().unwrap();
/// assert_eq!(server.battle().entities().teams().count(), 1);
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct CreateTeam<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "TeamId<R>: Serialize",
            deserialize = "TeamId<R>: Deserialize<'de>"
        ))
    )]
    id: TeamId<R>,

    /// Optional vector containing pairs of teams and relations.
    /// Set `relations` to explicitly set the relation betwen the newly created team
    /// and a list of existing teams.
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Option<Vec<(TeamId<R>, Relation)>>: Serialize",
            deserialize = "Option<Vec<(TeamId<R>, Relation)>>: Deserialize<'de>"
        ))
    )]
    relations: Option<Vec<(TeamId<R>, Relation)>>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Option<PowersSeed<R>>: Serialize",
            deserialize = "Option<PowersSeed<R>>: Deserialize<'de>"
        ))
    )]
    powers_seed: Option<PowersSeed<R>>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Option<ObjectivesSeed<R>>: Serialize",
            deserialize = "Option<ObjectivesSeed<R>>: Deserialize<'de>"
        ))
    )]
    objectives_seed: Option<ObjectivesSeed<R>>,
}

impl<R: BattleRules> Debug for CreateTeam<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "CreateTeam {{ id: {:?}, relations: {:?}, objectives_seed: {:?} }}",
            self.id, self.relations, self.objectives_seed
        )
    }
}

impl<R: BattleRules> Clone for CreateTeam<R> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            relations: self.relations.clone(),
            powers_seed: self.powers_seed.clone(),
            objectives_seed: self.objectives_seed.clone(),
        }
    }
}

impl<R: BattleRules> CreateTeam<R> {
    /// Returns a trigger for this event.
    pub fn trigger<'a, P: EventProcessor<R>>(
        processor: &'a mut P,
        id: TeamId<R>,
    ) -> CreateTeamTrigger<'a, R, P> {
        CreateTeamTrigger {
            processor,
            id,
            relations: None,
            powers_seed: None,
            objectives_seed: None,
        }
    }

    /// Returns the team id.
    pub fn id(&self) -> &TeamId<R> {
        &self.id
    }

    /// Returns the optional relations for the new team.
    pub fn relations(&self) -> &Option<Vec<(TeamId<R>, Relation)>> {
        &self.relations
    }

    /// Returns the seed to generate the team's powers.
    pub fn powers_seed(&self) -> &Option<PowersSeed<R>> {
        &self.powers_seed
    }

    /// Returns the seed to generate the team's objectives.
    pub fn objectives_seed(&self) -> &Option<ObjectivesSeed<R>> {
        &self.objectives_seed
    }
}

impl<R: BattleRules + 'static> Event<R> for CreateTeam<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        // New team must not already exist.
        if battle.entities().team(&self.id).is_some() {
            return Err(WeaselError::DuplicatedTeam(self.id.clone()));
        }
        if let Some(relations) = &self.relations {
            for (team_id, relation) in relations {
                // Prevent self relation assignment.
                if *team_id == self.id {
                    return Err(WeaselError::SelfRelation);
                }
                // Prevent explicit kinship.
                if *relation == Relation::Kin {
                    return Err(WeaselError::KinshipRelation);
                }
                // Teams in the relations list must exist.
                if battle.entities().team(&team_id).is_none() {
                    return Err(WeaselError::TeamNotFound(team_id.clone()));
                }
            }
        }
        Ok(())
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        // Powers' generation is influenced by the given powers_seed, if present.
        let it = battle.rules.team_rules().generate_powers(
            &self.powers_seed,
            &mut battle.entropy,
            &mut battle.metrics.write_handle(),
        );
        let powers = collect_from_iter(it);
        // Insert the new team.
        battle.state.entities.add_team(Team {
            id: self.id.clone(),
            creatures: Vec::new(),
            powers,
            conclusion: None,
            objectives: battle
                .rules
                .team_rules()
                .generate_objectives(&self.objectives_seed),
        });
        // Unpack explicit relations into a vector.
        let mut relations = if let Some(relations) = &self.relations {
            relations
                .iter()
                .map(|e| (RelationshipPair::new(self.id.clone(), e.0.clone()), e.1))
                .collect()
        } else {
            Vec::new()
        };
        // Set to `Relation::Enemy` all relations to other teams not explicitly set.
        for team_id in battle.entities().teams().map(|e| e.id()).filter(|e| {
            **e != self.id
                && self
                    .relations
                    .as_ref()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .find(|(id, _)| *id == **e)
                    .is_none()
        }) {
            relations.push((
                RelationshipPair::new(self.id.clone(), team_id.clone()),
                Relation::Enemy,
            ));
        }
        // Insert the new relations.
        battle.state.entities.update_relations(relations);
        // Update metrics.
        battle
            .metrics
            .write_handle()
            .add_system_u64(TEAMS_CREATED, 1)
            .unwrap_or_else(|err| panic!("constraint violated: {:?}", err));
    }

    fn kind(&self) -> EventKind {
        EventKind::CreateTeam
    }

    fn box_clone(&self) -> Box<dyn Event<R> + Send> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `CreateTeam` event.
pub struct CreateTeamTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: TeamId<R>,
    relations: Option<Vec<(TeamId<R>, Relation)>>,
    powers_seed: Option<PowersSeed<R>>,
    objectives_seed: Option<ObjectivesSeed<R>>,
}

impl<'a, R, P> CreateTeamTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    /// Adds a list of relationships between this team and other existing teams.
    pub fn relations(&'a mut self, relations: &[(TeamId<R>, Relation)]) -> &'a mut Self {
        self.relations = Some(relations.into());
        self
    }

    /// Adds a seed to drive the generation of this team powers.
    pub fn powers_seed(&'a mut self, seed: PowersSeed<R>) -> &'a mut Self {
        self.powers_seed = Some(seed);
        self
    }

    /// Adds a seed to drive the generation of this team objectives.
    pub fn objectives_seed(&'a mut self, seed: ObjectivesSeed<R>) -> &'a mut Self {
        self.objectives_seed = Some(seed);
        self
    }
}

impl<'a, R, P> EventTrigger<'a, R, P> for CreateTeamTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `CreateTeam` event.
    fn event(&self) -> Box<dyn Event<R> + Send> {
        Box::new(CreateTeam {
            id: self.id.clone(),
            relations: self.relations.clone(),
            powers_seed: self.powers_seed.clone(),
            objectives_seed: self.objectives_seed.clone(),
        })
    }
}

/// All possible kinds of relation between teams and thus entities.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum Relation {
    /// Represents an alliance.
    Ally,
    /// Represents enmity.
    Enemy,
    /// Reserved for entities in the same team.
    Kin,
}

/// A pair of two teams that are part of a relationship.
#[derive(Clone)]
pub(crate) struct RelationshipPair<R: BattleRules> {
    pub(crate) first: TeamId<R>,
    pub(crate) second: TeamId<R>,
}

impl<R: BattleRules> Debug for RelationshipPair<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "RelationshipPair {{ first: {:?}, second: {:?} }}",
            self.first, self.second
        )
    }
}

impl<R: BattleRules> RelationshipPair<R> {
    pub(crate) fn new(first: TeamId<R>, second: TeamId<R>) -> Self {
        Self { first, second }
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = TeamId<R>> {
        let first = iter::once(self.first.clone());
        let second = iter::once(self.second.clone());
        first.chain(second)
    }
}

impl<R: BattleRules> PartialEq for RelationshipPair<R> {
    fn eq(&self, other: &Self) -> bool {
        (self.first == other.first && self.second == other.second)
            || (self.first == other.second && self.second == other.first)
    }
}

impl<R: BattleRules> Eq for RelationshipPair<R> {}

impl<R: BattleRules> Hash for RelationshipPair<R> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if self.first > self.second {
            self.first.hash(state);
            self.second.hash(state);
        } else {
            self.second.hash(state);
            self.first.hash(state);
        }
    }
}

/// Event to set diplomatic relations between teams.
/// Relations are symmetric.
///
/// # Examples
/// ```
/// use weasel::{
///     battle_rules, rules::empty::*, Battle, BattleController, BattleRules, CreateTeam,
///     EventTrigger, Relation, Server, SetRelations,
/// };
///
/// battle_rules! {}
///
/// let battle = Battle::builder(CustomRules::new()).build();
/// let mut server = Server::builder(battle).build();
///
/// let team_blue_id = 1;
/// let team_red_id = 2;
/// CreateTeam::trigger(&mut server, team_blue_id).fire().unwrap();
/// CreateTeam::trigger(&mut server, team_red_id).fire().unwrap();
///
/// SetRelations::trigger(&mut server, &[(team_blue_id, team_red_id, Relation::Ally)])
///     .fire()
///     .unwrap();
/// assert_eq!(
///     server.battle().entities().relation(&team_blue_id, &team_red_id),
///     Some(Relation::Ally)
/// );
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct SetRelations<R: BattleRules> {
    /// Vector containing tuples of two teams and a relation.
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Vec<(TeamId<R>, TeamId<R>, Relation)>: Serialize",
            deserialize = "Vec<(TeamId<R>, TeamId<R>, Relation)>: Deserialize<'de>"
        ))
    )]
    relations: Vec<(TeamId<R>, TeamId<R>, Relation)>,
}

impl<R: BattleRules> Debug for SetRelations<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "SetRelations {{ relations: {:?} }}", self.relations)
    }
}

impl<R: BattleRules> Clone for SetRelations<R> {
    fn clone(&self) -> Self {
        Self {
            relations: self.relations.clone(),
        }
    }
}

impl<R: BattleRules> SetRelations<R> {
    /// Returns a trigger for this event.
    pub fn trigger<'a, P: EventProcessor<R>>(
        processor: &'a mut P,
        relations: &[(TeamId<R>, TeamId<R>, Relation)],
    ) -> SetRelationsTrigger<'a, R, P> {
        SetRelationsTrigger {
            processor,
            relations: relations.into(),
        }
    }

    /// Returns all relation changes.
    pub fn relations(&self) -> &Vec<(TeamId<R>, TeamId<R>, Relation)> {
        &self.relations
    }
}

impl<R: BattleRules + 'static> Event<R> for SetRelations<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        for (first, second, relation) in &self.relations {
            // Prevent self relation assignment.
            if *first == *second {
                return Err(WeaselError::SelfRelation);
            }
            // Prevent explicit kinship.
            if *relation == Relation::Kin {
                return Err(WeaselError::KinshipRelation);
            }
            // Teams in the relations list must exist.
            if battle.entities().team(first).is_none() {
                return Err(WeaselError::TeamNotFound(first.clone()));
            }
            if battle.entities().team(second).is_none() {
                return Err(WeaselError::TeamNotFound(second.clone()));
            }
        }
        Ok(())
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        // Insert the new relations.
        let vec = self
            .relations
            .iter()
            .map(|e| (RelationshipPair::new(e.0.clone(), e.1.clone()), e.2))
            .collect();
        battle.state.entities.update_relations(vec);
    }

    fn kind(&self) -> EventKind {
        EventKind::SetRelations
    }

    fn box_clone(&self) -> Box<dyn Event<R> + Send> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `SetRelations` event.
pub struct SetRelationsTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    relations: Vec<(TeamId<R>, TeamId<R>, Relation)>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for SetRelationsTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `SetRelations` event.
    fn event(&self) -> Box<dyn Event<R> + Send> {
        Box::new(SetRelations {
            relations: self.relations.clone(),
        })
    }
}

/// All possible conclusions for a team's objectives.
/// In other words, this tells if the team reached its objectives or failed.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum Conclusion {
    /// Team achieved its objectives.
    Victory,
    /// Team failed to achieve its objectives.
    Defeat,
}

/// Event to set the `Conclusion` of a team.
///
/// # Examples
/// ```
/// use weasel::{
///     battle_rules, rules::empty::*, Battle, BattleController, BattleRules, ConcludeObjectives,
///     Conclusion, CreateTeam, EventTrigger, Server,
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
/// ConcludeObjectives::trigger(&mut server, team_id, Conclusion::Victory)
///     .fire()
///     .unwrap();
/// assert_eq!(
///     server.battle().entities().team(&team_id).unwrap().conclusion(),
///     Some(Conclusion::Victory)
/// );
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ConcludeObjectives<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "TeamId<R>: Serialize",
            deserialize = "TeamId<R>: Deserialize<'de>"
        ))
    )]
    id: TeamId<R>,

    conclusion: Conclusion,
}

impl<R: BattleRules> Debug for ConcludeObjectives<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "ConcludeObjectives {{ id: {:?}, conclusion: {:?} }}",
            self.id, self.conclusion
        )
    }
}

impl<R: BattleRules> Clone for ConcludeObjectives<R> {
    fn clone(&self) -> Self {
        ConcludeObjectives {
            id: self.id.clone(),
            conclusion: self.conclusion,
        }
    }
}

impl<R: BattleRules> ConcludeObjectives<R> {
    /// Returns a trigger for this event.
    pub fn trigger<'a, P: EventProcessor<R>>(
        processor: &'a mut P,
        id: TeamId<R>,
        conclusion: Conclusion,
    ) -> ConcludeObjectivesTrigger<'a, R, P> {
        ConcludeObjectivesTrigger {
            processor,
            id,
            conclusion,
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for ConcludeObjectives<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        // Team must exist.
        if battle.entities().team(&self.id).is_none() {
            return Err(WeaselError::TeamNotFound(self.id.clone()));
        }
        Ok(())
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        // Change the team's conclusion.
        let team = battle
            .state
            .entities
            .team_mut(&self.id)
            .unwrap_or_else(|| panic!("constraint violated: team {:?} not found", self.id));
        team.conclusion = Some(self.conclusion);
    }

    fn kind(&self) -> EventKind {
        EventKind::ConcludeObjectives
    }

    fn box_clone(&self) -> Box<dyn Event<R> + Send> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `ConcludeObjectives` event.
pub struct ConcludeObjectivesTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: TeamId<R>,
    conclusion: Conclusion,
}

impl<'a, R, P> EventTrigger<'a, R, P> for ConcludeObjectivesTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `ConcludeObjectives` event.
    fn event(&self) -> Box<dyn Event<R> + Send> {
        Box::new(ConcludeObjectives {
            id: self.id.clone(),
            conclusion: self.conclusion,
        })
    }
}

/// Event to reset a team's objectives.
/// Team's `Conclusion` is resetted as well since the objectives changed.
///
/// # Examples
/// ```
/// use weasel::{
///     battle_rules, rules::empty::*, Battle, BattleController, BattleRules, ConcludeObjectives,
///     Conclusion, CreateTeam, EventTrigger, ResetObjectives, Server,
/// };
///
/// battle_rules! {}
///
/// let battle = Battle::builder(CustomRules::new()).build();
/// let mut server = Server::builder(battle).build();
///
/// let team_id = 1;
/// CreateTeam::trigger(&mut server, team_id).fire().unwrap();
/// ConcludeObjectives::trigger(&mut server, team_id, Conclusion::Victory)
///     .fire()
///     .unwrap();
///
/// ResetObjectives::trigger(&mut server, team_id).fire().unwrap();
/// assert_eq!(
///     server.battle().entities().team(&team_id).unwrap().conclusion(),
///     None
/// );
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ResetObjectives<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "TeamId<R>: Serialize",
            deserialize = "TeamId<R>: Deserialize<'de>"
        ))
    )]
    id: TeamId<R>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Option<ObjectivesSeed<R>>: Serialize",
            deserialize = "Option<ObjectivesSeed<R>>: Deserialize<'de>"
        ))
    )]
    seed: Option<ObjectivesSeed<R>>,
}

impl<R: BattleRules> ResetObjectives<R> {
    /// Returns a trigger for this event.
    pub fn trigger<P: EventProcessor<R>>(
        processor: &mut P,
        id: TeamId<R>,
    ) -> ResetObjectivesTrigger<R, P> {
        ResetObjectivesTrigger {
            processor,
            id,
            seed: None,
        }
    }

    /// Returns the team id.
    pub fn id(&self) -> &TeamId<R> {
        &self.id
    }

    /// Returns the new seed.
    pub fn seed(&self) -> &Option<ObjectivesSeed<R>> {
        &self.seed
    }
}

impl<R: BattleRules> Debug for ResetObjectives<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "ResetObjectives {{ id: {:?}, seed: {:?} }}",
            self.id, self.seed
        )
    }
}

impl<R: BattleRules> Clone for ResetObjectives<R> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            seed: self.seed.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for ResetObjectives<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        // Team must exist.
        if battle.entities().team(&self.id).is_none() {
            return Err(WeaselError::TeamNotFound(self.id.clone()));
        }
        Ok(())
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        // Regenerate the team's objectives.
        let team = battle
            .state
            .entities
            .team_mut(&self.id)
            .unwrap_or_else(|| panic!("constraint violated: team {:?} not found", self.id));
        team.objectives = battle.rules.team_rules().generate_objectives(&self.seed);
        // Reset the team's conclusion.
        team.conclusion = None;
    }

    fn kind(&self) -> EventKind {
        EventKind::ResetObjectives
    }

    fn box_clone(&self) -> Box<dyn Event<R> + Send> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `ResetObjectives` event.
pub struct ResetObjectivesTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: TeamId<R>,
    seed: Option<ObjectivesSeed<R>>,
}

impl<'a, R, P> ResetObjectivesTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    /// Adds a seed to drive the generation of the new objectives.
    pub fn seed(&'a mut self, seed: ObjectivesSeed<R>) -> &'a mut Self {
        self.seed = Some(seed);
        self
    }
}

impl<'a, R, P> EventTrigger<'a, R, P> for ResetObjectivesTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `ResetObjectives` event.
    fn event(&self) -> Box<dyn Event<R> + Send> {
        Box::new(ResetObjectives {
            id: self.id.clone(),
            seed: self.seed.clone(),
        })
    }
}

/// Event to remove a team from a battle.
/// Teams can be removed only if they are empty.
///
/// # Examples
/// ```
/// use weasel::{
///     battle_rules, rules::empty::*, Battle, BattleController, BattleRules, CreateTeam,
///     EventTrigger, RemoveTeam, Server,
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
/// RemoveTeam::trigger(&mut server, team_id).fire().unwrap();
/// assert_eq!(server.battle().entities().teams().count(), 0);
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct RemoveTeam<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "TeamId<R>: Serialize",
            deserialize = "TeamId<R>: Deserialize<'de>"
        ))
    )]
    id: TeamId<R>,
}

impl<R: BattleRules> RemoveTeam<R> {
    /// Returns a trigger for this event.
    pub fn trigger<P: EventProcessor<R>>(
        processor: &mut P,
        id: TeamId<R>,
    ) -> RemoveTeamTrigger<R, P> {
        RemoveTeamTrigger { processor, id }
    }

    /// Returns the id of the team to be removed.
    pub fn id(&self) -> &TeamId<R> {
        &self.id
    }
}

impl<R: BattleRules> Debug for RemoveTeam<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "RemoveTeam {{ id: {:?} }}", self.id)
    }
}

impl<R: BattleRules> Clone for RemoveTeam<R> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for RemoveTeam<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        // Team must exist.
        if let Some(team) = battle.entities().team(&self.id) {
            // Team must not have any creature.
            if team.creatures().peekable().peek().is_some() {
                return Err(WeaselError::TeamNotEmpty(self.id.clone()));
            }
            Ok(())
        } else {
            Err(WeaselError::TeamNotFound(self.id.clone()))
        }
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        // Remove the team.
        battle
            .state
            .entities
            .remove_team(&self.id)
            .unwrap_or_else(|err| panic!("constraint violated: {:?}", err));
        // Remove rights of players towards this team.
        battle.rights_mut().remove_team(&self.id);
    }

    fn kind(&self) -> EventKind {
        EventKind::RemoveTeam
    }

    fn box_clone(&self) -> Box<dyn Event<R> + Send> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `RemoveTeam` event.
pub struct RemoveTeamTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: TeamId<R>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for RemoveTeamTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `RemoveTeam` event.
    fn event(&self) -> Box<dyn Event<R> + Send> {
        Box::new(RemoveTeam {
            id: self.id.clone(),
        })
    }
}

/// An event to alter the powers of a team.
///
/// # Examples
/// ```
/// use weasel::{
///     battle_rules, rules::empty::*, AlterPowers, Battle, BattleController, BattleRules,
///     CreateTeam, EventKind, EventTrigger, Server,
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
/// let alteration = ();
/// AlterPowers::trigger(&mut server, team_id, alteration)
///     .fire()
///     .unwrap();
/// assert_eq!(
///     server.battle().history().events().iter().last().unwrap().kind(),
///     EventKind::AlterPowers
/// );
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct AlterPowers<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "TeamId<R>: Serialize",
            deserialize = "TeamId<R>: Deserialize<'de>"
        ))
    )]
    id: TeamId<R>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "PowersAlteration<R>: Serialize",
            deserialize = "PowersAlteration<R>: Deserialize<'de>"
        ))
    )]
    alteration: PowersAlteration<R>,
}

impl<R: BattleRules> AlterPowers<R> {
    /// Returns a trigger for this event.
    pub fn trigger<'a, P: EventProcessor<R>>(
        processor: &'a mut P,
        id: TeamId<R>,
        alteration: PowersAlteration<R>,
    ) -> AlterPowersTrigger<'a, R, P> {
        AlterPowersTrigger {
            processor,
            id,
            alteration,
        }
    }

    /// Returns the team id.
    pub fn id(&self) -> &TeamId<R> {
        &self.id
    }

    /// Returns the definition of the changes to the team's powers.
    pub fn alteration(&self) -> &PowersAlteration<R> {
        &self.alteration
    }
}

impl<R: BattleRules> Debug for AlterPowers<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "AlterPowers {{ id: {:?}, alteration: {:?} }}",
            self.id, self.alteration
        )
    }
}

impl<R: BattleRules> Clone for AlterPowers<R> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            alteration: self.alteration.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for AlterPowers<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        // Team must exist.
        if battle.entities().team(&self.id).is_some() {
            Ok(())
        } else {
            Err(WeaselError::TeamNotFound(self.id.clone()))
        }
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        // Retrieve the team.
        let team = battle
            .state
            .entities
            .team_mut(&self.id)
            .unwrap_or_else(|| panic!("constraint violated: team {:?} not found", self.id));
        // Alter the team.
        battle.rules.team_rules().alter_powers(
            team,
            &self.alteration,
            &mut battle.entropy,
            &mut battle.metrics.write_handle(),
        );
    }

    fn kind(&self) -> EventKind {
        EventKind::AlterPowers
    }

    fn box_clone(&self) -> Box<dyn Event<R> + Send> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire an `AlterPowers` event.
pub struct AlterPowersTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: TeamId<R>,
    alteration: PowersAlteration<R>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for AlterPowersTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns an `AlterPowers` event.
    fn event(&self) -> Box<dyn Event<R> + Send> {
        Box::new(AlterPowers {
            id: self.id.clone(),
            alteration: self.alteration.clone(),
        })
    }
}

/// An event to regenerate the powers of a team.
///
/// A new set of powers is created from a seed.\
/// - Powers already present in the team won't be modified.
/// - Powers that the team didn't have before will be added.
/// - Current team's powers that are not present in the new set will be removed
///   from the team.
///
/// # Examples
/// ```
/// use weasel::{
///     battle_rules, rules::empty::*, Battle, BattleController, BattleRules, CreateTeam,
///     EventKind, EventTrigger, RegeneratePowers, Server,
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
/// RegeneratePowers::trigger(&mut server, team_id)
///     .fire()
///     .unwrap();
/// assert_eq!(
///     server.battle().history().events().iter().last().unwrap().kind(),
///     EventKind::RegeneratePowers
/// );
/// ```
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct RegeneratePowers<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "TeamId<R>: Serialize",
            deserialize = "TeamId<R>: Deserialize<'de>"
        ))
    )]
    id: TeamId<R>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Option<PowersSeed<R>>: Serialize",
            deserialize = "Option<PowersSeed<R>>: Deserialize<'de>"
        ))
    )]
    seed: Option<PowersSeed<R>>,
}

impl<R: BattleRules> RegeneratePowers<R> {
    /// Returns a trigger for this event.
    pub fn trigger<P: EventProcessor<R>>(
        processor: &'_ mut P,
        id: TeamId<R>,
    ) -> RegeneratePowersTrigger<'_, R, P> {
        RegeneratePowersTrigger {
            processor,
            id,
            seed: None,
        }
    }

    /// Returns the team id.
    pub fn id(&self) -> &TeamId<R> {
        &self.id
    }

    /// Returns the seed to regenerate the team's powers.
    pub fn seed(&self) -> &Option<PowersSeed<R>> {
        &self.seed
    }
}

impl<R: BattleRules> Debug for RegeneratePowers<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "RegeneratePowers {{ id: {:?}, seed: {:?} }}",
            self.id, self.seed
        )
    }
}

impl<R: BattleRules> Clone for RegeneratePowers<R> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            seed: self.seed.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for RegeneratePowers<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        // Team must exist.
        if battle.entities().team(&self.id).is_some() {
            Ok(())
        } else {
            Err(WeaselError::TeamNotFound(self.id.clone()))
        }
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        // Retrieve the team.
        let team = battle
            .state
            .entities
            .team_mut(&self.id)
            .unwrap_or_else(|| panic!("constraint violated: team {:?} not found", self.id));
        // Generate a new set of powers.
        let powers: Vec<_> = battle
            .rules
            .team_rules()
            .generate_powers(
                &self.seed,
                &mut battle.entropy,
                &mut battle.metrics.write_handle(),
            )
            .collect();
        let mut to_remove = Vec::new();
        // Remove all team's powers not present in the new set.
        for power in team.powers() {
            if powers.iter().find(|e| e.id() == power.id()).is_none() {
                to_remove.push(power.id().clone());
            }
        }
        for power_id in to_remove {
            team.remove_power(&power_id);
        }
        // Add all powers present in the new set but not in the team.
        for power in powers {
            if team.power(power.id()).is_none() {
                team.add_power(power);
            }
        }
    }

    fn kind(&self) -> EventKind {
        EventKind::RegeneratePowers
    }

    fn box_clone(&self) -> Box<dyn Event<R> + Send> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `RegeneratePowers` event.
pub struct RegeneratePowersTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: TeamId<R>,
    seed: Option<PowersSeed<R>>,
}

impl<'a, R, P> RegeneratePowersTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    /// Adds a seed to drive the regeneration of this team's powers.
    pub fn seed(&'a mut self, seed: PowersSeed<R>) -> &'a mut Self {
        self.seed = Some(seed);
        self
    }
}

impl<'a, R, P> EventTrigger<'a, R, P> for RegeneratePowersTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `RegeneratePowers` event.
    fn event(&self) -> Box<dyn Event<R> + Send> {
        Box::new(RegeneratePowers {
            id: self.id.clone(),
            seed: self.seed.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::statistic::SimpleStatistic;
    use crate::util::tests::{server, team};
    use crate::{battle_rules, battle_rules_with_team, rules::empty::*};
    use std::collections::hash_map::DefaultHasher;

    fn get_hash<T: Hash>(item: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn relationship_hash_eq() {
        battle_rules! {}
        let r11 = RelationshipPair::<CustomRules>::new(1, 1);
        let r12 = RelationshipPair::<CustomRules>::new(1, 2);
        let r21 = RelationshipPair::<CustomRules>::new(2, 1);
        assert_eq!(r11, r11);
        assert_eq!(r12, r21);
        assert_ne!(r11, r12);
        assert_eq!(get_hash(&r11), get_hash(&r11));
        assert_eq!(get_hash(&r12), get_hash(&r21));
        assert_ne!(get_hash(&r11), get_hash(&r12));
    }

    #[derive(Default)]
    pub struct CustomTeamRules {}

    impl<R: BattleRules> TeamRules<R> for CustomTeamRules {
        type Id = u32;
        type Power = SimpleStatistic<u32, u32>;
        type PowersSeed = ();
        type Invocation = ();
        type PowersAlteration = ();
        type ObjectivesSeed = ();
        type Objectives = ();
    }

    #[test]
    fn mutable_powers() {
        battle_rules_with_team! { CustomTeamRules }
        // Create a battle.
        let mut server = server(CustomRules::new());
        team(&mut server, 1);
        let team = server.battle.state.entities.team_mut(&1).unwrap();
        assert!(team.power(&1).is_none());
        team.add_power(SimpleStatistic::new(1, 50));
        assert!(team.power(&1).is_some());
        team.power_mut(&1).unwrap().set_value(25);
        assert_eq!(team.power(&1).unwrap().value(), 25);
        team.powers_mut().last().unwrap().set_value(30);
        assert_eq!(team.power(&1).unwrap().value(), 30);
        team.remove_power(&1);
        assert!(team.power(&1).is_none());
    }
}
