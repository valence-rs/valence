use std::borrow::Cow;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use bevy_app::{CoreSet, Plugin};
use bevy_ecs::prelude::{Bundle, Component, Entity};
use bevy_ecs::query::{Added, With};
use bevy_ecs::schedule::{IntoSystemConfig, IntoSystemSetConfig, SystemSet};
use bevy_ecs::system::{Commands, Query, SystemParam};
use bevy_ecs::world::Ref;
#[doc(hidden)]
pub use bevy_hierarchy;
use bevy_hierarchy::{Children, Parent};
use rustc_hash::FxHashMap;
use valence_client::{Client, FlushPacketsSet, SpawnClientsSet};
use valence_core::__private::VarInt;
use valence_core::ident::Ident;
use valence_core::item::ItemStack;
use valence_core::packet::encode::WritePacket;
use valence_core::packet::s2c::play::AdvancementUpdateS2c;
use valence_core::packet::{Encode, Packet};
use valence_core::text::Text;

pub struct AdvancementPlugin;

#[derive(SystemSet, Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct WriteAdvancementPacketToClientsSet;

impl Plugin for AdvancementPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.configure_set(
            WriteAdvancementPacketToClientsSet
                .in_base_set(CoreSet::PostUpdate)
                .before(FlushPacketsSet),
        )
        .add_system(
            init_clients
                .after(SpawnClientsSet)
                .in_base_set(CoreSet::PreUpdate),
        )
        .add_system(send_advancement_update_packet.in_set(WriteAdvancementPacketToClientsSet));
    }
}

#[derive(Bundle)]
pub struct AdvancementBundle {
    pub advancement: Advancement,
}

impl AdvancementBundle {
    pub fn new(ident: Ident<Cow<'static, str>>) -> Self {
        Self {
            advancement: Advancement::new(ident),
        }
    }
}

fn init_clients(mut commands: Commands, query: Query<Entity, Added<Client>>) {
    for client in query.iter() {
        commands
            .entity(client)
            .insert(AdvancementClientUpdate::default());
    }
}

#[derive(SystemParam, Debug)]
pub(crate) struct SingleAdvancementUpdateQuery<'w, 's> {
    advancement_query: Query<
        'w,
        's,
        (
            &'static Advancement,
            Ref<'static, AdvancementRequirements>,
            Option<Ref<'static, AdvancementDisplay>>,
            Option<Ref<'static, Children>>,
            Option<&'static Parent>,
        ),
    >,
    advancement_id_query: Query<'w, 's, &'static Advancement>,
    criteria_query: Query<'w, 's, &'static AdvancementCriteria>,
    parent_query: Query<'w, 's, &'static Parent>,
}

#[derive(Debug)]
pub(crate) struct AdvancementUpdateS2COE<'w, 's, 'a> {
    client_update: AdvancementClientUpdate,
    queries: &'a SingleAdvancementUpdateQuery<'w, 's>,
}

impl<'w, 's, 'a> Encode for AdvancementUpdateS2COE<'w, 's, 'a> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        let SingleAdvancementUpdateQuery {
            advancement_query,
            advancement_id_query,
            criteria_query,
            parent_query,
        } = self.queries;

        let AdvancementClientUpdate {
            new_advancements,
            remove_advancements,
            progress,
        } = &self.client_update;

        // reset/clear
        false.encode(&mut w)?;

        // advancement_mapping
        {
            VarInt(new_advancements.len() as _).encode(&mut w)?;
            for new_advancement in new_advancements {
                let Ok((
                    a_identifier,
                    a_requirements,
                    a_display,
                    a_children,
                    a_parent
                )) = advancement_query.get(*new_advancement) else { continue; };

                // identifier
                a_identifier.get().encode(&mut w)?;

                // parent_id
                a_parent
                    .and_then(|a_parent| advancement_id_query.get(a_parent.get()).ok())
                    .map(|a_id| a_id.get())
                    .encode(&mut w)?;

                // display_data
                match a_display {
                    Some(a_display) => {
                        true.encode(&mut w)?;
                        // advancement display
                        {
                            a_display.title.encode(&mut w)?;
                            a_display.description.encode(&mut w)?;
                            a_display.icon.encode(&mut w)?;
                            VarInt(a_display.frame_type as _).encode(&mut w)?;
                            a_display.flags().encode(&mut w)?;
                            if let Some(ref background_texture) = a_display.background_texture {
                                background_texture.encode(&mut w)?;
                            }
                            a_display.x_coord.encode(&mut w)?;
                            a_display.y_coord.encode(&mut w)?
                        }
                    }
                    None => false.encode(&mut w)?,
                }

                // criteria
                {
                    let mut criteria = vec![];
                    if let Some(a_children) = a_children {
                        for a_child in a_children.iter() {
                            let Ok(c_identifier) = criteria_query.get(*a_child) else { continue; };
                            criteria.push(&c_identifier.0);
                        }
                    }
                    criteria.encode(&mut w)?;
                }

                // requirements
                {
                    VarInt(a_requirements.0.len() as _).encode(&mut w)?;
                    for requirements in a_requirements.0.iter() {
                        VarInt(requirements.len() as _).encode(&mut w)?;
                        for requirement in requirements {
                            let c_identifier = criteria_query
                                .get(*requirement)
                                .expect("Requirements's element is not criteria");
                            c_identifier.0.encode(&mut w)?;
                        }
                    }
                }
            }
        }

        // identifiers
        {
            VarInt(remove_advancements.len() as _).encode(&mut w)?;
            for a in remove_advancements {
                let a_identifier = advancement_id_query
                    .get(*a)
                    .expect("Remove advancements contains not advancement");
                a_identifier.0.encode(&mut w)?;
            }
        }

        // progress_mapping
        {
            let mut progress_mapping: FxHashMap<Entity, Vec<(Entity, Option<i64>)>> =
                FxHashMap::default();
            for progress in progress {
                let a = parent_query
                    .get(progress.0)
                    .expect("criterion does not have a parent");
                progress_mapping
                    .entry(a.get())
                    .and_modify(|v| v.push(*progress))
                    .or_insert(vec![*progress]);
            }

            VarInt(progress_mapping.len() as _).encode(&mut w)?;
            for (a, c_progress) in progress_mapping {
                let a_identifier = advancement_id_query
                    .get(a)
                    .expect("criterion's parent is not advancement");
                a_identifier.0.encode(&mut w)?;

                VarInt(c_progress.len() as _).encode(&mut w)?;
                for (c, c_progress) in c_progress {
                    let c_identifier = criteria_query
                        .get(c)
                        .expect("progress contains not criteria");
                    c_identifier.0.encode(&mut w)?;
                    c_progress.encode(&mut w)?;
                }
            }
        }

        Ok(())
    }
}

impl<'w, 's, 'a, 'b> Packet<'b> for AdvancementUpdateS2COE<'w, 's, 'a> {
    const PACKET_ID: i32 = AdvancementUpdateS2c::PACKET_ID;

    fn packet_id(&self) -> i32 {
        Self::PACKET_ID
    }

    fn packet_name(&self) -> &str {
        "AdvancementUpdateS2c_OnlyEncode"
    }

    fn encode_packet(&self, mut w: impl Write) -> anyhow::Result<()> {
        VarInt(Self::PACKET_ID)
            .encode(&mut w)
            .context("failed to encode packet ID")?;
        self.encode(w)
    }

    fn decode_packet(_r: &mut &'b [u8]) -> anyhow::Result<Self> {
        panic!("Packet can not be decoded")
    }
}

fn send_advancement_update_packet(
    mut client: Query<(&mut AdvancementClientUpdate, &mut Client)>,
    update_single_query: SingleAdvancementUpdateQuery,
) {
    for (mut advancement_client_update, mut client) in client.iter_mut() {
        if advancement_client_update.new_advancements.is_empty()
            && advancement_client_update.progress.is_empty()
            && advancement_client_update.remove_advancements.is_empty()
        {
            continue;
        }

        let advancement_client_update = std::mem::take(advancement_client_update.as_mut());

        client.write_packet(&AdvancementUpdateS2COE {
            queries: &update_single_query,
            client_update: advancement_client_update,
        });
    }
}

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

#[derive(Component)]
pub struct AdvancementRequirements(pub Vec<Vec<Entity>>);

#[derive(Component, Default, Debug)]
pub struct AdvancementClientUpdate {
    pub new_advancements: Vec<Entity>,
    pub remove_advancements: Vec<Entity>,
    pub progress: Vec<(Entity, Option<i64>)>,
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
                if let Ok(_) = advancement_check_query.get(child) {
                    Self::walk_advancements(child, children_query, advancement_check_query, func);
                }
            }
        }
    }

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

    pub fn criteria_done(&mut self, criteria: Entity) {
        self.progress.push((
            criteria,
            Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64
            ),
        ))
    }

    pub fn criteria_undone(&mut self, criteria: Entity) {
        self.progress.push((criteria, None))
    }
}
