use std::{borrow::Cow, ops::Range, marker::PhantomData};

use bevy_ecs::prelude::{Event, Component};
use valence_core::ident::Ident;

type RawExecuteId = u64;

#[derive(Component, Clone, Debug)]
pub struct Command {
    pub nodes: Vec<Node<'static>>,
    pub root: usize, 
}

impl Command {

    pub fn execute(&self, arg: String) -> Result<RawExecuteId, ()> {
        Ok(0)
    }

}

#[derive(Clone, Debug)]
pub struct Node<'a> {
    pub children: Vec<usize>,
    pub executable: Option<RawExecuteId>,
    pub redirect: Option<usize>,
    pub kind: NodeKind<'a>,
}

#[derive(Clone, Debug)]
pub enum NodeKind<'a> {
    Root,
    Literal(Cow<'a, str>),
    Argument {
        name: Cow<'a, str>,
        parser: Parser<'a>,
        suggestion: Option<Suggestion>,
    }
}

#[derive(Clone, Debug)]
pub enum Parser<'a> {
    Bool,
    Float { range: Range<f32> },
    Double { range: Range<f64> },
    Integer { range: Range<i32> },
    Long { range: Range<i64>, },
    String(StringArg),
    Entity { single: bool, only_players: bool },
    GameProfile,
    BlockPos,
    ColumnPos,
    Vec3,
    Vec2,
    BlockState,
    BlockPredicate,
    ItemStack,
    ItemPredicate,
    Color,
    Component,
    Message,
    NbtCompoundTag,
    NbtTag,
    NbtPath,
    Objective,
    ObjectiveCriteria,
    Operation,
    Particle,
    Angle,
    Rotation,
    ScoreboardSlot,
    ScoreHolder { allow_multiple: bool },
    Swizzle,
    Team,
    ItemSlot,
    ResourceLocation,
    Function,
    EntityAnchor,
    IntRange,
    FloatRange,
    Dimension,
    GameMode,
    Time,
    ResourceOrTag { registry: Ident<Cow<'a, str>> },
    ResourceOrTagKey { registry: Ident<Cow<'a, str>> },
    Resource { registry: Ident<Cow<'a, str>> },
    ResourceKey { registry: Ident<Cow<'a, str>> },
    TemplateMirror,
    TemplateRotation,
    Uuid,
}

#[derive(Copy, Clone, Debug)]
pub enum StringArg {
    SingleWord,
    QuotablePhrase,
    GreedyPhrase,
}

#[derive(Clone, Copy, Debug)]
pub enum Suggestion {
    AskServer(RawExecuteId),
    AllRecipes,
    AvailableSounds,
    AvailableBiomes,
    SummonableEntities,
}