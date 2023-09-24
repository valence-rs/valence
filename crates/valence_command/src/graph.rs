//! # Command graph implementation
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
//! If you want a cool graph of your own command graph you can use the display
//! trait on the [CommandGraph] struct. Then you can use a tool like
//! [Graphviz Online](https://dreampuf.github.io/GraphvizOnline) to look at the graph.

use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use petgraph::dot::Dot;
use petgraph::prelude::*;
use valence_server::protocol::packets::play::command_tree_s2c::{
    Node, NodeData, Parser, StringArg,
};
use valence_server::protocol::packets::play::CommandTreeS2c;
use valence_server::protocol::VarInt;

use crate::modifier_value::ModifierValue;
use crate::parsers::{CommandArg, ParseInput};
use crate::{CommandRegistry, CommandScopeRegistry};

/// This struct is used to store the command graph. (see module level docs for
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
#[derive(Clone, Debug, PartialEq)]
pub struct CommandNode {
    pub executable: bool,
    pub data: NodeData,
    pub scopes: Vec<String>,
}

impl Display for CommandNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.data {
            NodeData::Root => write!(f, "Root"),
            NodeData::Literal { name } => write!(f, "Literal: {}", name),
            NodeData::Argument { name, .. } => write!(f, "Argument: <{}>", name),
        }
    }
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

impl From<CommandGraph> for CommandTreeS2c {
    fn from(command_graph: CommandGraph) -> Self {
        let graph = command_graph.graph;
        let nodes_and_edges = graph.into_nodes_edges();

        let mut nodes: Vec<Node> = nodes_and_edges
            .0
            .into_iter()
            .map(|node| Node {
                children: Vec::new(),
                data: node.weight.data,
                executable: node.weight.executable,
                redirect_node: None,
            })
            .collect();

        let edges = nodes_and_edges.1;

        for edge in edges {
            match edge.weight {
                CommandEdgeType::Child => {
                    nodes[edge.source().index()]
                        .children
                        .push(VarInt::from(edge.target().index() as i32));
                }
                CommandEdgeType::Redirect => {
                    nodes[edge.source().index()].redirect_node =
                        Some(VarInt::from(edge.target().index() as i32));
                }
            }
        }

        CommandTreeS2c {
            commands: nodes,
            root_index: VarInt::from(command_graph.root.index() as i32),
        }
    }
}

/// Ergonomic builder pattern for adding executables, literals and arguments to
/// a command graph. See the derive macro for a more ergonomic way of doing this
/// for a basic command with an enum.
///
/// # Type Parameters
/// * `T` - the type that should be constructed by an executable when the
///   command is executed
///
/// # Example
/// ```
/// use std::collections::HashMap;
/// use petgraph::visit::{EdgeCount, NodeCount};
/// use valence_command::graph::{
///     CommandGraph, CommandGraphBuilder
/// };
/// use valence_command::{CommandRegistry};
/// use valence_command::parsers::CommandArg;
///
/// struct TestCommand {
///    test: i32,
/// }
///
/// let mut command_graph = CommandRegistry::default();
/// let mut executable_map = HashMap::new();
/// let mut parser_map = HashMap::new();
/// let mut modifier_map = HashMap::new();
/// let mut command_graph_builder = CommandGraphBuilder::<TestCommand>::new(&mut command_graph, &mut executable_map, &mut parser_map, &mut modifier_map);
///
/// // simple command
/// let simple_command = command_graph_builder
///    .root() // transition to the root node
///    .literal("test") // add a literal node then transition to it
///    .argument("test")
///    // a player needs one of these scopes to execute the command
///    //(note: if you want an admin scope you should use the link method on the scope registry.)
///    .with_scopes(vec!["test:admin", "command:test"])
///    .with_parser::<i32>()
///    // it is reasonably safe to unwrap here because we know that the argument is an integer
///    .with_executable(|args| TestCommand { test: i32::parse_arg(args).unwrap() })
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
#[allow(clippy::type_complexity)]
pub struct CommandGraphBuilder<'a, T> {
    // We do not own the graph, we just have a mutable reference to it
    graph: &'a mut CommandGraph,
    current_node: NodeIndex,
    executables: &'a mut HashMap<NodeIndex, fn(&mut ParseInput) -> T>,
    parsers: &'a mut HashMap<NodeIndex, fn(&mut ParseInput) -> bool>,
    modifiers: &'a mut HashMap<NodeIndex, fn(String, &mut HashMap<ModifierValue, ModifierValue>)>,
    scopes_added: Vec<String>, /* we need to keep track of added scopes so we can add them to
                                * the registry later */
}

impl<'a, T> CommandGraphBuilder<'a, T> {
    /// Creates a new command graph builder
    ///
    /// # Arguments
    /// * registry - the command registry to add the commands to
    /// * executables - the map of node indices to executable parser functions
    #[allow(clippy::type_complexity)]
    pub fn new(
        registry: &'a mut CommandRegistry,
        executables: &'a mut HashMap<NodeIndex, fn(&mut ParseInput) -> T>,
        parsers: &'a mut HashMap<NodeIndex, fn(&mut ParseInput) -> bool>,
        modifiers: &'a mut HashMap<
            NodeIndex,
            fn(String, &mut HashMap<ModifierValue, ModifierValue>),
        >,
    ) -> Self {
        CommandGraphBuilder {
            current_node: registry.graph.root,
            graph: &mut registry.graph,
            executables,
            parsers,
            modifiers,
            scopes_added: Vec::new(),
        }
    }

    /// Transitions to the root node. Use this to start building a new command
    /// from root.
    pub fn root(&mut self) -> &mut Self {
        self.current_node = self.graph.root;
        self
    }

    /// Creates a new literal node and transitions to it.
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

    /// Creates a new argument node and transitions to it.
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

    /// Creates a new redirect edge from the current node to the node specified.
    /// For info on what a redirect edge is, see the module level documentation.
    ///
    /// # Example
    /// ```
    /// use std::collections::HashMap;
    ///
    /// use valence_command::graph::CommandGraphBuilder;
    /// use valence_command::CommandRegistry;
    ///
    /// struct TestCommand;
    ///
    /// let mut command_graph = CommandRegistry::default();
    /// let mut executable_map = HashMap::new();
    /// let mut parser_map = HashMap::new();
    /// let mut modifier_map = HashMap::new();
    /// let mut command_graph_builder = CommandGraphBuilder::<TestCommand>::new(
    ///     &mut command_graph,
    ///     &mut executable_map,
    ///     &mut parser_map,
    ///     &mut modifier_map,
    /// );
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

    /// Sets up the executable function for the current node. This function will
    /// be called when the command is executed and should parse the args and
    /// return the `T` type.
    ///
    /// # Arguments
    /// * executable - the executable function to add
    ///
    /// # Example
    /// have a look at the example for [CommandGraphBuilder]
    pub fn with_executable(&mut self, executable: fn(&mut ParseInput) -> T) -> &mut Self {
        let graph = &mut self.graph.graph;
        let current_node = &mut self.current_node;

        let node = graph.node_weight_mut(*current_node).unwrap();

        node.executable = true;
        self.executables.insert(*current_node, executable);

        self
    }

    /// Adds a modifier to the current node
    ///
    /// # Arguments
    /// * modifier - the modifier function to add
    ///
    /// # Example
    /// ```
    /// use std::collections::HashMap;
    ///
    /// use valence_command::graph::CommandGraphBuilder;
    /// use valence_command::CommandRegistry;
    ///
    /// struct TestCommand;
    ///
    /// let mut command_graph = CommandRegistry::default();
    /// let mut executable_map = HashMap::new();
    /// let mut parser_map = HashMap::new();
    /// let mut modifier_map = HashMap::new();
    /// let mut command_graph_builder =
    ///    CommandGraphBuilder::<TestCommand>::new(&mut command_graph, &mut executable_map, &mut parser_map, &mut modifier_map);
    ///
    /// command_graph_builder
    ///     .root() // transition to the root node
    ///     .literal("test") // add a literal node then transition to it
    ///     .with_modifier(|_, modifiers| {
    ///        modifiers.insert("test".into(), "test".into()); // this will trigger when the node is passed
    ///     })
    ///     .literal("command") // add a literal node then transition to it
    ///     .with_executable(|_| TestCommand);
    /// ```
    pub fn with_modifier(
        &mut self,
        modifier: fn(String, &mut HashMap<ModifierValue, ModifierValue>),
    ) -> &mut Self {
        let current_node = &mut self.current_node;

        self.modifiers.insert(*current_node, modifier);

        self
    }

    /// Sets the required scopes for the current node
    ///
    /// # Arguments
    /// * scopes - a list of scopes for that are aloud to access a command node
    ///   and its children (list of strings following the system described in
    ///   [command_scopes](crate::scopes))
    pub fn with_scopes(&mut self, scopes: Vec<impl Into<String>>) -> &mut Self {
        let graph = &mut self.graph.graph;
        let current_node = &mut self.current_node;

        let node = graph.node_weight_mut(*current_node).unwrap();

        node.scopes = scopes.into_iter().map(|s| s.into()).collect();
        self.scopes_added.extend(node.scopes.clone());

        self
    }

    /// Applies the scopes to the registry
    ///
    /// # Arguments
    /// * registry - the registry to apply the scopes to
    pub fn apply_scopes(&mut self, registry: &mut CommandScopeRegistry) -> &mut Self {
        for scope in self.scopes_added.clone() {
            registry.add_scope(scope);
        }
        self.scopes_added.clear();
        self
    }

    /// Sets the parser for the current node. This will decide how the argument
    /// is parsed client side and will be used to check the argument before
    /// it is passed to the executable. The node should be an argument node
    /// or nothing will happen.
    ///
    /// # Type Parameters
    /// * `P` - the parser to use for the current node (must be [CommandArg])
    pub fn with_parser<P: CommandArg>(&mut self) -> &mut Self {
        let graph = &mut self.graph.graph;
        let current_node = self.current_node;

        let node = graph.node_weight_mut(current_node).unwrap();
        self.parsers
            .insert(current_node, |input| P::parse_arg(input).is_ok());

        let parser = P::display();

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

    /// Transitions to the node specified.
    pub fn at(&mut self, node: NodeIndex) -> &mut Self {
        self.current_node = node;
        self
    }

    /// Gets the id of the current node (useful for commands that have multiple
    /// children).
    pub fn id(&self) -> NodeIndex {
        self.current_node
    }
}
