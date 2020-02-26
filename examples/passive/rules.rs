#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use weasel::ability::AbilityId;
use weasel::actor::{Actor, ActorRules, AlterAbilities};
use weasel::battle::{BattleRules, BattleState};
use weasel::entropy::Entropy;
use weasel::event::{EventQueue, EventTrigger};
use weasel::metric::WriteMetrics;
use weasel::rules::ability::SimpleAbility;
use weasel::{battle_rules, battle_rules_with_actor, rules::empty::*};

/// Id for the active ability 'punch'.
pub(crate) static PUNCH: AbilityId<CustomRules> = 1;
/// Id for the passive ability 'power up'.
pub(crate) static POWER_UP: AbilityId<CustomRules> = 2;
/// Starting power for punches.
pub(crate) static PUNCH_START_POWER: u32 = 10;

// In this example we only need to redefine the actor rules
battle_rules_with_actor! {
    CustomActorRules
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum AbilityPower {
    Passive,
    Attack(u32),
}

// Define our custom actor rules.
#[derive(Default)]
pub struct CustomActorRules {}

impl ActorRules<CustomRules> for CustomActorRules {
    // Abilities can either be passives or direct attacks.
    type Ability = SimpleAbility<u32, AbilityPower>;
    // Vector with Id of abilities to generate.
    type AbilitiesSeed = Vec<AbilityId<CustomRules>>;
    // We don't care for activations in this example.
    type Activation = ();
    // We need to be able to modify the PUNCH ability.
    // Let's use a tuple with ability id and a new AbilityPower.
    type AbilitiesAlteration = (u32, AbilityPower);

    fn generate_abilities(
        &self,
        seed: &Option<Self::AbilitiesSeed>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> Box<dyn Iterator<Item = Self::Ability>> {
        let mut v = Vec::new();
        // Generate abilities from the seed.
        if let Some(seed) = seed {
            for id in seed {
                if *id == PUNCH {
                    // For PUNCH we generate an attack ability.
                    v.push(SimpleAbility::new(
                        *id,
                        AbilityPower::Attack(PUNCH_START_POWER),
                    ));
                } else if *id == POWER_UP {
                    // For POWER_UP we generate a passive ability.
                    v.push(SimpleAbility::new(*id, AbilityPower::Passive));
                }
            }
        }
        Box::new(v.into_iter())
    }

    fn alter(
        &self,
        actor: &mut dyn Actor<CustomRules>,
        alteration: &Self::AbilitiesAlteration,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        // Alter abilities.
        // We know the id to alter and what is the new value.
        let id_to_alter = alteration.0;
        let new_power = alteration.1;
        if let Some(ability) = actor.ability_mut(&id_to_alter) {
            ability.set_power(new_power);
        }
    }

    fn on_round_end(
        &self,
        state: &BattleState<CustomRules>,
        actor: &dyn Actor<CustomRules>,
        event_queue: &mut Option<EventQueue<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        // In this method we activate the effect of our passive.
        // First check if the actor knows the passive ability.
        if actor.ability(&POWER_UP).is_some() {
            // Now we take the number of creatures in the game.
            let count = state.entities().creatures().count();
            // Get the current power of the actor's punch.
            if let Some(punch) = actor.ability(&PUNCH) {
                // Sum the number of creatures to the power of punch.
                let current_power = if let AbilityPower::Attack(p) = punch.power() {
                    p
                } else {
                    0
                };
                let new_power = current_power + count as u32;
                // Construct an ability alteration.
                let alteration = (PUNCH, AbilityPower::Attack(new_power));
                // Alter the actor punch ability.
                AlterAbilities::trigger(event_queue, *actor.entity_id(), alteration).fire();
            }
        }
    }
}
