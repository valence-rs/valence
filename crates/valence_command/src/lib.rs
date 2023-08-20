pub mod arg_parser;
pub mod command_graph;
pub mod command_scopes;
pub mod handler;
pub mod manager;

use bevy_ecs::prelude::Resource;
pub use command_scopes::CommandScopeRegistry;

use crate::arg_parser::{CommandArg, CommandArgParseError};
use crate::command_graph::{CommandGraph, CommandGraphBuilder};

pub trait Command {
    fn assemble_graph(graph: &mut CommandGraphBuilder<Self>) where Self: Sized;
}

pub trait CommandArgSet {
    fn parse_args(inputs: Vec<String>) -> Result<Self, CommandArgParseError>
    where
        Self: Sized;
}

impl CommandArgSet for () {
    fn parse_args(_inputs: Vec<String>) -> Result<Self, CommandArgParseError> {
        Ok(())
    }
}

impl<T> CommandArgSet for T
where
    T: CommandArg,
{
    fn parse_args(inputs: Vec<String>) -> Result<Self, CommandArgParseError> {
        T::arg_from_string(inputs[0].clone())
    }
}

impl<T> CommandArgSet for (T,)
where
    T: CommandArg,
{
    fn parse_args(inputs: Vec<String>) -> Result<Self, CommandArgParseError> {
        Ok((T::arg_from_string(inputs[0].clone())?, ))
    }
}

impl<A, B> CommandArgSet for (A, B)
where
    A: CommandArg,
    B: CommandArg,
{
    fn parse_args(inputs: Vec<String>) -> Result<Self, CommandArgParseError> {
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
            fn parse_args(inputs: Vec<String>) -> Result<Self, CommandArgParseError> {
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

#[derive(Resource, Default)]
pub struct CommandRegistry {
    pub graph: CommandGraph,
}