//! Command graph implementation
//!
//! This is the core of the command system. It is a graph of `CommandNode`s that
//! are connected by the `CommandEdgeType`. The graph is used to determine what
//! command to run when a command is entered. The graph is also used to generate
//! the command tree that is sent to the client.
//!
//! ### The graph is a directed graph with 3 types of nodes:
//! * Root node ([NodeData::Root]) - This is the root of the graph.  It is used
//!   to connect all the
//! other nodes to the graph. It is always present and there should only be one.
//! * Literal node ([NodeData::Literal]) - This is a literal part of a command.
//!   It is a string that
//! must be matched exactly by the client to trigger the validity of the node.
//! For example, the command `/teleport` would have a literal node with the name
//! `teleport` which is a child of the root node.
//! * Argument node ([NodeData::Argument]) - This is a node that represents an
//!   argument in a
//! command. It is a string that is matched by the client and checked by the
//! server. For example, the command `/teleport 0 0 0` would have 1 argument
//! node with the name "<destination:location>" and the parser [Parser::Vec3]
//! which is a child of the literal node with the name `teleport`.
//!
//! #### and 2 types of edges:
//! * Child edge ([CommandEdgeType::Child]) - This is an edge that connects a
//!   parent node to a
//! child node. It is used to determine what nodes are valid children of a
//! parent node. for example, the literal node with the name `teleport` would
//! have a child edge to the argument node with the name
//! "<destination:location>". This means that the argument node is a valid child
//! of the literal node.
//! * Redirect edge ([CommandEdgeType::Redirect]) - This edge is special. It is
//!   used to redirect the
//! client to another node. For example, the literal node with the name `tp`
//! would have a Redirect edge to the literal node with the name `teleport`.
//! This means that if the client enters the command `/tp` the server will
//! redirect the client to the literal node with the name `teleport`. Making the
//! command `/tp` functionally equivalent to `/teleport`.
//!
//! # Cool Example Graph For Possible Implementation Of Teleport Command (made with graphviz)
//! ```text
//!                                               ┌────────────────────────────────┐
//!                                               │              Root              │ ─┐
//!                                               └────────────────────────────────┘  │
//!                                                 │                                 │
//!                                                 │ Child                           │
//!                                                 ▼                                 │
//!                                               ┌────────────────────────────────┐  │
//!                                               │          Literal: tp           │  │
//!                                               └────────────────────────────────┘  │
//!                                                 │                                 │
//!                                                 │ Redirect                        │ Child
//!                                                 ▼                                 ▼
//! ┌──────────────────────────────────┐  Child   ┌──────────────────────────────────────────────────────────────────────────────┐
//! │  Argument: <destination:entity>  │ ◀─────── │                              Literal: teleport                               │
//! └──────────────────────────────────┘          └──────────────────────────────────────────────────────────────────────────────┘
//!                                                 │                                           │
//!                                                 │ Child                                     │ Child
//!                                                 ▼                                           ▼
//! ┌──────────────────────────────────┐  Child   ┌────────────────────────────────┐          ┌──────────────────────────────────┐
//! │ Argument: <destination:location> │ ◀─────── │   Argument: <target:entity>    │          │ Argument: <destination:location> │
//! └──────────────────────────────────┘          └────────────────────────────────┘          └──────────────────────────────────┘
//!                                                 │
//!                                                 │ Child
//!                                                 ▼
//!                                               ┌────────────────────────────────┐
//!                                               │ Argument: <destination:entity> │
//!                                               └────────────────────────────────┘
//! ```

use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use anyhow::bail;
use petgraph::dot::Dot;
use petgraph::prelude::*;
use valence_server::protocol::packets::play::command_tree_s2c::{
    Node, NodeData as PacketNodeData, Parser, StringArg, Suggestion,
};
use valence_server::protocol::packets::play::CommandTreeS2c;
use valence_server::protocol::{Decode, Encode, VarInt};
use valence_server::Ident;

use crate::arg_parser::{ArgLen, CommandArg, GreedyString, QuotableString};
use crate::command_scopes::Scope;
use crate::{arg_parser, CommandRegistry};

/// This struct is used to store the command graph.(see module level docs for
/// more info)
#[derive(Debug, Clone)]
pub struct CommandGraph {
    pub graph: Graph<CommandNode, CommandEdgeType>,
    pub root: NodeIndex,
}

impl Default for CommandGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Output the graph in graphviz dot format to do visual debugging. (this was
/// used to make the cool graph in the module level docs)
impl Display for CommandGraph {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Dot::new(&self.graph))
    }
}

impl CommandGraph {
    pub fn new() -> Self {
        let mut graph = Graph::<CommandNode, CommandEdgeType>::new();
        let root = graph.add_node(CommandNode {
            executable: false,
            data: NodeData::Root,
            scopes: vec![],
        });

        Self { graph, root }
    }
}

/// Data for the nodes in the graph (see module level docs for more info)
#[derive(Clone, Debug)]
pub struct CommandNode {
    pub executable: bool,
    pub data: NodeData,
    pub scopes: Vec<Scope>,
}

impl Display for CommandNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.data {
            NodeData::Root => write!(f, "Root"),
            NodeData::Literal { name } => write!(f, "Literal: {}", name),
            NodeData::Argument { name, .. } => write!(f, "Argument: <{}>", name),
        }
    }
}

#[derive(Clone, Debug)]
pub enum NodeData {
    Root,
    Literal {
        name: String,
    },
    Argument {
        name: String,
        parser: Parser,
        suggestion: Option<Suggestion>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CommandEdgeType {
    Redirect,
    Child,
}

impl Display for CommandEdgeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandEdgeType::Redirect => write!(f, "Redirect"),
            CommandEdgeType::Child => write!(f, "Child"),
        }
    }
}

fn validate_min_max<T: PartialOrd>(val: T, min: Option<T>, max: Option<T>) -> bool {
    if let Some(min) = min {
        if val < min {
            return false;
        }
    }
    if let Some(max) = max {
        if val > max {
            return false;
        }
    }
    true
}

pub fn parser_len(parser: &Parser) -> ArgLen {
    match parser {
        // TODO: Investigate this further
        Parser::Bool => ArgLen::Exact(1),
        Parser::Float { .. } => ArgLen::Exact(1),
        Parser::Double { .. } => ArgLen::Exact(1),
        Parser::Integer { .. } => ArgLen::Exact(1),
        Parser::Long { .. } => ArgLen::Exact(1),
        Parser::String(arg) => match arg {
            StringArg::SingleWord => ArgLen::Exact(1),
            StringArg::QuotablePhrase => ArgLen::Within('"'),
            StringArg::GreedyPhrase => ArgLen::Infinite,
        },
        Parser::Entity { .. } => ArgLen::Exact(1),
        Parser::GameProfile => ArgLen::Exact(1),
        Parser::BlockPos => ArgLen::Exact(3),
        Parser::ColumnPos => ArgLen::Exact(3),
        Parser::Vec3 => ArgLen::Exact(3),
        Parser::Vec2 => ArgLen::Exact(2),
        Parser::BlockState => ArgLen::Exact(1),
        Parser::BlockPredicate => ArgLen::Exact(1),
        Parser::ItemStack => ArgLen::Exact(1),
        Parser::ItemPredicate => ArgLen::Exact(1),
        Parser::Color => ArgLen::Exact(1),
        Parser::Component => ArgLen::Infinite,
        Parser::Message => ArgLen::Infinite,
        Parser::NbtCompoundTag => ArgLen::Infinite,
        Parser::NbtTag => ArgLen::Infinite,
        Parser::NbtPath => ArgLen::Infinite,
        Parser::Objective => ArgLen::Exact(1),
        Parser::ObjectiveCriteria => ArgLen::Exact(1),
        Parser::Operation => ArgLen::Exact(1),
        Parser::Particle => ArgLen::Exact(1),
        Parser::Angle => ArgLen::Exact(1),
        Parser::Rotation => ArgLen::Exact(2),
        Parser::ScoreboardSlot => ArgLen::Exact(1),
        Parser::ScoreHolder { .. } => ArgLen::Exact(1),
        Parser::Swizzle => ArgLen::Exact(3),
        Parser::Team => ArgLen::Exact(1),
        Parser::ItemSlot => ArgLen::Exact(1),
        Parser::ResourceLocation => ArgLen::Exact(1),
        Parser::Function => ArgLen::Exact(1),
        Parser::EntityAnchor => ArgLen::Exact(1),
        Parser::IntRange => ArgLen::Exact(2),
        Parser::FloatRange => ArgLen::Exact(2),
        Parser::Dimension => ArgLen::Exact(1),
        Parser::GameMode => ArgLen::Exact(1),
        Parser::Time => ArgLen::Exact(1),
        Parser::ResourceOrTag { .. } => ArgLen::Exact(1),
        Parser::ResourceOrTagKey { .. } => ArgLen::Exact(1),
        Parser::Resource { .. } => ArgLen::Exact(1),
        Parser::ResourceKey { .. } => ArgLen::Exact(1),
        Parser::TemplateMirror => ArgLen::Exact(1),
        Parser::TemplateRotation => ArgLen::Exact(1),
        Parser::Uuid => ArgLen::Exact(1),
    }
}

pub fn parser_valid_for(parser: &Parser, arg: String) -> bool {
    match parser {
        Parser::Bool => bool::arg_from_string(arg).is_ok(),
        Parser::Float { min, max } => {
            let float = f32::arg_from_string(arg);
            match float {
                Ok(float) => validate_min_max(float, *min, *max),
                Err(_) => false,
            }
        }
        Parser::Double { min, max } => {
            let double = f64::arg_from_string(arg);
            match double {
                Ok(double) => validate_min_max(double, *min, *max),
                Err(_) => false,
            }
        }
        Parser::Integer { min, max } => {
            let int = i32::arg_from_string(arg);
            match int {
                Ok(int) => validate_min_max(int, *min, *max),
                Err(_) => false,
            }
        }
        Parser::Long { min, max } => {
            let long = i64::arg_from_string(arg);
            match long {
                Ok(long) => validate_min_max(long, *min, *max),
                Err(_) => false,
            }
        }
        Parser::String(arg_type) => match arg_type {
            StringArg::SingleWord => String::arg_from_string(arg).is_ok(),
            StringArg::QuotablePhrase => QuotableString::arg_from_string(arg).is_ok(),
            StringArg::GreedyPhrase => GreedyString::arg_from_string(arg).is_ok(),
        },
        Parser::Entity {
            single,
            only_players,
        } => {
            if *single && *only_players {
                arg_parser::SinglePlayerSelector::arg_from_string(arg).is_ok()
            } else if *single && !*only_players {
                arg_parser::SingleEntitySelector::arg_from_string(arg).is_ok()
            } else if *only_players && !*single {
                arg_parser::PlayerSelector::arg_from_string(arg).is_ok()
            } else {
                arg_parser::EntitySelector::arg_from_string(arg).is_ok()
            }
        }
        Parser::GameProfile => {
            String::arg_from_string(arg).is_ok() // TODO
        }
        Parser::BlockPos => arg_parser::BlockPos::arg_from_string(arg).is_ok(),
        Parser::ColumnPos => arg_parser::ColumnPos::arg_from_string(arg).is_ok(),
        Parser::Vec3 => arg_parser::Vec3::arg_from_string(arg).is_ok(),
        Parser::Vec2 => arg_parser::Vec2::arg_from_string(arg).is_ok(),
        Parser::BlockState => arg_parser::BlockState::arg_from_string(arg).is_ok(),
        Parser::BlockPredicate => arg_parser::BlockPredicate::arg_from_string(arg).is_ok(),
        Parser::ItemStack => arg_parser::ItemStack::arg_from_string(arg).is_ok(),
        Parser::ItemPredicate => {
            String::arg_from_string(arg).is_ok() // TODO
        }
        Parser::Color => arg_parser::ChatColor::arg_from_string(arg).is_ok(),
        Parser::Component => arg_parser::JsonChatComponent::arg_from_string(arg).is_ok(),
        Parser::Message => arg_parser::Message::arg_from_string(arg).is_ok(),
        Parser::NbtCompoundTag => arg_parser::JsonChatComponent::arg_from_string(arg).is_ok(),
        Parser::NbtTag => arg_parser::NbtTag::arg_from_string(arg).is_ok(),
        Parser::NbtPath => arg_parser::NbtPath::arg_from_string(arg).is_ok(),
        Parser::Objective => arg_parser::Objective::arg_from_string(arg).is_ok(),
        Parser::ObjectiveCriteria => arg_parser::ObjectiveCriteria::arg_from_string(arg).is_ok(),
        Parser::Operation => arg_parser::Objective::arg_from_string(arg).is_ok(),
        Parser::Particle => {
            String::arg_from_string(arg).is_ok() // TODO
        }
        Parser::Angle => arg_parser::Angle::arg_from_string(arg).is_ok(),
        Parser::Rotation => arg_parser::Rotation::arg_from_string(arg).is_ok(),
        Parser::ScoreboardSlot => arg_parser::ScoreboardSlot::arg_from_string(arg).is_ok(),
        Parser::ScoreHolder { .. } => arg_parser::ScoreHolder::arg_from_string(arg).is_ok(),
        Parser::Swizzle => arg_parser::ScoreHolder::arg_from_string(arg).is_ok(),
        Parser::Team => arg_parser::TeamName::arg_from_string(arg).is_ok(),
        Parser::ItemSlot => arg_parser::ItemStack::arg_from_string(arg).is_ok(),
        Parser::ResourceLocation => arg_parser::ResourceLocation::arg_from_string(arg).is_ok(),
        Parser::Function => arg_parser::Function::arg_from_string(arg).is_ok(),
        Parser::EntityAnchor => arg_parser::EntityAnchor::arg_from_string(arg).is_ok(),
        Parser::IntRange => arg_parser::IntRange::arg_from_string(arg).is_ok(),
        Parser::FloatRange => arg_parser::FloatRange::arg_from_string(arg).is_ok(),
        Parser::Dimension => arg_parser::Dimension::arg_from_string(arg).is_ok(),
        Parser::GameMode => arg_parser::GameMode::arg_from_string(arg).is_ok(),
        Parser::Time => arg_parser::Time::arg_from_string(arg).is_ok(),
        Parser::ResourceOrTag { .. } => {
            String::arg_from_string(arg).is_ok() // TODO
        }
        Parser::ResourceOrTagKey { .. } => {
            String::arg_from_string(arg).is_ok() // TODO
        }
        Parser::Resource { .. } => {
            String::arg_from_string(arg).is_ok() // TODO
        }
        Parser::ResourceKey { .. } => {
            String::arg_from_string(arg).is_ok() // TODO
        }
        Parser::TemplateMirror => {
            String::arg_from_string(arg).is_ok() // TODO
        }
        Parser::TemplateRotation => {
            String::arg_from_string(arg).is_ok() // TODO
        }
        Parser::Uuid => arg_parser::Uuid::arg_from_string(arg).is_ok(),
    }
}

impl From<NodeData> for PacketNodeData {
    fn from(value: NodeData) -> Self {
        match value {
            NodeData::Root => PacketNodeData::Root,
            NodeData::Literal { name } => PacketNodeData::Literal { name },
            NodeData::Argument {
                name,
                parser,
                suggestion,
            } => PacketNodeData::Argument {
                name,
                parser,
                suggestion,
            },
        }
    }
}

impl From<CommandGraph> for CommandTreeS2c {
    fn from(value: CommandGraph) -> Self {
        let mut nodes = Vec::new();
        let graph = value.graph;
        let root_index_graph = value.root;
        let root_index = 0;
        let mut nodes_to_be_allocated = Vec::new();

        // Find all the nodes children and redirects that have to happen
        for node in graph.node_indices() {
            let mut children = Vec::new();

            let mut redirect_node = None;
            for edge in graph.edges_directed(node, Outgoing) {
                match edge.weight() {
                    &CommandEdgeType::Redirect => redirect_node = Some(edge.target()),
                    &CommandEdgeType::Child => children.push(edge.target()),
                }
            }

            // we dont actually know where in the list the redirect node and children are
            // yet, so we have to do this
            nodes_to_be_allocated.push((node, children, redirect_node));
        }

        let mut index_map: HashMap<NodeIndex, usize> = HashMap::new();

        // Finalise the index of all nodes in the vec
        for (index, _, _) in &nodes_to_be_allocated {
            let mut node = CommandNode {
                executable: false,
                data: NodeData::Root,
                scopes: Vec::new(),
            };

            if *index == root_index_graph {
                nodes.push(CommandNode {
                    executable: false,
                    data: NodeData::Root,
                    scopes: Vec::new(),
                });
                index_map.insert(*index, nodes.len() - 1);
                continue;
            } else {
                let node_data = graph.node_weight(*index).unwrap();
                node.data = node_data.data.clone().into();
                node.executable = node_data.executable;
                node.scopes = node_data.scopes.clone();
            }

            nodes.push(CommandNode {
                executable: node.executable,
                data: node.data,
                scopes: node.scopes,
            });

            index_map.insert(*index, nodes.len() - 1);
        }

        let mut packet_nodes = Vec::new();

        for (index, children, redirect) in &nodes_to_be_allocated {
            let mut packet_children: Vec<VarInt> = Vec::new();

            for child in children {
                packet_children.push((index_map[&child] as i32).into())
            }

            let packet_redirect: Option<VarInt> =
                redirect.map(|towards| (index_map[&towards] as i32).into());

            packet_nodes.push(Node {
                children: packet_children,
                data: nodes[index_map[&index]].data.clone().into(),
                executable: nodes[index_map[&index]].executable,
                redirect_node: packet_redirect,
            });
        }

        // insert the children and the redirects
        CommandTreeS2c {
            commands: packet_nodes,
            root_index: root_index.into(),
        }
    }
}

/// ergonomic builder pattern for adding executables literals and arguments to a
/// command graph
///
/// * `T` - the type that should be constructed by an executable when the
///   command is executed
///
/// # Example
/// ```
/// use std::collections::HashMap;
/// use petgraph::visit::{EdgeCount, NodeCount};
/// use valence_command::arg_parser::CommandArg;
/// use valence_command::command_graph::{
///     CommandGraph, CommandGraphBuilder, Parser
/// };
/// use valence_command::{CommandArgSet, CommandRegistry};
///
/// struct TestCommand {
///    test: i32,
/// }
///
/// let mut command_graph = CommandRegistry::default();
/// let mut executable_map = HashMap::new();
/// let mut command_graph_builder = CommandGraphBuilder::<TestCommand>::new(&mut command_graph, &mut executable_map);
///
/// // simple command
/// let simple_command = command_graph_builder
///    .root() // transition to the root node
///    .literal("test") // add a literal node then transition to it
///    .argument("test")
///    // a player needs one of these scopes to execute the command
///    .with_scopes(vec!["test:admin", "command:test"])
///    .with_parser(Parser::Integer { min: None, max: None })
///    // it is reasonably safe to unwrap here because we know that the argument is an integer
///    .with_executable(|args| TestCommand { test: i32::parse_args(args).unwrap() })
///    .id();
///
/// // complex command (redirects back to the simple command)
/// command_graph_builder
///     .root()
///     .literal("test")
///     .literal("command")
///     .redirect_to(simple_command);
///
/// assert_eq!(command_graph.graph.graph.node_count(), 5); // root, test, command, <test>, test
/// // 5 edges, 2 for the simple command, 2 for the complex command and 1 for the redirect
/// assert_eq!(command_graph.graph.graph.edge_count(), 5);
/// ```
///
/// in this example we can execute either of the following commands for the same
/// result:
/// - `/test test 1`
/// - `/test command test 1`
///
/// the executables from these commands will both return a `TestCommand` with
/// the value `1`
pub struct CommandGraphBuilder<'a, T> {
    // We do not own the graph, we just have a mutable reference to it
    graph: &'a mut CommandGraph,
    executables: &'a mut HashMap<NodeIndex, fn(Vec<String>) -> T>,
    current_node: NodeIndex,
}

impl<'a, T> CommandGraphBuilder<'a, T> {
    /// creates a new command graph builder
    ///
    /// # Arguments
    /// * registry - the command registry to add the commands to
    /// * executables - the map of node indices to executable parser functions
    pub fn new(
        registry: &'a mut CommandRegistry,
        executables: &'a mut HashMap<NodeIndex, fn(Vec<String>) -> T>,
    ) -> Self {
        CommandGraphBuilder {
            current_node: registry.graph.root,
            graph: &mut registry.graph,
            executables,
        }
    }

    /// sets the current node to the root node. This is useful for when you want
    /// to add a new command to the graph in the same builder.
    pub fn root(&mut self) -> &mut Self {
        self.current_node = self.graph.root;
        self
    }

    /// creates a new literal node and transitions to it
    ///
    /// # Default Values
    /// * executable - `false`
    /// * scopes - `Vec::new()`
    pub fn literal(&mut self, literal: impl Into<String>) -> &mut Self {
        let graph = &mut self.graph.graph;
        let current_node = &mut self.current_node;

        let literal_node = graph.add_node(CommandNode {
            executable: false,
            data: NodeData::Literal {
                name: literal.into(),
            },
            scopes: Vec::new(),
        });

        graph.add_edge(*current_node, literal_node, CommandEdgeType::Child);

        *current_node = literal_node;

        self
    }

    /// creates a new argument node and transitions to it
    ///
    /// # Default Values
    /// * executable - `false`
    /// * scopes - `Vec::new()`
    /// * parser - `StringArg::SingleWord`
    /// * suggestion - `None`
    pub fn argument(&mut self, argument: impl Into<String>) -> &mut Self {
        let graph = &mut self.graph.graph;
        let current_node = &mut self.current_node;

        let argument_node = graph.add_node(CommandNode {
            executable: false,
            data: NodeData::Argument {
                name: argument.into(),
                parser: Parser::String(StringArg::SingleWord),
                suggestion: None,
            },
            scopes: Vec::new(),
        });

        graph.add_edge(*current_node, argument_node, CommandEdgeType::Child);

        *current_node = argument_node;

        self
    }

    /// creates a new redirect edge from the current node to the node specified
    ///
    /// # Example
    /// ```
    /// use std::collections::HashMap;
    ///
    /// use valence_command::command_graph::CommandGraphBuilder;
    /// use valence_command::CommandRegistry;
    ///
    /// struct TestCommand;
    ///
    /// let mut command_graph = CommandRegistry::default();
    /// let mut executable_map = HashMap::new();
    /// let mut command_graph_builder =
    ///     CommandGraphBuilder::<TestCommand>::new(&mut command_graph, &mut executable_map);
    ///
    /// let simple_command = command_graph_builder
    ///   .root() // transition to the root node
    ///   .literal("test") // add a literal node then transition to it
    ///   .id(); // get the id of the literal node
    ///
    /// command_graph_builder
    ///     .root() // transition to the root node
    ///     .literal("test") // add a literal node then transition to it
    ///     .literal("command") // add a literal node then transition to it
    ///     .redirect_to(simple_command); // redirect to the simple command
    /// ```
    pub fn redirect_to(&mut self, node: NodeIndex) -> &mut Self {
        let graph = &mut self.graph.graph;
        let current_node = &mut self.current_node;

        graph.add_edge(*current_node, node, CommandEdgeType::Redirect);

        *current_node = node;

        self
    }

    /// sets the executable flag on the current node to true and adds the
    /// executable to the map
    ///
    /// # Arguments
    /// * executable - the executable function to add
    ///
    /// # Example
    /// have a look at the example for [CommandGraphBuilder]
    pub fn with_executable(&mut self, executable: fn(Vec<String>) -> T) -> &mut Self {
        let graph = &mut self.graph.graph;
        let current_node = &mut self.current_node;

        let node = graph.node_weight_mut(*current_node).unwrap();

        node.executable = true;
        self.executables.insert(*current_node, executable);

        self
    }

    /// sets the required scopes for the current node
    ///
    /// # Arguments
    /// * scopes - a list of scopes for that are aloud to access a command node
    ///   and its children
    /// (list of strings following the system described in
    /// [command_scopes](crate::command_scopes))
    pub fn with_scopes(&mut self, scopes: Vec<impl Into<Scope>>) -> &mut Self {
        let graph = &mut self.graph.graph;
        let current_node = &mut self.current_node;

        let node = graph.node_weight_mut(*current_node).unwrap();

        node.scopes = scopes.into_iter().map(|s| s.into()).collect();

        self
    }

    /// sets the parser for the current node. This will decide how the argument
    /// is parsed client side and will be used to check the argument before
    /// it is passed to the executable. The node should be an argument node
    /// or nothing will happen.
    ///
    /// # Arguments
    /// * parser - the parser to use for the current node
    pub fn with_parser(&mut self, parser: Parser) -> &mut Self {
        let graph = &mut self.graph.graph;
        let current_node = &mut self.current_node;

        let node = graph.node_weight_mut(*current_node).unwrap();

        node.data = match node.data.clone() {
            NodeData::Argument {
                name, suggestion, ..
            } => NodeData::Argument {
                name,
                parser,
                suggestion,
            },
            NodeData::Literal { name } => NodeData::Literal { name },
            NodeData::Root => NodeData::Root,
        };

        self
    }

    /// transitions to the node specified
    pub fn at(&mut self, node: NodeIndex) -> &mut Self {
        self.current_node = node;
        self
    }

    /// gets the id of the current node (useful for commands that have multiple
    /// children)
    pub fn id(&self) -> NodeIndex {
        self.current_node
    }
}
