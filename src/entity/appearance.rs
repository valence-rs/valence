use glm::DVec3;

use crate::{glm, Aabb, WorldId};

// TODO: override default clone_from impl for Appearance.
/// Encodes the type of an entity (pig, player, item frame, etc.) along with any
/// state that would influence its appearance to clients.
#[derive(Clone, Debug)]
pub enum Appearance {
    /// The default appearance.
    ///
    /// Entities with an appearance of `None` will not be visible to clients.
    None,
    Player(Player),
}

impl Appearance {
    pub fn position(&self) -> Option<DVec3> {
        match self {
            Appearance::None => None,
            Appearance::Player(p) => Some(p.position),
        }
    }

    pub fn position_mut(&mut self) -> Option<&mut DVec3> {
        match self {
            Appearance::None => None,
            Appearance::Player(p) => Some(&mut p.position),
        }
    }

    pub fn world(&self) -> Option<WorldId> {
        match self {
            Appearance::None => None,
            Appearance::Player(p) => Some(p.world),
        }
    }

    pub fn world_mut(&mut self) -> Option<&mut WorldId> {
        match self {
            Appearance::None => None,
            Appearance::Player(p) => Some(&mut p.world),
        }
    }

    pub fn aabb(&self) -> Option<Aabb<f64, 3>> {
        match self {
            Appearance::None => None,
            Appearance::Player(p) => Some(p.aabb()),
        }
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Debug)]
pub struct Player {
    pub position: DVec3,
    pub world: WorldId,
}

impl Player {
    pub fn new(position: DVec3, world: WorldId) -> Self {
        Self { position, world }
    }

    pub fn aabb(&self) -> Aabb<f64, 3> {
        // TODO: player hitbox dimensions change depending on pose
        Aabb::from_center_and_dimensions(self.position, glm::vec3(0.6, 1.8, 0.6))
    }
}

impl From<Player> for Appearance {
    fn from(p: Player) -> Self {
        Self::Player(p)
    }
}
