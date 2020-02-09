//! Player-oriented features.

use crate::battle::BattleRules;
use crate::error::{WeaselError, WeaselResult};
use crate::team::TeamId;

/// Type to uniquely identify players.
///
/// PlayerId may be used to authorize client's events. In such case you must make sure that
/// malicious clients can't easily fabricate the id of another player.\
/// One way is to assign
/// a randomly generated PlayerId to each client. Another solution is to assign PlayerId in
/// the server itself when an new event is received from a secure socket.
pub type PlayerId = u64;

/// Manages players' rights to initiate events on behalf of a given team.
pub(crate) struct Rights<R: BattleRules> {
    data: Vec<(PlayerId, Vec<TeamId<R>>)>,
}

impl<R: BattleRules> Rights<R> {
    pub(crate) fn new() -> Rights<R> {
        Rights { data: Vec::new() }
    }

    /// Removes all players without any rights.
    fn cleanup_players(&mut self) {
        self.data.retain(|(_, rights)| !rights.is_empty());
    }

    /// Add rights for `team` to `player`.
    fn add(&mut self, player: PlayerId, team: &TeamId<R>) {
        if let Some((_, rights)) = self.data.iter_mut().find(|(e, _)| *e == player) {
            if rights.iter_mut().find(|e| *e == team).is_none() {
                rights.push(team.clone());
            }
        } else {
            self.data.push((player, vec![team.clone()]));
        }
    }

    /// Remove rights for `team` to `player`.
    fn remove(&mut self, player: PlayerId, team: &TeamId<R>) {
        if let Some((_, rights)) = self.data.iter_mut().find(|(e, _)| *e == player) {
            let index = rights.iter().position(|e| e == team);
            if let Some(index) = index {
                rights.remove(index);
            }
        }
        self.cleanup_players();
    }

    /// Removes all stored rights.
    fn clear(&mut self) {
        self.data.clear();
    }

    /// Returns an iterator over all players' rights.
    fn get(&self) -> impl Iterator<Item = (PlayerId, &[TeamId<R>])> {
        self.data.iter().map(|(player, vec)| (*player, &vec[..]))
    }

    /// Returns `true` if `player` has rights for `team`.
    fn check(&self, player: PlayerId, team: &TeamId<R>) -> bool {
        if let Some((_, rights)) = self.data.iter().find(|(e, _)| *e == player) {
            return rights.iter().any(|e| e == team);
        }
        false
    }

    /// Remove all occurrences of a team from all players' rights.
    fn remove_team(&mut self, team: &TeamId<R>) {
        for (_, rights) in &mut self.data {
            let index = rights.iter().position(|e| e == team);
            if let Some(index) = index {
                rights.remove(index);
            }
        }
        self.cleanup_players();
    }

    /// Remove all rights of a player.
    fn remove_player(&mut self, player: PlayerId) {
        let index = self.data.iter().position(|(e, _)| *e == player);
        if let Some(index) = index {
            self.data.remove(index);
        }
    }
}

/// A structure to access player's rights.
/// Rights are used to control which players can act on behalf of what teams.
pub struct RightsHandle<'a, R>
where
    R: BattleRules,
{
    rights: &'a Rights<R>,
}

impl<'a, R> RightsHandle<'a, R>
where
    R: BattleRules,
{
    pub(crate) fn new(rights: &'a Rights<R>) -> RightsHandle<'a, R> {
        RightsHandle { rights }
    }

    /// Returns an iterator over all players' rights.
    pub fn get(&self) -> impl Iterator<Item = (PlayerId, &[TeamId<R>])> {
        self.rights.get()
    }

    /// Returns `true` if `player` has rights to control `team`.
    pub fn check(&self, player: PlayerId, team: &TeamId<R>) -> bool {
        self.rights.check(player, team)
    }
}

/// A structure to access and manipulate player's rights.
/// Rights are used to control which players can act on behalf of what teams.
pub struct RightsHandleMut<'a, R, I>
where
    R: BattleRules,
    I: Iterator<Item = &'a TeamId<R>>,
{
    rights: &'a mut Rights<R>,
    teams: I,
}

impl<'a, R, I> RightsHandleMut<'a, R, I>
where
    R: BattleRules,
    I: Iterator<Item = &'a TeamId<R>>,
{
    pub(crate) fn new(rights: &'a mut Rights<R>, teams: I) -> RightsHandleMut<'a, R, I> {
        RightsHandleMut { rights, teams }
    }

    /// Add rights to control the team with the given id to `player`. The team must exist.
    pub fn add(&mut self, player: PlayerId, team: &TeamId<R>) -> WeaselResult<(), R> {
        if !self.teams.any(|x| x == team) {
            return Err(WeaselError::TeamNotFound(team.clone()));
        }
        self.rights.add(player, team);
        Ok(())
    }

    /// Remove player rights to control the team with the given id.
    pub fn remove(&mut self, player: PlayerId, team: &TeamId<R>) {
        self.rights.remove(player, team);
    }

    /// Removes all stored rights.
    pub fn clear(&mut self) {
        self.rights.clear();
    }

    /// Remove all occurrences of a team from all players' rights.
    pub fn remove_team(&mut self, team: &TeamId<R>) {
        self.rights.remove_team(team);
    }

    /// Remove all rights of a player.
    pub fn remove_player(&mut self, player: PlayerId) {
        self.rights.remove_player(player);
    }

    /// Returns an iterator over all players' rights.
    pub fn get(&self) -> impl Iterator<Item = (PlayerId, &[TeamId<R>])> {
        self.rights.get()
    }

    /// Returns `true` if `player` has rights to control `team`.
    pub fn check(&self, player: PlayerId, team: &TeamId<R>) -> bool {
        self.rights.check(player, team)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::Battle;
    use crate::{battle_rules, rules::empty::*};

    static PLAYER_1_ID: PlayerId = 1;
    static PLAYER_2_ID: PlayerId = 2;
    static TEAM_1_ID: u32 = 1;
    static TEAM_2_ID: u32 = 2;

    battle_rules! {}

    #[test]
    fn change_rights() {
        let mut rights: Rights<CustomRules> = Rights::new();
        // Add rights for team 1 to player 1.
        rights.add(PLAYER_1_ID, &TEAM_1_ID);
        assert_eq!(rights.data.len(), 1);
        // Add rights for team 2 to player 1.
        rights.add(PLAYER_1_ID, &TEAM_2_ID);
        assert_eq!(rights.data.len(), 1);
        // Add rights for team 1 to player 2.
        rights.add(PLAYER_2_ID, &TEAM_1_ID);
        assert_eq!(rights.data.len(), 2);
        assert_eq!(rights.check(PLAYER_1_ID, &TEAM_1_ID), true);
        assert_eq!(rights.check(PLAYER_1_ID, &TEAM_2_ID), true);
        assert_eq!(rights.check(PLAYER_2_ID, &TEAM_1_ID), true);
        assert_eq!(rights.check(PLAYER_2_ID, &TEAM_2_ID), false);
        // Remove rights for team 2 to player 1.
        rights.remove(PLAYER_1_ID, &TEAM_2_ID);
        assert_eq!(rights.check(PLAYER_1_ID, &TEAM_2_ID), false);
        // Remove rights for team 1 to player 2.
        rights.remove(PLAYER_2_ID, &TEAM_1_ID);
        assert_eq!(rights.check(PLAYER_2_ID, &TEAM_1_ID), false);
        assert_eq!(rights.data.len(), 1);
        // Clear.
        assert_eq!(rights.check(PLAYER_1_ID, &TEAM_1_ID), true);
        rights.clear();
        assert_eq!(rights.data.len(), 0);
        assert_eq!(rights.check(PLAYER_1_ID, &TEAM_1_ID), false);
    }

    #[test]
    fn remove_team() {
        let mut rights: Rights<CustomRules> = Rights::new();
        // Add rights for team 1 to player 1 and player 2.
        rights.add(PLAYER_1_ID, &TEAM_1_ID);
        rights.add(PLAYER_2_ID, &TEAM_1_ID);
        assert_eq!(rights.check(PLAYER_1_ID, &TEAM_1_ID), true);
        assert_eq!(rights.check(PLAYER_2_ID, &TEAM_1_ID), true);
        // Add rights for team 2 to player 1.
        rights.add(PLAYER_1_ID, &TEAM_2_ID);
        assert_eq!(rights.check(PLAYER_1_ID, &TEAM_2_ID), true);
        assert_eq!(rights.data.len(), 2);
        // Remove team 1.
        rights.remove_team(&TEAM_1_ID);
        assert_eq!(rights.check(PLAYER_1_ID, &TEAM_1_ID), false);
        assert_eq!(rights.check(PLAYER_2_ID, &TEAM_1_ID), false);
        assert_eq!(rights.data.len(), 1);
    }

    #[test]
    fn remove_player() {
        let mut rights: Rights<CustomRules> = Rights::new();
        // Add rights for team 1 to player 1 and player 2.
        rights.add(PLAYER_1_ID, &TEAM_1_ID);
        rights.add(PLAYER_2_ID, &TEAM_1_ID);
        assert_eq!(rights.check(PLAYER_1_ID, &TEAM_1_ID), true);
        assert_eq!(rights.check(PLAYER_2_ID, &TEAM_1_ID), true);
        assert_eq!(rights.data.len(), 2);
        // Remove player 1.
        rights.remove_player(PLAYER_1_ID);
        assert_eq!(rights.check(PLAYER_1_ID, &TEAM_1_ID), false);
        assert_eq!(rights.data.len(), 1);
    }

    #[test]
    fn handle() {
        let mut battle = Battle::builder(CustomRules::new()).build();
        // Check that add() verifies team's existence.
        assert_eq!(
            battle.rights_mut().add(PLAYER_1_ID, &TEAM_1_ID).err(),
            Some(WeaselError::TeamNotFound(TEAM_1_ID))
        );
        assert_eq!(battle.rights().get().count(), 0);
    }
}
