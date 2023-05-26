#![doc = include_str!("../README.md")]
#![allow(clippy::type_complexity)]

pub mod event;
pub mod packet;

use std::borrow::Cow;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy_app::{CoreSet, Plugin};
use bevy_ecs::prelude::{Bundle, Component, Entity};
use bevy_ecs::query::{Added, Changed, Or, With};
use bevy_ecs::schedule::{IntoSystemConfig, IntoSystemSetConfig, SystemSet};
use bevy_ecs::system::{Commands, Query, SystemParam};
pub use bevy_hierarchy;
use bevy_hierarchy::{Children, Parent};
use event::{handle_advancement_tab_change, AdvancementTabChange};
use packet::SelectAdvancementTabS2c;
use rustc_hash::FxHashMap;
use valence_client::{Client, FlushPacketsSet, SpawnClientsSet};
use valence_core::ident::Ident;
use valence_core::item::ItemStack;
use valence_core::protocol::encode::WritePacket;
use valence_core::protocol::raw::RawBytes;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{packet_id, Encode, Packet};
use valence_core::text::Text;

pub struct AdvancementPlugin;

#[derive(SystemSet, Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct WriteAdvancementPacketToClientsSet;

#[derive(SystemSet, Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct WriteAdvancementToCacheSet;

impl Plugin for AdvancementPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.configure_sets((
            WriteAdvancementPacketToClientsSet
                .in_base_set(CoreSet::PostUpdate)
                .before(FlushPacketsSet),
            WriteAdvancementToCacheSet
                .in_base_set(CoreSet::PostUpdate)
                .before(WriteAdvancementPacketToClientsSet),
        ))
        .add_event::<AdvancementTabChange>()
        .add_system(
            add_advancement_update_component_to_new_clients
                .after(SpawnClientsSet)
                .in_base_set(CoreSet::PreUpdate),
        )
        .add_system(handle_advancement_tab_change.in_base_set(CoreSet::PreUpdate))
        .add_system(update_advancement_cached_bytes.in_set(WriteAdvancementToCacheSet))
        .add_system(send_advancement_update_packet.in_set(WriteAdvancementPacketToClientsSet));
    }
}

/// Components for advancement that are required
/// Optional components:
/// [AdvancementDisplay]
/// [Parent] - parent advancement
#[derive(Bundle)]
pub struct AdvancementBundle {
    pub advancement: Advancement,
    pub requirements: AdvancementRequirements,
    pub cached_bytes: AdvancementCachedBytes,
}

fn add_advancement_update_component_to_new_clients(
    mut commands: Commands,
    query: Query<Entity, Added<Client>>,
) {
    for client in query.iter() {
        commands
            .entity(client)
            .insert(AdvancementClientUpdate::default());
    }
}

#[derive(SystemParam, Debug)]
struct UpdateAdvancementCachedBytesQuery<'w, 's> {
    advancement_id_query: Query<'w, 's, &'static Advancement>,
    criteria_query: Query<'w, 's, &'static AdvancementCriteria>,
}

impl<'w, 's> UpdateAdvancementCachedBytesQuery<'w, 's> {
    fn write(
        &self,
        a_identifier: &Advancement,
        a_requirements: &AdvancementRequirements,
        a_display: Option<&AdvancementDisplay>,
        a_children: Option<&Children>,
        a_parent: Option<&Parent>,
        w: impl Write,
    ) -> anyhow::Result<()> {
        let Self {
            advancement_id_query,
            criteria_query,
        } = self;

        let mut pkt = packet::Advancement {
            parent_id: None,
            display_data: None,
            criteria: vec![],
            requirements: vec![],
        };

        if let Some(a_parent) = a_parent {
            let a_identifier = advancement_id_query.get(a_parent.get())?;
            pkt.parent_id = Some(a_identifier.0.borrowed());
        }

        if let Some(a_display) = a_display {
            pkt.display_data = Some(packet::AdvancementDisplay {
                title: Cow::Borrowed(&a_display.title),
                description: Cow::Borrowed(&a_display.description),
                icon: &a_display.icon,
                frame_type: VarInt(a_display.frame_type as _),
                flags: a_display.flags(),
                background_texture: a_display.background_texture.as_ref().map(|v| v.borrowed()),
                x_coord: a_display.x_coord,
                y_coord: a_display.y_coord,
            });
        }

        if let Some(a_children) = a_children {
            for a_child in a_children.iter() {
                let Ok(c_identifier) = criteria_query.get(*a_child) else { continue; };
                pkt.criteria.push((c_identifier.0.borrowed(), ()));
            }
        }

        for requirements in a_requirements.0.iter() {
            let mut requirements_p = vec![];
            for requirement in requirements {
                let c_identifier = criteria_query.get(*requirement)?;
                requirements_p.push(c_identifier.0.as_str());
            }
            pkt.requirements.push(packet::AdvancementRequirements {
                requirement: requirements_p,
            });
        }

        (&a_identifier.0, pkt).encode(w)
    }
}

fn update_advancement_cached_bytes(
    mut query: Query<
        (
            &Advancement,
            &AdvancementRequirements,
            &mut AdvancementCachedBytes,
            Option<&AdvancementDisplay>,
            Option<&Children>,
            Option<&Parent>,
        ),
        Or<(
            Changed<AdvancementDisplay>,
            Changed<Children>,
            Changed<Parent>,
            Changed<AdvancementRequirements>,
        )>,
    >,
    update_advancement_cached_bytes_query: UpdateAdvancementCachedBytesQuery,
) {
    for (a_identifier, a_requirements, mut a_bytes, a_display, a_children, a_parent) in
        query.iter_mut()
    {
        a_bytes.0.clear();
        update_advancement_cached_bytes_query
            .write(
                a_identifier,
                a_requirements,
                a_display,
                a_children,
                a_parent,
                &mut a_bytes.0,
            )
            .expect("Failed to write an advancement");
    }
}

#[derive(SystemParam, Debug)]
#[allow(clippy::type_complexity)]
pub(crate) struct SingleAdvancementUpdateQuery<'w, 's> {
    advancement_bytes_query: Query<'w, 's, &'static AdvancementCachedBytes>,
    advancement_id_query: Query<'w, 's, &'static Advancement>,
    criteria_query: Query<'w, 's, &'static AdvancementCriteria>,
    parent_query: Query<'w, 's, &'static Parent>,
}

#[derive(Debug)]
pub(crate) struct AdvancementUpdateEncodeS2c<'w, 's, 'a> {
    client_update: AdvancementClientUpdate,
    queries: &'a SingleAdvancementUpdateQuery<'w, 's>,
}

impl<'w, 's, 'a> Encode for AdvancementUpdateEncodeS2c<'w, 's, 'a> {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        let SingleAdvancementUpdateQuery {
            advancement_bytes_query,
            advancement_id_query,
            criteria_query,
            parent_query,
        } = self.queries;

        let AdvancementClientUpdate {
            new_advancements,
            remove_advancements,
            progress,
            force_tab_update: _,
            reset,
        } = &self.client_update;

        let mut pkt = packet::GenericAdvancementUpdateS2c {
            reset: *reset,
            advancement_mapping: vec![],
            identifiers: vec![],
            progress_mapping: vec![],
        };

        for new_advancement in new_advancements {
            let a_cached_bytes = advancement_bytes_query.get(*new_advancement)?;
            pkt.advancement_mapping
                .push(RawBytes(a_cached_bytes.0.as_slice()));
        }

        for remove_advancement in remove_advancements {
            let a_identifier = advancement_id_query.get(*remove_advancement)?;
            pkt.identifiers.push(a_identifier.0.borrowed());
        }

        let mut progress_mapping: FxHashMap<Entity, Vec<(Entity, Option<i64>)>> =
            FxHashMap::default();
        for progress in progress {
            let a = parent_query.get(progress.0)?;
            progress_mapping
                .entry(a.get())
                .and_modify(|v| v.push(*progress))
                .or_insert(vec![*progress]);
        }

        for (a, c_progresses) in progress_mapping {
            let a_identifier = advancement_id_query.get(a)?;
            let mut c_progresses_p = vec![];
            for (c, c_progress) in c_progresses {
                let c_identifier = criteria_query.get(c)?;
                c_progresses_p.push(packet::AdvancementCriteria {
                    criterion_identifier: c_identifier.0.borrowed(),
                    criterion_progress: c_progress,
                });
            }
            pkt.progress_mapping
                .push((a_identifier.0.borrowed(), c_progresses_p));
        }

        pkt.encode(w)
    }
}

impl<'w, 's, 'a> Packet for AdvancementUpdateEncodeS2c<'w, 's, 'a> {
    const ID: i32 = packet_id::ADVANCEMENT_UPDATE_S2C;
    const NAME: &'static str = "AdvancementUpdateEncodeS2c";
}

#[allow(clippy::type_complexity)]
fn send_advancement_update_packet(
    mut client: Query<(&mut AdvancementClientUpdate, &mut Client)>,
    update_single_query: SingleAdvancementUpdateQuery,
) {
    for (mut advancement_client_update, mut client) in client.iter_mut() {
        match advancement_client_update.force_tab_update {
            ForceTabUpdate::None => {}
            ForceTabUpdate::First => {
                client.write_packet(&SelectAdvancementTabS2c { identifier: None })
            }
            ForceTabUpdate::Spec(spec) => {
                if let Ok(a_identifier) = update_single_query.advancement_id_query.get(spec) {
                    client.write_packet(&SelectAdvancementTabS2c {
                        identifier: Some(a_identifier.0.borrowed()),
                    });
                }
            }
        }

        if ForceTabUpdate::None != advancement_client_update.force_tab_update {
            advancement_client_update.force_tab_update = ForceTabUpdate::None;
        }

        if advancement_client_update.new_advancements.is_empty()
            && advancement_client_update.progress.is_empty()
            && advancement_client_update.remove_advancements.is_empty()
            && !advancement_client_update.reset
        {
            continue;
        }

        let advancement_client_update = std::mem::replace(
            advancement_client_update.as_mut(),
            AdvancementClientUpdate {
                reset: false,
                ..Default::default()
            },
        );

        client.write_packet(&AdvancementUpdateEncodeS2c {
            queries: &update_single_query,
            client_update: advancement_client_update,
        });
    }
}

/// Advancement's id. May not be updated.
#[derive(Component)]
pub struct Advancement(Ident<Cow<'static, str>>);

impl Advancement {
    pub fn new(ident: Ident<Cow<'static, str>>) -> Advancement {
        Self(ident)
    }

    pub fn get(&self) -> &Ident<Cow<'static, str>> {
        &self.0
    }
}

#[derive(Clone, Copy)]
pub enum AdvancementFrameType {
    Task,
    Challenge,
    Goal,
}

/// Advancement display. Optional component
#[derive(Component)]
pub struct AdvancementDisplay {
    pub title: Text,
    pub description: Text,
    pub icon: Option<ItemStack>,
    pub frame_type: AdvancementFrameType,
    pub show_toast: bool,
    pub hidden: bool,
    pub background_texture: Option<Ident<Cow<'static, str>>>,
    pub x_coord: f32,
    pub y_coord: f32,
}

impl AdvancementDisplay {
    pub(crate) fn flags(&self) -> i32 {
        let mut flags = 0;
        flags |= self.background_texture.is_some() as i32;
        flags |= (self.show_toast as i32) << 1;
        flags |= (self.hidden as i32) << 2;
        flags
    }
}

/// Criteria's identifier. May not be updated
#[derive(Component)]
pub struct AdvancementCriteria(Ident<Cow<'static, str>>);

impl AdvancementCriteria {
    pub fn new(ident: Ident<Cow<'static, str>>) -> Self {
        Self(ident)
    }

    pub fn get(&self) -> &Ident<Cow<'static, str>> {
        &self.0
    }
}

/// Requirements for advancement to be completed.
/// All columns should be completed, column is completed when any of criteria in
/// this column is completed.
#[derive(Component, Default)]
pub struct AdvancementRequirements(pub Vec<Vec<Entity>>);

#[derive(Component, Default)]
pub struct AdvancementCachedBytes(pub(crate) Vec<u8>);

#[derive(Default, Debug, PartialEq)]
pub enum ForceTabUpdate {
    #[default]
    None,
    First,
    /// Should contain only root advancement otherwise the first will be chosen
    Spec(Entity),
}

#[derive(Component, Debug)]
pub struct AdvancementClientUpdate {
    /// Which advancement's descriptions send to client
    pub new_advancements: Vec<Entity>,
    /// Which advancements remove from client
    pub remove_advancements: Vec<Entity>,
    /// Criteria progress update.
    /// If None then criteria is not done otherwise it is done
    pub progress: Vec<(Entity, Option<i64>)>,
    /// Forces client to open a tab
    pub force_tab_update: ForceTabUpdate,
    /// Defines if other advancements should be removed.
    /// Also with this flag, client will not show a toast for advancements,
    /// which are completed. When the packet is sent, turns to false
    pub reset: bool,
}

impl Default for AdvancementClientUpdate {
    fn default() -> Self {
        Self {
            new_advancements: vec![],
            remove_advancements: vec![],
            progress: vec![],
            force_tab_update: ForceTabUpdate::default(),
            reset: true,
        }
    }
}

impl AdvancementClientUpdate {
    pub(crate) fn walk_advancements(
        root: Entity,
        children_query: &Query<&Children>,
        advancement_check_query: &Query<(), With<Advancement>>,
        func: &mut impl FnMut(Entity),
    ) {
        func(root);
        if let Ok(children) = children_query.get(root) {
            for child in children.iter() {
                let child = *child;
                if advancement_check_query.get(child).is_ok() {
                    Self::walk_advancements(child, children_query, advancement_check_query, func);
                }
            }
        }
    }

    /// Sends all advancements from the root
    pub fn send_advancements(
        &mut self,
        root: Entity,
        children_query: &Query<&Children>,
        advancement_check_query: &Query<(), With<Advancement>>,
    ) {
        Self::walk_advancements(root, children_query, advancement_check_query, &mut |e| {
            self.new_advancements.push(e)
        });
    }

    /// Removes all advancements from the root
    pub fn remove_advancements(
        &mut self,
        root: Entity,
        children_query: &Query<&Children>,
        advancement_check_query: &Query<(), With<Advancement>>,
    ) {
        Self::walk_advancements(root, children_query, advancement_check_query, &mut |e| {
            self.remove_advancements.push(e)
        });
    }

    /// Marks criteria as done
    pub fn criteria_done(&mut self, criteria: Entity) {
        self.progress.push((
            criteria,
            Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64,
            ),
        ))
    }

    /// Marks criteria as undone
    pub fn criteria_undone(&mut self, criteria: Entity) {
        self.progress.push((criteria, None))
    }
}
