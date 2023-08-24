//! Scope graph for the Valence Command system.
//!
//! ## Breakdown
//! Each scope is a node in the graph. a path from one node to another indicates
//! that the first scope implies the second. A colon in the scope name indicates
//! a sub-scope. you can use this to create a hierarchy of scopes. for example,
//! the scope "valence:command" implies "valence:command:tp". this means that if
//! a player has the "valence:command" scope, they can use the "tp" command.
//!
//! You may also link scopes together in the registry. this is useful for admin
//! scope umbrellas. for example, if the scope "valence:admin" is linked to
//! "valence:command:teleport", It means that if a player has the
//! "valence:admin" scope, they can use the "teleport" command.
//!
//! # Example
//! ```
//! use valence_command::command_scopes::{CommandScopeRegistry, Scope};
//!
//! let mut registry = CommandScopeRegistry::new();
//!
//! // add a scope to the registry
//! registry.add_scope("valence:command:teleport");
//!
//! // we added 4 scopes to the registry. "valence", "valence:command", "valence:command:teleport",
//! // and the root scope.
//! assert_eq!(registry.scope_count(), 4);
//!
//! registry.add_scope("valence:admin");
//!
//! // add a scope to the registry with a link to another scope
//! registry.link("valence:admin", "valence:command:teleport");
//!
//! // the "valence:admin" scope implies the "valence:command:teleport" scope
//! assert_eq!(
//!     registry.grants("valence:admin", "valence:command:teleport"),
//!     true
//! );
//! ```

use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};

use bevy_ecs::prelude::Component;
use bevy_ecs::system::Resource;
use petgraph::dot;
use petgraph::dot::Dot;
use petgraph::prelude::*;

/// Store the scope graph and provide methods for querying it.
#[derive(Clone, Resource)]
pub struct CommandScopeRegistry {
    graph: Graph<Scope, ()>,
    string_to_node: HashMap<String, NodeIndex>,
    root: NodeIndex,
}

impl Debug for CommandScopeRegistry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}",
            Dot::with_config(&self.graph, &[dot::Config::EdgeNoLabel])
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Scope {
    pub name: String,
}

impl Display for Scope {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.name)
    }
}

impl From<String> for Scope {
    fn from(name: String) -> Self {
        Self { name }
    }
}

impl From<&String> for Scope {
    fn from(name: &String) -> Self {
        Self { name: name.clone() }
    }
}

impl From<&str> for Scope {
    fn from(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl Scope {
    /// create a new scope from a the perm name
    pub fn new(name: String) -> Self {
        Self { name }
    }

    /// get the name of this scope
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Command scope Component for players. this is a list of scopes that a player
/// has. if a player has a scope, they can use any command that requires
/// that scope.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Component, Default)]
pub struct CommandScopes {
    pub scopes: Vec<Scope>,
}

impl CommandScopes {
    /// create a new scope component
    pub fn new() -> Self {
        Self::default()
    }

    /// add a scope to this component
    pub fn add(&mut self, scope: impl Into<Scope>) {
        self.scopes.push(scope.into());
    }

    /// remove a scope from this component
    pub fn remove(&mut self, scope: impl Into<Scope>) {
        let scope = scope.into();
        self.scopes.retain(|p| p != &scope);
    }
}

impl Default for CommandScopeRegistry {
    fn default() -> Self {
        let mut graph = Graph::new();
        let root = graph.add_node(Scope {
            name: "root".into(),
        });
        Self {
            graph,
            string_to_node: HashMap::new(),
            root,
        }
    }
}

impl CommandScopeRegistry {
    /// create a new scope registry
    pub fn new() -> Self {
        Self::default()
    }

    /// add a scope to the registry.
    ///
    /// # Example
    /// ```
    /// use valence_command::CommandScopeRegistry;
    ///
    /// let mut registry = CommandScopeRegistry::new();
    ///
    /// // creates two nodes: "valence" and "command" with an edge from "valence" to "command"
    /// registry.add_scope("valence:command");
    /// // creates one node: "valence:command:tp" with an edge from "valence:command" to
    /// // "valence:command:tp"
    /// registry.add_scope("valence:command:tp");
    ///
    /// // the root node is always present
    /// assert_eq!(registry.scope_count(), 4);
    /// ```
    pub fn add_scope(&mut self, scope: impl Into<Scope>) {
        let scope = scope.into().name;
        let mut current_node = self.root;
        let mut prefix = String::new();
        for part in scope.split(':') {
            let node = self
                .string_to_node
                .entry(prefix.clone() + part)
                .or_insert_with(|| {
                    let node = self.graph.add_node(Scope {
                        name: part.to_string(),
                    });
                    self.graph.add_edge(current_node, node, ());
                    node
                });
            current_node = *node;

            prefix = prefix + part + ":";
        }
    }

    /// remove a scope from the registry
    ///
    /// # Example
    /// ```
    /// use valence_command::CommandScopeRegistry;
    ///
    /// let mut registry = CommandScopeRegistry::new();
    ///
    /// registry.add_scope("valence:command");
    /// registry.add_scope("valence:command:tp");
    ///
    /// assert_eq!(registry.scope_count(), 4);
    ///
    /// registry.remove_scope("valence:command:tp");
    ///
    /// assert_eq!(registry.scope_count(), 3);
    /// ```
    pub fn remove_scope(&mut self, scope: impl Into<Scope>) {
        let scope = scope.into().name;
        if let Some(node) = self.string_to_node.remove(&*scope) {
            self.graph.remove_node(node);
        };
    }

    /// check if a scope grants another scope
    ///
    /// # Example
    /// ```
    /// use valence_command::CommandScopeRegistry;
    ///
    /// let mut registry = CommandScopeRegistry::new();
    ///
    /// registry.add_scope("valence:command");
    /// registry.add_scope("valence:command:tp");
    ///
    /// assert!(registry.grants("valence:command", "valence:command:tp")); // command implies tp
    /// assert!(!registry.grants("valence:command:tp", "valence:command")); // tp does not imply command
    /// ```
    pub fn grants(&self, scope: impl Into<Scope>, other: impl Into<Scope>) -> bool {
        let scope = scope.into().name;
        let other = other.into().name;

        if scope == other {
            return true;
        }

        let scope_idx = match self.string_to_node.get(&*scope) {
            None => {
                return false;
            }
            Some(idx) => *idx,
        };
        let other_idx = match self.string_to_node.get(&*other) {
            None => {
                return false;
            }
            Some(idx) => *idx,
        };

        if scope_idx == self.root {
            return true;
        }

        // if we can reach the other scope from the scope, then the scope
        // grants the other scope
        let mut dfs = Dfs::new(&self.graph, scope_idx);
        while let Some(node) = dfs.next(&self.graph) {
            if node == other_idx {
                return true;
            }
        }
        false
    }

    /// do any of the scopes in the list grant the other scope?
    ///
    /// # Example
    /// ```
    /// use valence_command::CommandScopeRegistry;
    ///
    /// let mut registry = CommandScopeRegistry::new();
    ///
    /// registry.add_scope("valence:command");
    /// registry.add_scope("valence:command:tp");
    /// registry.add_scope("valence:admin");
    ///
    /// assert!(registry.any_grants(
    ///     &vec!["valence:admin", "valence:command"],
    ///     "valence:command:tp"
    /// ));
    /// ```
    pub fn any_grants(&self, scopes: &Vec<impl Into<Scope> + Clone>, other: impl Into<Scope>) -> bool {
        let other = other.into();

        for scope in scopes {
            let scope = (*scope).clone().into();
            if self.grants(scope, other.clone()) {
                return true;
            }
        }
        false
    }

    /// Create a link between two scopes so that one implies the other.
    ///
    /// # Panics
    /// Panics if either scope does not exist.
    ///
    /// # Example
    /// ```
    /// use valence_command::CommandScopeRegistry;
    ///
    /// let mut registry = CommandScopeRegistry::new();
    ///
    /// registry.add_scope("valence:command");
    /// registry.add_scope("valence:command:tp");
    /// registry.add_scope("valence:admin");
    ///
    /// registry.link("valence:admin", "valence:command");
    ///
    /// assert!(registry.grants("valence:admin", "valence:command"));
    /// ```
    pub fn link(&mut self, scope: &str, other: &str) {
        let scope_idx = *self
            .string_to_node
            .get(scope)
            .unwrap_or_else(|| panic!("scope {} does not exist", scope));
        let other_idx = *self
            .string_to_node
            .get(other)
            .unwrap_or_else(|| panic!("scope {} does not exist", scope));

        self.graph.add_edge(scope_idx, other_idx, ());
    }

    /// get the number of scopes in the registry
    pub fn scope_count(&self) -> usize {
        self.graph.node_count()
    }
}
