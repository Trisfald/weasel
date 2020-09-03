use weasel::ability::AbilityId;
use weasel::character::StatisticId;
use weasel::rules::{ability::SimpleAbility, statistic::SimpleStatistic};
use weasel::{
    battle_rules, rules::empty::*, Action, Actor, ActorRules, BattleRules, BattleState,
    CharacterRules, Entities, EntityId, Entropy, EventQueue, EventTrigger, MoveEntity,
    PositionClaim, RoundsRules, Space, SpaceRules, TeamRules, WeaselError, WeaselResult,
    WriteMetrics,
};

pub(crate) static CARD_VALUE_STAT: StatisticId<CustomRules> = 0;
pub(crate) static PLAY_CARD_ABILITY: AbilityId<CustomRules> = 0;

// Define our custom character rules.
// Since this's a card game, players will just handle creatures (the 'cards').
#[derive(Default)]
pub struct MyCharacterRules {}

impl CharacterRules<CustomRules> for MyCharacterRules {
    // Id for cards.
    type CreatureId = u8;
    // No objects in this game
    type ObjectId = ();
    // Our cards have just a statistic that tells what the card's value is.
    // We use the `SimpleStatistic` type from weasel to avoid implementing our own.
    type Statistic = SimpleStatistic<u8, u8>;
    // The seed is equal to the card value.
    type StatisticsSeed = u8;
    // A card value is immutable.
    type StatisticsAlteration = ();
    // This game doesn't have long lasting status effects.
    type Status = EmptyStatus;
    type StatusesAlteration = ();

    // In this method we generate statistics of cards.
    fn generate_statistics(
        &self,
        seed: &Option<Self::StatisticsSeed>,
        _: &mut Entropy<CustomRules>,
        _: &mut WriteMetrics<CustomRules>,
    ) -> Box<dyn Iterator<Item = Self::Statistic>> {
        let value = seed.unwrap();
        // Generate one statistic with the card value.
        let v = vec![SimpleStatistic::new(CARD_VALUE_STAT, value)];
        Box::new(v.into_iter())
    }
}

// Define our custom team rules. A team is equivalent to a player.
#[derive(Default)]
pub struct MyTeamRules {}

impl TeamRules<CustomRules> for MyTeamRules {
    type Id = u8;
    // Teams don't have powers in this example.
    type Power = EmptyPower;
    type PowersSeed = ();
    type PowersAlteration = ();
    // How many turns a team has won.
    type ObjectivesSeed = u8;
    // Our objective is to win 'turns', so a simple counter will suffice.
    type Objectives = u8;

    fn generate_objectives(&self, seed: &Option<Self::ObjectivesSeed>) -> Self::Objectives {
        seed.unwrap_or_default()
    }
}

// We define the round rules to impose an ordering to player's moves.
#[derive(Default)]
pub struct MyRoundsRules {}

impl RoundsRules<CustomRules> for MyRoundsRules {
    // No seed. Rounds' ordering is static.
    type RoundsSeed = ();
    // The model is just a counter.
    type RoundsModel = u8;

    fn generate_model(&self, _: &Option<Self::RoundsSeed>) -> Self::RoundsModel {
        // The first player to move will be the one at index 0.
        0
    }

    fn eligible(&self, model: &Self::RoundsModel, actor: &dyn Actor<CustomRules>) -> bool {
        // A card can be played only if it belongs to the right team.
        actor.team_id() == model
    }

    fn on_end(
        &self,
        _: &Entities<CustomRules>,
        _: &Space<CustomRules>,
        model: &mut Self::RoundsModel,
        _: &dyn Actor<CustomRules>,
        _: &mut Entropy<CustomRules>,
        _: &mut WriteMetrics<CustomRules>,
    ) {
        // When a player turn ends bump the counter, wrapping at 3
        // so that it cycles between 0, 1 and 2.
        *model = (*model + 1) % 3;
    }
}

// Space rules in this case define the position of the cards during the game.
// The game is played by moving cards around, after all!
// In summary, a card can be either in a player's hand or on the table.
#[derive(Default)]
pub struct MySpaceRules {}

impl SpaceRules<CustomRules> for MySpaceRules {
    // false: in hand, true: on the table.
    type Position = bool;
    type SpaceSeed = ();
    // Array with the id of cards on the table.
    type SpaceModel = [Option<EntityId<CustomRules>>; 3];
    type SpaceAlteration = ();

    fn generate_model(&self, _seed: &Option<Self::SpaceSeed>) -> Self::SpaceModel {
        // At the start the table is empty.
        [None, None, None]
    }

    fn check_move(
        &self,
        model: &Self::SpaceModel,
        _claim: PositionClaim<CustomRules>,
        position: &Self::Position,
    ) -> WeaselResult<(), CustomRules> {
        // We can play a card only if the table isn't full.
        if *position {
            if model.iter().any(|e| e.is_none()) {
                return Ok(());
            } else {
                return Err(WeaselError::UserError("move not allowed".to_string()));
            }
        }
        Ok(())
    }

    fn move_entity(
        &self,
        model: &mut Self::SpaceModel,
        claim: PositionClaim<CustomRules>,
        position: Option<&Self::Position>,
        _: &mut WriteMetrics<CustomRules>,
    ) {
        match position {
            Some(play) => {
                // If we try to put a card to the table.
                if *play {
                    // Find an empty slot.
                    let index = model.iter().position(|e| e.is_none()).unwrap();
                    // Insert the card.
                    model[index] = Some(*claim.entity_id());
                }
            }
            None => {
                // Remove the card from the table.
                for entry in model {
                    if let Some(id) = entry {
                        if id == claim.entity_id() {
                            *entry = None;
                        }
                    }
                }
            }
        }
    }
}

// Define our custom actor rules.
#[derive(Default)]
pub struct MyActorRules {}

impl ActorRules<CustomRules> for MyActorRules {
    // The only ability in this game is 'play a card'.
    type Ability = SimpleAbility<u8, ()>;
    type AbilitiesSeed = ();
    // We don't need anything else but the card, to play it.
    type Activation = ();
    // Abilities are immutable.
    type AbilitiesAlteration = ();

    fn generate_abilities(
        &self,
        _: &Option<Self::AbilitiesSeed>,
        _: &mut Entropy<CustomRules>,
        _: &mut WriteMetrics<CustomRules>,
    ) -> Box<dyn Iterator<Item = Self::Ability>> {
        let v = vec![SimpleAbility::new(PLAY_CARD_ABILITY, ())];
        Box::new(v.into_iter())
    }

    fn activate(
        &self,
        _: &BattleState<CustomRules>,
        action: Action<CustomRules>,
        event_queue: &mut Option<EventQueue<CustomRules>>,
        _: &mut Entropy<CustomRules>,
        _: &mut WriteMetrics<CustomRules>,
    ) {
        // The result of playing a card is to change its position from the hand to the table.
        let card = action.actor.entity_id();
        MoveEntity::trigger(event_queue, *card, true).fire();
    }
}

// We use the `battle_rules` macro to define a type `CustomRules` that implements
// the `BattleRules` trait, which as the name suggests defines the game's rules.
// We mix our custom defined rules with default (empty) ones.
battle_rules! {
    MyTeamRules,
    MyCharacterRules,
    MyActorRules,
    EmptyFightRules,
    EmptyUserRules,
    MySpaceRules,
    MyRoundsRules,
    EmptyEntropyRules
}
