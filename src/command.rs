#![macro_use]

use crate::protocol::node::*;
use crate::protocol::{BoundedInt, Node, VarInt};

pub trait ToCommand {
    fn to_command() -> Command;
}

pub struct Command {
    pub name: String,
    pub arguments: Vec<Argument>,
}

pub fn commands_to_nodes(commands: impl IntoIterator<Item = Command>) -> Vec<Node> {
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
}

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

macro_rules! def_command {
    (
        //$(#[$enum_attrs:meta])*
        $name:ident {
            $(
                //$(#[$variant_attrs:meta])*
                $field:ident$(:$typ:ty)?
            ),* $(,)?
        }
    ) => {
        struct $name {

        }

        impl ToCommand for $name {
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
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::protocol::BoundedString;
    def_command! {
        Add2Numbers {
            number1: i32,
            number2: i32,
        }
    }

    #[test]
    fn test_macro() {
        assert_eq!(
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
        )
    }
}
