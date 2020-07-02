use weasel::battle::BattleRules;
use weasel::character::{CharacterRules, StatisticId};
use weasel::entropy::Entropy;
use weasel::metric::WriteMetrics;
use weasel::rules::statistic::SimpleStatistic;
use weasel::{battle_rules, battle_rules_with_character, rules::empty::*};

pub(crate) static CARD_VALUE_STAT: StatisticId<CustomRules> = 0;

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
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> Box<dyn Iterator<Item = Self::Statistic>> {
        let value = seed.unwrap();
        // Generate one statistic with the card value.
        let v = vec![SimpleStatistic::new(CARD_VALUE_STAT, value)];
        Box::new(v.into_iter())
    }
}

// We use the `battle_rules_with_character` macro to define a type `CustomRules` that implements
// the `BattleRules` trait, which as the name suggests defines the game's rules.
// All rules are the default (empty) ones except the CharacterRules we just defined above.
battle_rules_with_character! {
    MyCharacterRules
}
