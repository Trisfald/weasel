#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use weasel::actor::Actor;
use weasel::battle::{Battle, BattleRules};
use weasel::entity::{Entities, EntityId};
use weasel::entropy::Entropy;
use weasel::event::EventTrigger;
use weasel::metric::{system::*, WriteMetrics};
use weasel::round::{EndRound, ResetRounds, RoundState, RoundsRules, StartRound};
use weasel::server::Server;
use weasel::space::Space;
use weasel::WeaselError;
use weasel::{battle_rules, battle_rules_with_rounds, rules::empty::*};

const TEAM_1_ID: u32 = 1;
const CREATURE_1_ID: u32 = 0;
const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
const CREATURE_2_ID: u32 = 1;
const ENTITY_2_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_2_ID);
const CREATURE_ERR_ID: u32 = 2;
const ENTITY_ERR_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_ERR_ID);

#[derive(Clone, Default, Debug)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
struct Model {
    starts: u32,
    ends: u32,
    adds: u32,
    last: Option<EntityId<CustomRules>>,
}

#[derive(Default)]
struct CustomRoundsRules {}

impl RoundsRules<CustomRules> for CustomRoundsRules {
    type RoundsSeed = Model;
    type RoundsModel = Model;

    fn generate_model(&self, seed: &Option<Self::RoundsSeed>) -> Self::RoundsModel {
        match seed {
            Some(seed) => seed.clone(),
            None => Model::default(),
        }
    }

    fn eligible(&self, model: &Self::RoundsModel, actor: &dyn Actor<CustomRules>) -> bool {
        let entity_id = if model.last == Some(ENTITY_1_ID) {
            ENTITY_2_ID
        } else {
            ENTITY_1_ID
        };
        entity_id == *actor.entity_id()
    }

    fn on_start(
        &self,
        _entities: &Entities<CustomRules>,
        _space: &Space<CustomRules>,
        model: &mut Self::RoundsModel,
        actor: &dyn Actor<CustomRules>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        model.starts += 1;
        model.last = Some(*actor.entity_id());
    }

    fn on_end(
        &self,
        _entities: &Entities<CustomRules>,
        _space: &Space<CustomRules>,
        model: &mut Self::RoundsModel,
        _: &dyn Actor<CustomRules>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        model.ends += 1;
    }

    fn on_actor_added(
        &self,
        model: &mut Self::RoundsModel,
        _: &dyn Actor<CustomRules>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        model.adds += 1;
    }
}

battle_rules_with_rounds! { CustomRoundsRules }

macro_rules! server {
    () => {{
        let mut model = Model::default();
        model.last = Some(ENTITY_2_ID);
        let battle = Battle::builder(CustomRules::new()).build();
        let mut server = Server::builder(battle).build();
        assert_eq!(
            ResetRounds::trigger(&mut server).seed(model).fire().err(),
            None
        );
        util::team(&mut server, TEAM_1_ID);
        util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
        util::creature(&mut server, CREATURE_2_ID, TEAM_1_ID, ());
        server
    }};
}

#[test]
fn start_round() {
    // Initialize the battle.
    let mut server = server!();
    // Pre-start checks.
    assert_eq!(server.battle().rounds().model().adds, 2);
    assert_eq!(*server.battle().rounds().state(), RoundState::<_>::Ready);
    // Check start is prevented for faulty conditions.
    assert_eq!(
        StartRound::trigger(&mut server, ENTITY_ERR_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::EntityNotFound(ENTITY_ERR_ID))
    );
    assert_eq!(
        StartRound::trigger(&mut server, ENTITY_2_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::ActorNotEligible(ENTITY_2_ID))
    );
    assert_eq!(*server.battle().rounds().state(), RoundState::<_>::Ready);
    assert_eq!(server.battle().rounds().model().starts, 0);
    assert_eq!(server.battle().metrics().system_u64(ROUNDS_STARTED), None);
    // Check start works.
    util::start_round(&mut server, &ENTITY_1_ID);
    // Post-start checks.
    assert_eq!(
        *server.battle().rounds().state(),
        RoundState::<_>::Started(ENTITY_1_ID)
    );
    assert_eq!(server.battle().rounds().model().starts, 1);
    assert_eq!(
        server.battle().metrics().system_u64(ROUNDS_STARTED),
        Some(1)
    );
    // Another start in a row must not work.
    assert_eq!(
        StartRound::trigger(&mut server, ENTITY_2_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::RoundInProgress)
    );
    assert_eq!(
        *server.battle().rounds().state(),
        RoundState::<_>::Started(ENTITY_1_ID)
    );
    assert_eq!(server.battle().rounds().model().starts, 1);
    assert_eq!(
        server.battle().metrics().system_u64(ROUNDS_STARTED),
        Some(1)
    );
}

#[test]
fn end_round() {
    // Initialize the battle.
    let mut server = server!();
    // Pre-start checks.
    assert_eq!(server.battle().rounds().model().adds, 2);
    assert_eq!(*server.battle().rounds().state(), RoundState::<_>::Ready);
    // Check end is prevented for faulty conditions.
    assert_eq!(
        EndRound::trigger(&mut server)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::NoRoundInProgress)
    );
    assert_eq!(server.battle().rounds().model().ends, 0);
    assert_eq!(*server.battle().rounds().state(), RoundState::<_>::Ready);
    // Start round.
    util::start_round(&mut server, &ENTITY_1_ID);
    // Check end works.
    util::end_round(&mut server);
    // Post-end checks.
    assert_eq!(server.battle().rounds().model().ends, 1);
    assert_eq!(*server.battle().rounds().state(), RoundState::<_>::Ready);
    // Check start round.
    util::start_round(&mut server, &ENTITY_2_ID);
}

#[test]
fn reset_rounds() {
    // Initialize the battle.
    let mut server = server!();
    // Start a round.
    util::start_round(&mut server, &ENTITY_1_ID);
    // Check that rounds model can't be changed while the round is in progress.
    assert_eq!(
        ResetRounds::trigger(&mut server)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::RoundInProgress)
    );
    // Changing the rounds model in between rounds should be fine.
    util::end_round(&mut server);
    assert_eq!(ResetRounds::trigger(&mut server).fire().err(), None);
}
