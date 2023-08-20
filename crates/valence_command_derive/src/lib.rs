use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenTree};
use quote::{format_ident, quote};

use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Expr, Fields, Meta,
};

#[proc_macro_derive(Command, attributes(command, scopes, paths))]
pub fn derive_command(a_input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(a_input as DeriveInput);

    let enum_name = input.ident;

    let mut alias_paths = input
        .attrs
        .iter()
        .filter_map(parse_path)// get_command_attr returns Option<(String, Vec<String>)>
        .next()// there should only be one command name
        .expect("Command names not provided");

    let base_path = alias_paths.remove(0);

    let outer_scopes = input
        .attrs
        .iter()
        .filter_map(|attr| get_lit_list_attr(attr, "scopes"))
        .next()
        .unwrap_or(Vec::new());

    let fields = match input.data {
        Data::Enum(ref data_enum) => &data_enum.variants,
        _ => panic!("Command must be an enum"),
    };

    let mut paths = Vec::new();
    // let mut expanded_variants = Vec::new();
    for variant in fields {
        for attr in variant.attrs.iter() {
            if let Some(attr_paths) = parse_path(attr) {
                paths.push((attr_paths, variant.fields.clone(), variant.ident.clone()));
            }
        }
    }

    let mut expanded_nodes = Vec::new();

    for (paths, fields, variant_ident) in paths {
        expanded_nodes.push({
            let processed = process_paths(&enum_name, paths, &fields, variant_ident.clone(), true);
            quote! { #processed; }
        });
    }

    let base_command_expansion = {
        let processed = process_paths(
            &enum_name,
            vec![base_path],
            &Fields::Unit,
            format_ident!("{}Root", enum_name),
            false,
        ); // this will error if the base path has args
        let mut expanded_main_command = quote! {
            let command_root_node = #processed
        };

        if !outer_scopes.is_empty() {
            expanded_main_command = quote! {
                #expanded_main_command
                    .with_scopes(vec![#(#outer_scopes),*])
            }
        }

        quote! {
            #expanded_main_command.id();
        }
    };

    let command_alias_expansion = {
        let mut alias_expansion = quote! {};
        for path in alias_paths {
            let processed = process_paths(
                &enum_name,
                vec![path],
                &Fields::Unit,
                format_ident!("{}Root", enum_name),
                false,
            );

            alias_expansion = quote! {
                #alias_expansion

                #processed
                    .redirect_to(command_root_node)
            };

            if !outer_scopes.is_empty() {
                alias_expansion = quote! {
                    #alias_expansion
                        .with_scopes(vec![#(#outer_scopes),*])
                }
            }

            alias_expansion = quote! {
                #alias_expansion;
            }
        }

        alias_expansion
    };

    let _new_struct = format_ident!("{}Command", enum_name);

    let expanded = quote! {

        impl valence_command::Command for #enum_name {
            fn assemble_graph(command_graph: &mut valence_command::command_graph::CommandGraphBuilder<Self>) {
                #base_command_expansion

                #command_alias_expansion

                #(#expanded_nodes)*
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

fn process_paths(
    enum_name: &Ident,
    paths: Vec<Vec<CommandArg>>,
    fields: &Fields,
    variant_ident: Ident,
    executables: bool,
) -> proc_macro2::TokenStream {
    let mut inner_expansion = quote! {};
    let mut first = true;

    for path in paths {
        if !first {
            inner_expansion = if executables {
                quote! {
                        #inner_expansion;

                        command_graph.at(command_root_node)
                }
            } else {
                quote! {
                    #inner_expansion;

                    command_graph.root()
                }
            };
        } else {
            inner_expansion = if executables {
                quote! {
                    command_graph.at(command_root_node)
                }
            } else {
                quote! {
                    command_graph.root()
                }
            };

            first = false;
        }

        let mut final_executable = Vec::new();
        for (i, arg) in path.iter().enumerate() {
            match arg {
                CommandArg::Literal(lit) => {
                    inner_expansion = quote! {
                        #inner_expansion.literal(#lit)

                    };
                    if executables && i == path.len() - 1 {
                        inner_expansion = quote! {
                            #inner_expansion
                                .with_executable(|s| #enum_name::#variant_ident{#(#final_executable,)*})
                        };
                    }
                }
                CommandArg::Required(ident) => {
                    let field_type = &fields
                        .iter()
                        .find(|field| field.ident.as_ref().unwrap() == ident)
                        .expect("Required arg not found")
                        .ty;
                    let ident_string = ident.to_string();

                    inner_expansion = quote! {
                        #inner_expansion
                            .argument(#ident_string)
                            .with_parser::<#field_type>()
                    };

                    final_executable.push(quote! {
                        #ident: #field_type::parse_arg(s).unwrap()
                    });

                    if i == path.len() - 1 {
                        inner_expansion = quote! {
                            #inner_expansion
                                .with_executable(|s| {
                                    #enum_name::#variant_ident {
                                        #(#final_executable,)*
                                    }
                                })
                        };
                    }
                }
                CommandArg::Optional(ident) => {
                    let field_type = &fields
                        .iter()
                        .find(|field| field.ident.as_ref().unwrap() == ident)
                        .expect("Optional arg not found")
                        .ty;
                    let so_far_ident = format_ident!("graph_til_{}", ident);

                    // get what is inside the Option<...>
                    let option_inner = match field_type {
                        syn::Type::Path(ref type_path) => {
                            let path = &type_path.path;
                            if path.segments.len() != 1 {
                                panic!("Option type must be a single path segment");
                            }
                            let segment = &path.segments.first().unwrap();
                            if segment.ident != "Option" {
                                panic!("Must be an Option type");
                            }
                            match &segment.arguments {
                                syn::PathArguments::AngleBracketed(ref angle_bracketed) => {
                                    if angle_bracketed.args.len() != 1 {
                                        panic!("Option type must have a single generic argument");
                                    }
                                    match angle_bracketed.args.first().unwrap() {
                                        syn::GenericArgument::Type(ref generic_type) => {
                                            generic_type
                                        }
                                        _ => panic!(
                                            "Option type must have a single generic argument"
                                        ),
                                    }
                                }
                                _ => panic!("Option type must have a single generic argument"),
                            }
                        }
                        _ => panic!("Option type must be a single path segment"),
                    };

                    let ident_string = ident.to_string();

                    // find the ident of all following optional args
                    let mut next_optional_args = Vec::new();
                    for next_arg in path.iter().skip(i + 1) {
                        match next_arg {
                            CommandArg::Optional(ident) => next_optional_args.push(ident),
                            _ => panic!(
                                "Only optional args can follow an optional arg, found {:?}",
                                next_arg
                            ),
                        }
                    }

                    inner_expansion = quote! {
                        let #so_far_ident = {#inner_expansion
                            .with_executable(|s| {
                                #enum_name::#variant_ident {
                                    #(#final_executable,)*
                                    #ident: None,
                                    #(#next_optional_args: None,)*
                                }
                            })
                            .id()};

                        command_graph.at(#so_far_ident)
                            .argument(#ident_string)
                            .with_parser::<#option_inner>()
                    };

                    final_executable.push(quote! {
                        #ident: Some(#option_inner::parse_arg(s).unwrap())
                    });

                    if i == path.len() - 1 {
                        inner_expansion = quote! {
                            #inner_expansion
                                .with_executable(|s| {
                                    #enum_name::#variant_ident {
                                        #(#final_executable,)*
                                    }
                                })
                        };
                    }
                }
            }
        }
    }
    quote!(#inner_expansion)
}

#[derive(Debug)]
enum CommandArg {
    Required(Ident),
    Optional(Ident),
    Literal(String),
}

// example input: #[paths = "strawberry {0?}"]
// example output: [CommandArg::Literal("Strawberry"), CommandArg::Optional(0)]
fn parse_path(path: &Attribute) -> Option<Vec<Vec<CommandArg>>> {
    let path_strings: Vec<String> = get_lit_list_attr(path, "paths")?;

    let mut paths = Vec::new();
    // we now have the path as a string eg "strawberry {0?}"
    // the first word is a literal
    // the next word is an optional arg with the index 0
    for path_str in path_strings {
        let mut args = Vec::new();
        for word in path_str.split_whitespace() {
            if word.starts_with('{') && word.ends_with('}') {
                if word.ends_with("?}") {
                    args.push(CommandArg::Optional(format_ident!(
                        "{}",
                        word[1..word.len() - 2].to_string()
                    )));
                } else {
                    args.push(CommandArg::Required(format_ident!(
                        "{}",
                        word[1..word.len() - 1].to_string()
                    )));
                }
            } else {
                args.push(CommandArg::Literal(word.to_string()));
            }
        }
        paths.push(args);
    }

    Some(paths)
}

fn get_lit_list_attr(attr: &Attribute, ident: &str) -> Option<Vec<String>> {
    match attr.meta {
        Meta::NameValue(ref key_value) => {
            if !key_value.path.is_ident(ident) {
                return None;
            }

            match key_value.value {
                Expr::Lit(ref lit) => match lit.lit {
                    syn::Lit::Str(ref lit_str) => Some(vec![lit_str.value()]),
                    _ => None,
                },
                _ => None,
            }
        }
        Meta::List(ref list) => {
            if !list.path.is_ident(ident) {
                return None;
            }

            let mut path_strings = Vec::new();
            // parse as array with strings
            let mut comma_next = false;
            for token in list.tokens.clone() {
                match token {
                    TokenTree::Literal(lit) => {
                        if comma_next {
                            return None;
                        }
                        let lit_str = lit.to_string();
                        path_strings.push(
                            lit_str
                                .strip_prefix('"')
                                .unwrap()
                                .strip_suffix('"')
                                .unwrap()
                                .to_string(),
                        );
                        comma_next = true;
                    }
                    TokenTree::Punct(punct) => {
                        if punct.as_char() != ',' || !comma_next {
                            return None;
                        }
                        comma_next = false;
                    }
                    _ => return None,
                }
            }
            Some(path_strings)
        }
        _ => None,
    }
}