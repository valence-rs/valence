#![macro_use]

use std::any::{Any, TypeId};
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use crate::protocol::node::*;
use crate::protocol::{BoundedInt, BoundedString, Node, VarInt};
use crate::protocol::packets::s2c;

pub trait ToCommand {
    fn to_command() -> Command;
}

pub struct Command {
    pub name: String,
    pub arguments: Vec<Argument>,
}

pub struct Arguments {

}

/*pub fn commands_to_nodes(commands: impl IntoIterator<Item = Command>) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut root_children = Vec::new();
    for command in commands {
        process_arg(&mut nodes, &mut command.arguments.clone().into_iter());
        let first_arg_id = nodes.len() - 1;
        root_children.push(VarInt(nodes.len() as i32));
        nodes.push(Node {
            children: vec![VarInt(first_arg_id as i32)],
            data: NodeData::Literal(Literal {
                name: command.name.into(),
            }),
            is_executable: true,
            redirect_node: None,
        });
    }

    nodes.push(Node {
        children: root_children,
        data: NodeData::Root,
        is_executable: false,
        redirect_node: None,
    });

    nodes
}*/

fn process_arg(
    nodes: &mut Vec<Node>,
    arguments: &mut impl Iterator<Item = Argument>,
) -> Vec<VarInt> {
    if let Some(current_argument) = arguments.next() {
        let node = Node {
            data: NodeData::Argument(current_argument),
            children: process_arg(nodes, arguments),
            is_executable: true,
            redirect_node: None,
        };
        let id = nodes.len();
        nodes.push(node);
        vec![VarInt(id as i32)]
    } else {
        Vec::new()
    }
}

pub trait ParserProperties<T> {
    fn properties() -> T;
}

impl<const MIN: i64, const MAX: i64> ParserProperties<BrigadierInteger>
    for BoundedInt<i32, { MIN }, { MAX }>
{
    fn properties() -> BrigadierInteger {
        BrigadierInteger {
            min: Some(MIN as i32),
            max: Some(MAX as i32),
        }
    }
}

impl ParserProperties<BrigadierInteger> for i32 {
    fn properties() -> BrigadierInteger {
        BrigadierInteger {
            min: None,
            max: None,
        }
    }
}

impl AddToCommands for i32 {
    fn add_to_commands(command_registry: &mut CommandRegistry) -> VarInt {
        let id = VarInt(command_registry.graph.len() as i32);
        command_registry.graph.push(Node {
            children: Vec::new(),
            data: NodeData::Argument(Argument {
                name: Default::default(),
                parser: Parser::BrigadierInteger(BrigadierInteger {
                    min: None,
                    max: None,
                }),
                suggestions_type: None
            }),
            is_executable: false,
            redirect_node: None,
        });
        /*match command_registry.cache.entry(TypeId::of::<Self>()) {
            Entry::Vacant(vac) => {
                vac.insert(id);
                command_registry.graph.push(Node {
                    children: Vec::new(),
                    data: NodeData::Argument(Argument {
                        name: Default::default(),
                        parser: Parser::BrigadierInteger(BrigadierInteger {
                            min: None,
                            max: None,
                        }),
                        suggestions_type: None
                    }),
                    is_executable: false,
                    redirect_node: None,
                });
            }
            Entry::Occupied(occ) => {
                command_registry.graph.push(Node {
                    children: Vec::new(),
                    data: NodeData::Argument(Argument {
                        name: Default::default(),
                        parser: Parser::BrigadierInteger(BrigadierInteger {
                            min: None,
                            max: None,
                        }),
                        suggestions_type: None
                    }),
                    is_executable: false,
                    redirect_node: Some(occ.get().clone()),
                });
            }
        };*/
        id
    }
}

#[derive(Debug)]
pub struct CommandRegistry {
    pub cache: BTreeMap<TypeId, VarInt>,
    pub graph: Vec<Node>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        CommandRegistry {
            cache: Default::default(),
            graph: vec![
                Node {
                    children: vec![],
                    data: NodeData::Root,
                    is_executable: false,
                    redirect_node: None
                }
            ],
        }
    }

    pub fn register_command<C: AddToCommands>(&mut self) {
        let id = C::add_to_commands(self);
        self.graph[0].children.push(id);
    }

    pub fn to_packet(&self) -> s2c::play::Commands {
        s2c::play::Commands {
            nodes: self.graph.clone(),
            root_index: VarInt(0),
        }
    }
}

pub trait AddToCommands {
    fn add_to_commands(commands: &mut CommandRegistry) -> VarInt;
}

macro_rules! if_typ_is_empty_expr {
    (, $t:expr, $f:expr) => {
        $t
    };
    ($typ:ty, $t:expr, $f:expr) => {
        $f
    };
}

#[macro_export]
macro_rules! def_command {
    (
        //$(#[$enum_attrs:meta])*
        struct $name:ident {
            $(
                //$(#[$field_attrs:meta])*
                $field:ident:$typ:ty
            ),* $(,)?
        }
    ) => {
        struct $name {
            $(
                //$(#[$field_attrs])*
                $field:$typ
            ),*
        }

        /*impl ToCommand for $name {
            fn to_command() -> Command {
                Command {
                    name: stringify!($name).into(),//name: stringify!($name).to_snake_case().into(),
                    arguments: vec![
                        $(Argument {
                            name: stringify!($field).to_string().into(),
                            parser: $(<$typ>::properties().into())?,
                            suggestions_type: None,
                        }),*
                    ]
                }
            }
        }*/

        impl $crate::commands::AddToCommands for $name {
            fn add_to_commands(command_registry: &mut CommandRegistry) -> $crate::protocol::VarInt {
                use std::collections::btree_map::Entry;
                use std::any::TypeId;
                use $crate::protocol::{*,node::*};

                let id = command_registry.graph.len();
                let entry = command_registry.cache.entry(TypeId::of::<Self>());
                match entry {
                    Entry::Occupied(occ) => {
                        command_registry.graph.push(Node {
                            children: Vec::new(),
                            data: NodeData::Literal(Literal{
                                name: BoundedString(String::new()), //just temporarily, parent sets name
                            }),
                            is_executable: false,
                            redirect_node: Some(occ.get().clone()),
                        });
                    }
                    Entry::Vacant(vac) => {
                        vac.insert(VarInt(id as i32));
                        command_registry.graph.push(Node {
                            children: Vec::new(),
                            data: NodeData::Literal(Literal{
                                name: stringify!($name).to_string().into(),
                            }),
                            is_executable: false,
                            redirect_node: None,
                        });
                        let mut current_id = id;
                        $({
                            let id = <$typ>::add_to_commands(command_registry).0 as usize;
                            *command_registry.graph[id].mut_name() = stringify!($field).to_string().into();
                            command_registry.graph[current_id].children.push(VarInt(id as i32));
                            current_id = id;
                        })*
                        //command_registry.graph[current_id].children.push(VarInt(id as i32));
                    }
                }
                VarInt(id as i32)
            }
        }
    };
    (
        //$(#[$enum_attrs:meta])*
        enum $name:ident {
            $(
                //$(#[$variant_attrs:meta])*
                $variant:ident$(($typ:ty))?
            ),* $(,)?
        }
    ) => {
        enum $name {
            $(
                //$(#[$variant_attrs])*
                $variant$(($typ))?
            ),*
        }

        /*impl ToCommand for $name {
            fn to_command() -> Command {
                Command {
                    name: stringify!($name).into(),//name: stringify!($name).to_snake_case().into(),
                    arguments: vec![
                        $(Argument {
                            name: stringify!($variant).to_string().into(),
                            parser: $(<$typ>::properties().into())?,
                            suggestions_type: None,
                        }),*
                    ]
                }
            }
        }*/

        macro_rules! if_typ_is_empty_expr {
            (, $t:expr, $f:expr) => {
                $t
            };
            ($condtyp:ty, $t:expr, $f:expr) => {
                $f
            };
        }

        impl $crate::commands::AddToCommands for $name {
            fn add_to_commands(command_registry: &mut CommandRegistry) -> $crate::protocol::VarInt {
                use std::collections::btree_map::Entry;
                use std::any::TypeId;
                use $crate::protocol::{*,node::*};

                let id = command_registry.graph.len();
                let entry = command_registry.cache.entry(TypeId::of::<Self>());
                match entry {
                    Entry::Occupied(occ) => {
                        command_registry.graph.push(Node {
                            children: Vec::new(),
                            data: NodeData::Literal(Literal{
                                name: stringify!($name).to_string().into(),
                            }),
                            is_executable: false,
                            redirect_node: Some(occ.get().clone()),
                        });
                    }
                    Entry::Vacant(vac) => {
                        let mut children = Vec::new();
                        vac.insert(VarInt(id as i32));
                        command_registry.graph.push(Node {
                            children: Vec::new(),
                            data: NodeData::Literal(Literal{
                                name: stringify!($name).to_string().into(),
                            }),
                            is_executable: false,
                            redirect_node: None,
                        });
                        $({
                            let id = if_typ_is_empty_expr!($($typ)?,
                                {
                                    let id = command_registry.graph.len();
                                    command_registry.graph.push(Node {
                                        children: vec![],
                                        data: NodeData::Literal(Literal{
                                            name: stringify!($variant).to_string().into(),
                                        }),
                                        is_executable: true,
                                        redirect_node: None,
                                    });
                                    id
                                },
                                {
                                    let id = $(<$typ>)?::add_to_commands(command_registry).0 as usize;
                                    *command_registry.graph[id].mut_name() = stringify!($variant).to_string().into();
                                    id
                                }
                            );
                            children.push(VarInt(id as i32));
                        })*
                        dbg!(&children);
                        command_registry.graph[id].children = children;
                    }
                }
                VarInt(id as i32)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::protocol::BoundedString;
    def_command! {
        struct Add2Numbers {
            number1: i32,
            number2: i32,
        }
    }

    def_command! {
        enum ComplexCommand {
            Help(Help),
            Kill,
            Stop,
        }
    }

    def_command! {
        enum Help {
            Kill,
            Stop,
        }
    }

    #[test]
    fn test_macro() {
        let mut commands = CommandRegistry::new();
        commands.register_command::<Add2Numbers>();
        panic!("{:#?}", commands);

       /* assert_eq!(

            commands_to_nodes(vec![Add2Numbers::to_command()]),
            vec![
                Node {
                    children: vec![],
                    data: Argument {
                        name: BoundedString("number2".into()),
                        parser: BrigadierInteger {
                            min: None,
                            max: None
                        }
                            .into(),
                        suggestions_type: None
                    }
                        .into(),
                    is_executable: true,
                    redirect_node: None
                },
                Node {
                    children: vec![VarInt(0)],
                    data: Argument {
                        name: BoundedString("number1".into()),
                        parser: BrigadierInteger {
                            min: None,
                            max: None
                        }
                            .into(),
                        suggestions_type: None
                    }
                        .into(),
                    is_executable: true,
                    redirect_node: None
                },
                Node {
                    children: vec![VarInt(1)],
                    data: Literal {
                        name: BoundedString("Add2Numbers".into())
                    }
                        .into(),
                    is_executable: true,
                    redirect_node: None
                },
                Node {
                    children: vec![VarInt(2)],
                    data: NodeData::Root,
                    is_executable: false,
                    redirect_node: None
                },
            ]
        )*/
    }
}
