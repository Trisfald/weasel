use std::fmt::{Display, Formatter, Result};
use weasel::actor::Actor;
use weasel::battle::BattleRules;
use weasel::character::{CharacterRules, StatisticId};
use weasel::entity::{Entities, EntityId};
use weasel::entropy::Entropy;
use weasel::metric::WriteMetrics;
use weasel::round::RoundsRules;
use weasel::rules::entropy::UniformDistribution;
use weasel::rules::statistic::SimpleStatistic;
use weasel::space::Space;
use weasel::{battle_rules, rules::empty::*};

static SPEED: StatisticId<CustomRules> = 0;

// Declare the battle rules with the help of a macro.
battle_rules! {
    EmptyTeamRules,
    // We must provide our own character rules to define creatures' statistics.
    CustomCharacterRules,
    EmptyActorRules,
    EmptyFightRules,
    EmptyUserRules,
    EmptySpaceRules,
    // Our own rules to decide the acting order.
    CustomRoundsRules,
    // We want entropy rules that can randomize the initiative score.
    UniformDistribution<u32>
}

// Define our custom character rules.
#[derive(Default)]
pub struct CustomCharacterRules {}

impl CharacterRules<CustomRules> for CustomCharacterRules {
    // Just use an integer as creature id.
    type CreatureId = u8;
    // Use statistics with integers as both id and value.
    type Statistic = SimpleStatistic<u8, u16>;
    // The seed will contain the value of speed.
    type StatisticsSeed = u16;
    // We never alter statistics in this example.
    type StatisticsAlteration = ();

    fn generate_statistics(
        &self,
        seed: &Option<Self::StatisticsSeed>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> Box<dyn Iterator<Item = Self::Statistic>> {
        // Generate a single statistic: speed.
        let v = vec![SimpleStatistic::new(SPEED, seed.unwrap())];
        Box::new(v.into_iter())
    }
}

// Define our custom rounds rules.
#[derive(Default)]
pub struct CustomRoundsRules {}

impl RoundsRules<CustomRules> for CustomRoundsRules {
    // No need for a seed. We always use a fresh model at the start of the battle.
    type RoundsSeed = ();
    // The model is a struct to hold the initiative of all actors.
    type RoundsModel = InitiativeModel;

    fn generate_model(&self, _: &Option<Self::RoundsSeed>) -> Self::RoundsModel {
        // Return a default model.
        InitiativeModel::default()
    }

    fn eligible(&self, model: &Self::RoundsModel, actor: &dyn Actor<CustomRules>) -> bool {
        // An actor can act only if he's at the top of the initiative ranking.
        if model.actors.is_empty() {
            false
        } else {
            model.actors[0].0 == *actor.entity_id()
        }
    }

    fn on_end(
        &self,
        entities: &Entities<CustomRules>,
        _: &Space<CustomRules>,
        model: &mut Self::RoundsModel,
        actor: &dyn Actor<CustomRules>,
        entropy: &mut Entropy<CustomRules>,
        _: &mut WriteMetrics<CustomRules>,
    ) {
        // We add speed +- 25% to all actors's initiative.
        for actor in entities.actors() {
            let speed: f64 = actor.statistic(&SPEED).unwrap().value().into();
            model.update(
                actor,
                entropy.generate(
                    (speed - (speed * 0.25)) as u32,
                    (speed + (speed * 0.25)) as u32,
                ),
            );
        }
        // We set the initiative of the outgoing actor to 0.
        model.reset(actor);
        // Now sort the initiative of all actors.
        model.sort();
    }

    fn on_actor_added(
        &self,
        model: &mut Self::RoundsModel,
        actor: &dyn Actor<CustomRules>,
        _: &mut Entropy<CustomRules>,
        _: &mut WriteMetrics<CustomRules>,
    ) {
        // When a new actor is added we simply insert him in the initiative table.
        model.insert(actor);
    }

    fn on_actor_removed(
        &self,
        model: &mut Self::RoundsModel,
        actor: &dyn Actor<CustomRules>,
        _: &mut Entropy<CustomRules>,
        _: &mut WriteMetrics<CustomRules>,
    ) {
        // Remove the actor from the model.
        model.remove(actor);
    }
}

#[derive(Default)]
pub(crate) struct InitiativeModel {
    // A vector where we store all actors with their current initiative score.
    actors: Vec<(EntityId<CustomRules>, u32)>,
}

impl InitiativeModel {
    /// Sorts the actors by their initiative score (descending).
    fn sort(&mut self) {
        self.actors.sort_by(|lhs, rhs| rhs.1.cmp(&lhs.1));
    }

    fn insert(&mut self, actor: &dyn Actor<CustomRules>) {
        // Insert the actor with an initial score equal to his speed.
        self.actors.push((
            actor.entity_id().clone(),
            actor.statistic(&SPEED).unwrap().value().into(),
        ));
        // Sort the actors.
        self.sort();
    }

    /// Returns the actor with the highest score of initiative.
    pub(crate) fn top(&self) -> EntityId<CustomRules> {
        self.actors[0].0.clone()
    }

    /// Sets the initiative score of the given actor to 0.
    fn reset(&mut self, actor: &dyn Actor<CustomRules>) {
        if let Some(index) = self.actor_index(actor) {
            self.actors[index].1 = 0;
        }
    }

    /// Adds `value` to the initiative of `actor`.
    fn update(&mut self, actor: &dyn Actor<CustomRules>, value: u32) {
        if let Some(index) = self.actor_index(actor) {
            self.actors[index].1 += value;
        }
    }

    /// Removes the given actor.
    fn remove(&mut self, actor: &dyn Actor<CustomRules>) {
        if let Some(index) = self.actor_index(actor) {
            self.actors.remove(index);
        }
    }

    fn actor_index(&self, actor: &dyn Actor<CustomRules>) -> Option<usize> {
        self.actors
            .iter()
            .position(|(actor_id, _)| actor_id == actor.entity_id())
    }
}

impl Display for InitiativeModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        // Print a table with all actors and their initiative score.
        writeln!(f, "Actor              Score")?;
        for (actor_id, score) in &self.actors {
            write!(f, "{}       {}\n", actor_id, score)?;
        }
        Ok(())
    }
}
