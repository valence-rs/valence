use bevy_ecs::prelude::*;
use bevy_ecs::query::WorldQuery;
use valence_protocol::types::Hand;

use super::event::{
    ClientSettings, HandSwing, StartSneaking, StartSprinting, StopSneaking, StopSprinting,
};
use super::{Client, ViewDistance};
use crate::entity::player::PlayerModelParts;
use crate::entity::{entity, player, EntityAnimation, EntityAnimations, EntityKind, Pose};

#[doc(hidden)]
#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct DefaultEventHandlerQuery {
    client: &'static mut Client,
    view_dist: &'static mut ViewDistance,
    player_model_parts: Option<&'static mut PlayerModelParts>,
    pose: &'static mut entity::Pose,
    flags: &'static mut entity::Flags,
    animations: Option<&'static mut EntityAnimations>,
    entity_kind: Option<&'static EntityKind>,
    main_arm: Option<&'static mut player::MainArm>,
}

/// The default event handler system which handles client events in a
/// reasonable default way.
///
/// For instance, movement events are handled by changing the entity's
/// position/rotation to match the received movement, crouching makes the
/// entity crouch, etc.
///
/// This system's primary purpose is to reduce boilerplate code in the
/// examples, but it can be used as a quick way to get started in your own
/// code. The precise behavior of this system is left unspecified and
/// is subject to change.
///
/// This system must be scheduled to run in the
/// [`EventLoopSchedule`]. Otherwise, it may
/// not function correctly.
#[allow(clippy::too_many_arguments)]
pub fn default_event_handler(
    mut clients: Query<DefaultEventHandlerQuery>,
    mut update_settings_events: EventReader<ClientSettings>,
    // mut player_move_events: EventReader<PlayerMove>,
    mut start_sneaking_events: EventReader<StartSneaking>,
    mut stop_sneaking: EventReader<StopSneaking>,
    mut start_sprinting: EventReader<StartSprinting>,
    mut stop_sprinting: EventReader<StopSprinting>,
    mut swing_arm: EventReader<HandSwing>,
) {
    for ClientSettings {
        client,
        view_distance,
        displayed_skin_parts,
        main_arm,
        ..
    } in update_settings_events.iter()
    {
        if let Ok(mut q) = clients.get_mut(*client) {
            q.view_dist.0 = *view_distance;

            if let Some(mut parts) = q.player_model_parts {
                parts.set_if_neq(PlayerModelParts(u8::from(*displayed_skin_parts)));
            }

            if let Some(mut player_main_arm) = q.main_arm {
                player_main_arm.0 = *main_arm as _;
            }
        }
    }

    /*
    for PlayerMove {
        client,
        position,
        yaw,
        pitch,
        on_ground,
        ..
    } in player_move.iter()
    {
        if let Ok((_, Some(mut mcentity), _)) = clients.get_mut(*client) {
            mcentity.set_position(*position);
            mcentity.set_yaw(*yaw);
            mcentity.set_head_yaw(*yaw);
            mcentity.set_pitch(*pitch);
            mcentity.set_on_ground(*on_ground);
        }
    }
    */

    for StartSneaking { client } in start_sneaking_events.iter() {
        if let Ok(mut q) = clients.get_mut(*client) {
            q.pose.set_if_neq(entity::Pose(Pose::Sneaking));
        }
    }

    for StopSneaking { client } in stop_sneaking.iter() {
        if let Ok(mut q) = clients.get_mut(*client) {
            q.pose.set_if_neq(entity::Pose(Pose::Standing));
        }
    }

    for StartSprinting { client } in start_sprinting.iter() {
        if let Ok(mut q) = clients.get_mut(*client) {
            q.flags.set_sprinting(true);
        }
    }

    for StopSprinting { client } in stop_sprinting.iter() {
        if let Ok(mut q) = clients.get_mut(*client) {
            q.flags.set_sprinting(false);
        }
    }

    for HandSwing { client, hand } in swing_arm.iter() {
        if let Ok(q) = clients.get_mut(*client) {
            if let (Some(mut animations), Some(&EntityKind::PLAYER)) = (q.animations, q.entity_kind)
            {
                animations.trigger(match hand {
                    Hand::Main => EntityAnimation::SwingMainHand,
                    Hand::Off => EntityAnimation::SwingOffHand,
                });
            }
        }
    }
}
