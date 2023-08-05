pub mod arg_parser;
pub mod command_graph;
pub mod handler;
pub mod manager;
pub mod packet;
pub mod command_scopes;





use bevy_ecs::entity::Entity;
use bevy_ecs::event::Event;
use bevy_ecs::prelude::{Resource};
pub use command_scopes::CommandScopeRegistry;



use crate::arg_parser::{CommandArg, CommandArgParseError};
use crate::command_graph::{CommandGraph, CommandGraphBuilder};

pub trait Command {
    type CommandExecutables: Send + Sync; // usually an enum of all the possible commands

    fn name() -> String;
    fn assemble_graph(&self, graph: &mut CommandGraphBuilder<Self::CommandExecutables>);
}

pub trait Executable: Event {
    fn from_args(
        &self,
        executor: Entity,
        args: String,
    ) -> Result<Box<dyn Event>, CommandArgParseError>;
}

pub trait CommandArgSet {
    type ArgResult;

    fn parse_args(inputs: Vec<String>) -> Result<Self::ArgResult, CommandArgParseError>
    where
        Self: Sized;
}

impl CommandArgSet for () {
    type ArgResult = ();

    fn parse_args(_inputs: Vec<String>) -> Result<Self::ArgResult, CommandArgParseError> {
        Ok(())
    }
}

impl<T> CommandArgSet for T
where
    T: CommandArg,
{
    type ArgResult = T::Result;

    fn parse_args(inputs: Vec<String>) -> Result<Self::ArgResult, CommandArgParseError> {
        Ok(T::arg_from_string(inputs[0].clone())?)
    }
}

impl<T> CommandArgSet for (T,)
where
    T: CommandArg,
{
    type ArgResult = T::Result;

    fn parse_args(inputs: Vec<String>) -> Result<Self::ArgResult, CommandArgParseError> {
        Ok(T::arg_from_string(inputs[0].clone())?)
    }
}

impl<A, B> CommandArgSet for (A, B)
where
    A: CommandArg,
    B: CommandArg,
{
    type ArgResult = (A::Result, B::Result);

    fn parse_args(inputs: Vec<String>) -> Result<Self::ArgResult, CommandArgParseError> {
        let mut inputs = inputs.into_iter();

        Ok((
            A::arg_from_string(inputs.next().unwrap())?,
            B::arg_from_string(inputs.next().unwrap())?,
        ))
    }
}

macro_rules! impl_arg_set {
    ($($arg:ident),*) => {
        impl<$($arg),*> CommandArgSet for ($($arg),*) where $($arg: CommandArg),* {
            type ArgResult = ($($arg::Result),*);

            fn parse_args(inputs: Vec<String>) -> Result<Self::ArgResult, CommandArgParseError> {
                let mut inputs = inputs.into_iter();

                Ok(($($arg::arg_from_string(inputs.next().unwrap())?),*))
            }
        }
    };
}

impl_arg_set!(A, B, C);
impl_arg_set!(A, B, C, D);
impl_arg_set!(A, B, C, D, E);
impl_arg_set!(A, B, C, D, E, F);
impl_arg_set!(A, B, C, D, E, F, G);
impl_arg_set!(A, B, C, D, E, F, G, H);
impl_arg_set!(A, B, C, D, E, F, G, H, I);
impl_arg_set!(A, B, C, D, E, F, G, H, I, J);
impl_arg_set!(A, B, C, D, E, F, G, H, I, J, K);
impl_arg_set!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_arg_set!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_arg_set!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_arg_set!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_arg_set!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P); // this is ridiculous
impl_arg_set!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
impl_arg_set!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
impl_arg_set!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, S, T);
impl_arg_set!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, S, T, U);
impl_arg_set!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, S, T, U, V);
impl_arg_set!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, S, T, U, V, W);
impl_arg_set!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, S, T, U, V, W, X);
impl_arg_set!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, S, T, U, V, W, X, Y);
impl_arg_set!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, S, T, U, V, W, X, Y, Z); // I'm sorry

// first arg is the executor (creates struct) second arg is the args
// executable!(ExecutorToLocation, arg_parser::Vec3) should be equivalent to
//
//#[derive(Event)]
// struct ExecutorToLocation {
//     executor: Entity,
//     args: (arg_parser::Vec3),
// }
//
// impl Default for ExecutorToLocation {
//     fn default() -> Self {
//         Self {
//             executor: Entity::PLACEHOLDER,
//             ..Default::default()
//         }
//     }
// }
//
// impl Executable for ExecutorToLocation {
//     fn from_args(executor: Entity, args: String) -> Result<Self,
// CommandArgParseError> {         let location =
// arg_parser::Vec3::from_string(args)?;         Ok(Self { executor, location })
//     }
// }

#[macro_export]
macro_rules! executable {
    ($name:ident, $($arg:ident),*) => {
        #[derive(Event)]
        struct $name {
            executor: Entity,
            args: ($(arg_parser::$arg),*),
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    executor: Entity::PLACEHOLDER,
                    args: ($(arg_parser::$arg::default()),*),
                }
            }
        }

        impl Executable for $name {
            fn from_args(&self, executor: Entity, args: String) -> Result<Box<(dyn valence::prelude::Event)>, CommandArgParseError> {
                let args = <($(arg_parser::$arg),*)>::from_string(args)?;
                Ok(Box::new(Self { executor, args }))
            }
        }
    };
}

#[derive(Resource, Default)]
pub struct CommandRegistry {
    pub graph: CommandGraph,
}

#[derive(Event, Debug)]
/// This event is sent when a command is partially typed into the console and
/// the user is still typing
pub struct CommandTypingEvent<T>
where
    T: Command + Resource,
{
    command: String,
    executor: Entity,
    _phantom: std::marker::PhantomData<T>,
}
