//! This module contains generic structs that you can use to compose rules.

/// Macro to quickly generate battle rules.
#[macro_export]
macro_rules! battle_rules {
    () => {
        battle_rules! {
            EmptyTeamRules,
            EmptyCharacterRules,
            EmptyActorRules,
            EmptyFightRules,
            EmptyUserRules,
            EmptySpaceRules,
            EmptyRoundsRules,
            EmptyEntropyRules
        }
    };
    ($ty: ty, $cy: ty, $ay: ty, $fy: ty, $uy: ty, $sy: ty, $ry: ty, $ey: ty) => {
        pub(crate) struct CustomRules {
            pub(crate) team_rules: $ty,
            pub(crate) character_rules: $cy,
            pub(crate) actor_rules: $ay,
            pub(crate) fight_rules: $fy,
            pub(crate) user_rules: $uy,
            pub(crate) space_rules: Option<$sy>,
            pub(crate) rounds_rules: Option<$ry>,
            pub(crate) entropy_rules: Option<$ey>,
            pub(crate) version: u32,
        }

        impl CustomRules {
            #[allow(dead_code)]
            pub(crate) fn new() -> Self {
                Self {
                    team_rules: <$ty>::default(),
                    character_rules: <$cy>::default(),
                    actor_rules: <$ay>::default(),
                    fight_rules: <$fy>::default(),
                    user_rules: <$uy>::default(),
                    space_rules: Some(<$sy>::default()),
                    rounds_rules: Some(<$ry>::default()),
                    entropy_rules: Some(<$ey>::default()),
                    version: 0,
                }
            }
        }

        impl BattleRules for CustomRules {
            type TR = $ty;
            type CR = $cy;
            type AR = $ay;
            type FR = $fy;
            type UR = $uy;
            type SR = $sy;
            type RR = $ry;
            type ER = $ey;
            type Version = u32;

            fn team_rules(&self) -> &Self::TR {
                &self.team_rules
            }
            fn character_rules(&self) -> &Self::CR {
                &self.character_rules
            }
            fn actor_rules(&self) -> &Self::AR {
                &self.actor_rules
            }
            fn fight_rules(&self) -> &Self::FR {
                &self.fight_rules
            }
            fn user_rules(&self) -> &Self::UR {
                &self.user_rules
            }
            fn space_rules(&mut self) -> Self::SR {
                self.space_rules.take().expect("space_rules is None!")
            }
            fn rounds_rules(&mut self) -> Self::RR {
                self.rounds_rules.take().expect("rounds_rules is None!")
            }
            fn entropy_rules(&mut self) -> Self::ER {
                self.entropy_rules.take().expect("entropy_rules is None!")
            }
            fn version(&self) -> &Self::Version {
                &self.version
            }
        }
    };
}

/// Empty battle rules with user defined `EntropyRules`.
#[macro_export]
macro_rules! battle_rules_with_entropy {
    ($ty: ty) => {
        battle_rules! {
            EmptyTeamRules,
            EmptyCharacterRules,
            EmptyActorRules,
            EmptyFightRules,
            EmptyUserRules,
            EmptySpaceRules,
            EmptyRoundsRules,
            $ty
        }
    };
}

/// Empty battle rules with user defined `SpaceRules`.
#[macro_export]
macro_rules! battle_rules_with_space {
    ($ty: ty) => {
        battle_rules! {
            EmptyTeamRules,
            EmptyCharacterRules,
            EmptyActorRules,
            EmptyFightRules,
            EmptyUserRules,
            $ty,
            EmptyRoundsRules,
            EmptyEntropyRules
        }
    };
}

/// Empty battle rules with user defined `RoundsRules`.
#[macro_export]
macro_rules! battle_rules_with_rounds {
    ($ty: ty) => {
        battle_rules! {
            EmptyTeamRules,
            EmptyCharacterRules,
            EmptyActorRules,
            EmptyFightRules,
            EmptyUserRules,
            EmptySpaceRules,
            $ty,
            EmptyEntropyRules
        }
    };
}

/// Empty battle rules with user defined `TeamRules`.
#[macro_export]
macro_rules! battle_rules_with_team {
    ($ty: ty) => {
        battle_rules! {
            $ty,
            EmptyCharacterRules,
            EmptyActorRules,
            EmptyFightRules,
            EmptyUserRules,
            EmptySpaceRules,
            EmptyRoundsRules,
            EmptyEntropyRules
        }
    };
}

/// Empty battle rules with user defined `ActorRules`.
#[macro_export]
macro_rules! battle_rules_with_actor {
    ($ty: ty) => {
        battle_rules! {
            EmptyTeamRules,
            EmptyCharacterRules,
            $ty,
            EmptyFightRules,
            EmptyUserRules,
            EmptySpaceRules,
            EmptyRoundsRules,
            EmptyEntropyRules
        }
    };
}

/// Empty battle rules with user defined `CharacterRules`.
#[macro_export]
macro_rules! battle_rules_with_character {
    ($ty: ty) => {
        battle_rules! {
            EmptyTeamRules,
            $ty,
            EmptyActorRules,
            EmptyFightRules,
            EmptyUserRules,
            EmptySpaceRules,
            EmptyRoundsRules,
            EmptyEntropyRules
        }
    };
}

/// Empty battle rules with user defined `FightRules`.
#[macro_export]
macro_rules! battle_rules_with_fight {
    ($ty: ty) => {
        battle_rules! {
            EmptyTeamRules,
            EmptyCharacterRules,
            EmptyActorRules,
            $ty,
            EmptyUserRules,
            EmptySpaceRules,
            EmptyRoundsRules,
            EmptyEntropyRules
        }
    };
}

/// Empty battle rules with user defined `UserRules`.
#[macro_export]
macro_rules! battle_rules_with_user {
    ($ty: ty) => {
        battle_rules! {
            EmptyTeamRules,
            EmptyCharacterRules,
            EmptyActorRules,
            EmptyFightRules,
            $ty,
            EmptySpaceRules,
            EmptyRoundsRules,
            EmptyEntropyRules
        }
    };
}
