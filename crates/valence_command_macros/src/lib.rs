use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenTree};
use quote::{format_ident, quote};
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Error, Expr, Fields, Meta, Result};

#[proc_macro_derive(Command, attributes(command, scopes, paths))]
pub fn derive_command(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match command(input) {
        Ok(expansion) => expansion,
        Err(err) => err.to_compile_error().into(),
    }
}

fn command(input: DeriveInput) -> Result<TokenStream> {
    let input_name = input.ident;

    let outer_scopes = input
        .attrs
        .iter()
        .find_map(|attr| get_lit_list_attr(attr, "scopes"))
        .unwrap_or_default();

    match input.data {
        Data::Enum(data_enum) => {
            let Some(mut alias_paths) = input.attrs.iter().find_map(parse_path) else {
                return Err(Error::new_spanned(
                    input_name,
                    "No paths attribute found for command enum",
                ));
            };

            let base_path = alias_paths.remove(0);

            let fields = &data_enum.variants;
            let mut paths = Vec::new();

            for variant in fields {
                for attr in &variant.attrs {
                    if let Some(attr_paths) = parse_path(attr) {
                        paths.push((attr_paths, variant.fields.clone(), variant.ident.clone()));
                    }
                }
            }

            let mut expanded_nodes = Vec::new();

            for (paths, fields, variant_ident) in paths {
                expanded_nodes.push({
                    let processed = process_paths_enum(
                        &input_name,
                        paths,
                        &fields,
                        variant_ident.clone(),
                        true,
                        outer_scopes.clone(),
                    );
                    quote! { #processed; }
                });
            }

            let base_command_expansion = {
                let processed = process_paths_enum(
                    &input_name,
                    vec![base_path],
                    &Fields::Unit,
                    format_ident!("{}Root", input_name), // this is more of placeholder
                    // (should never be used)
                    false,
                    outer_scopes.clone(),
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
                    let processed = process_paths_enum(
                        &input_name,
                        vec![path],
                        &Fields::Unit,
                        format_ident!("{}Root", input_name),
                        false,
                        outer_scopes.clone(),
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

            let _new_struct = format_ident!("{}Command", input_name);

            Ok(TokenStream::from(quote! {

                impl valence::command::Command for #input_name {
                    fn assemble_graph(command_graph: &mut valence::command::graph::CommandGraphBuilder<Self>) {
                        use valence::command::parsers::CommandArg;
                        #base_command_expansion

                        #command_alias_expansion

                        #(#expanded_nodes)*
                    }
                }
            }))
        }
        Data::Struct(x) => {
            let mut paths = Vec::new();

            for attr in &input.attrs {
                if let Some(attr_paths) = parse_path(attr) {
                    paths.push(attr_paths);
                }
            }

            let mut expanded_nodes = Vec::new();

            for path in paths {
                expanded_nodes.push({
                    let mut processed =
                        process_paths_struct(&input_name, path, &x.fields, outer_scopes.clone());
                    // add scopes

                    if !outer_scopes.is_empty() {
                        processed = quote! {
                            #processed
                                .with_scopes(vec![#(#outer_scopes),*])
                        }
                    }

                    quote! { #processed; }
                });
            }

            Ok(TokenStream::from(quote! {

                impl valence::command::Command for #input_name {
                    fn assemble_graph(command_graph: &mut valence::command::graph::CommandGraphBuilder<Self>) {
                        use valence::command::parsers::CommandArg;
                        #(#expanded_nodes)*
                    }
                }
            }))
        }
        Data::Union(x) => Err(Error::new_spanned(
            x.union_token,
            "Command enum must be an enum, not a union",
        )),
    }
}

fn process_paths_enum(
    enum_name: &Ident,
    paths: Vec<(Vec<CommandArg>, bool)>,
    fields: &Fields,
    variant_ident: Ident,
    executables: bool,
    outer_scopes: Vec<String>,
) -> proc_macro2::TokenStream {
    let mut inner_expansion = quote! {};
    let mut first = true;

    for path in paths {
        if !first {
            inner_expansion = if executables && !path.1 {
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
            inner_expansion = if executables && !path.1 {
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

        let path = path.0;

        let mut final_executable = Vec::new();
        for (i, arg) in path.iter().enumerate() {
            match arg {
                CommandArg::Literal(lit) => {
                    inner_expansion = quote! {
                        #inner_expansion.literal(#lit)
                    };
                    if !outer_scopes.is_empty() {
                        inner_expansion = quote! {
                            #inner_expansion.with_scopes(vec![#(#outer_scopes),*])
                        }
                    }
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
                        syn::Type::Path(type_path) => {
                            let path = &type_path.path;
                            if path.segments.len() != 1 {
                                return Error::new_spanned(
                                    path,
                                    "Option type must be a single path segment",
                                )
                                .into_compile_error();
                            }
                            let segment = &path.segments.first().unwrap();
                            if segment.ident != "Option" {
                                return Error::new_spanned(
                                    &segment.ident,
                                    "Option type must be a option",
                                )
                                .into_compile_error();
                            }
                            match &segment.arguments {
                                syn::PathArguments::AngleBracketed(angle_bracketed) => {
                                    if angle_bracketed.args.len() != 1 {
                                        return Error::new_spanned(
                                            angle_bracketed,
                                            "Option type must have a single generic argument",
                                        )
                                        .into_compile_error();
                                    }
                                    match angle_bracketed.args.first().unwrap() {
                                        syn::GenericArgument::Type(generic_type) => generic_type,
                                        _ => {
                                            return Error::new_spanned(
                                                angle_bracketed,
                                                "Option type must have a single generic argument",
                                            )
                                            .into_compile_error();
                                        }
                                    }
                                }
                                _ => {
                                    return Error::new_spanned(
                                        segment,
                                        "Option type must have a single generic argument",
                                    )
                                    .into_compile_error();
                                }
                            }
                        }
                        _ => {
                            return Error::new_spanned(
                                field_type,
                                "Option type must be a single path segment",
                            )
                            .into_compile_error();
                        }
                    };

                    let ident_string = ident.to_string();

                    // find the ident of all following optional args
                    let mut next_optional_args = Vec::new();
                    for next_arg in path.iter().skip(i + 1) {
                        match next_arg {
                            CommandArg::Optional(ident) => next_optional_args.push(ident),
                            _ => {
                                return Error::new_spanned(
                                    variant_ident,
                                    "Only optional args can follow an optional arg",
                                )
                                .into_compile_error();
                            }
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

fn process_paths_struct(
    struct_name: &Ident,
    paths: Vec<(Vec<CommandArg>, bool)>,
    fields: &Fields,
    outer_scopes: Vec<String>,
) -> proc_macro2::TokenStream {
    let mut inner_expansion = quote! {};
    let mut first = true;

    for path in paths {
        if !first {
            inner_expansion = quote! {
                #inner_expansion;

                command_graph.root()
            };
        } else {
            inner_expansion = quote! {
                command_graph.root()
            };

            first = false;
        }

        let path = path.0;

        let mut final_executable = Vec::new();
        let mut path_first = true;
        for (i, arg) in path.iter().enumerate() {
            match arg {
                CommandArg::Literal(lit) => {
                    inner_expansion = quote! {
                        #inner_expansion.literal(#lit)

                    };
                    if i == path.len() - 1 {
                        inner_expansion = quote! {
                            #inner_expansion
                                .with_executable(|s| #struct_name{#(#final_executable,)*})
                        };
                    }

                    if path_first {
                        inner_expansion = quote! {
                            #inner_expansion.with_scopes(vec![#(#outer_scopes),*])
                        };
                        path_first = false;
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
                                    #struct_name {
                                        #(#final_executable,)*
                                    }
                                })
                        };
                    }


                    if path_first {


                        if !outer_scopes.is_empty() {
                            inner_expansion = quote! {
                                #inner_expansion.with_scopes(vec![#(#outer_scopes),*])
                            };
                        }
                        path_first = false;
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
                        syn::Type::Path(type_path) => {
                            let path = &type_path.path;
                            if path.segments.len() != 1 {
                                return Error::new_spanned(
                                    path,
                                    "Option type must be a single path segment",
                                )
                                .into_compile_error();
                            }
                            let segment = &path.segments.first().unwrap();
                            if segment.ident != "Option" {
                                return Error::new_spanned(
                                    &segment.ident,
                                    "Option type must be a option",
                                )
                                .into_compile_error();
                            }
                            match &segment.arguments {
                                syn::PathArguments::AngleBracketed(angle_bracketed) => {
                                    if angle_bracketed.args.len() != 1 {
                                        return Error::new_spanned(
                                            angle_bracketed,
                                            "Option type must have a single generic argument",
                                        )
                                        .into_compile_error();
                                    }
                                    match angle_bracketed.args.first().unwrap() {
                                        syn::GenericArgument::Type(generic_type) => generic_type,
                                        _ => {
                                            return Error::new_spanned(
                                                angle_bracketed,
                                                "Option type must have a single generic argument",
                                            )
                                            .into_compile_error();
                                        }
                                    }
                                }
                                _ => {
                                    return Error::new_spanned(
                                        segment,
                                        "Option type must have a single generic argument",
                                    )
                                    .into_compile_error();
                                }
                            }
                        }
                        _ => {
                            return Error::new_spanned(
                                field_type,
                                "Option type must be a single path segment",
                            )
                            .into_compile_error();
                        }
                    };

                    let ident_string = ident.to_string();

                    // find the ident of all following optional args
                    let mut next_optional_args = Vec::new();
                    for next_arg in path.iter().skip(i + 1) {
                        match next_arg {
                            CommandArg::Optional(ident) => next_optional_args.push(ident),
                            _ => {
                                return Error::new_spanned(
                                    struct_name,
                                    "Only optional args can follow an optional arg",
                                )
                                .into_compile_error();
                            }
                        }
                    }

                    inner_expansion = quote! {
                        let #so_far_ident = {#inner_expansion
                            .with_executable(|s| {
                                #struct_name {
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
                                    #struct_name {
                                        #(#final_executable,)*
                                    }
                                })
                        };
                    }

                    if path_first {
                        inner_expansion = quote! {
                            #inner_expansion
                                .with_scopes(vec![#(#outer_scopes),*])
                        };
                        path_first = false;
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
fn parse_path(path: &Attribute) -> Option<Vec<(Vec<CommandArg>, bool)>> {
    let path_strings: Vec<String> = get_lit_list_attr(path, "paths")?;

    let mut paths = Vec::new();
    // we now have the path as a string eg "strawberry {0?}"
    // the first word is a literal
    // the next word is an optional arg with the index 0
    for path_str in path_strings {
        let mut args = Vec::new();
        let at_root = path_str.starts_with("{/}");

        for word in path_str.split_whitespace().skip(usize::from(at_root)) {
            if word.starts_with('{') && word.ends_with('}') {
                if word.ends_with("?}") {
                    args.push(CommandArg::Optional(format_ident!(
                        "{}",
                        word[1..word.len() - 2].to_owned()
                    )));
                } else {
                    args.push(CommandArg::Required(format_ident!(
                        "{}",
                        word[1..word.len() - 1].to_owned()
                    )));
                }
            } else {
                args.push(CommandArg::Literal(word.to_owned()));
            }
        }
        paths.push((args, at_root));
    }

    Some(paths)
}

fn get_lit_list_attr(attr: &Attribute, ident: &str) -> Option<Vec<String>> {
    match &attr.meta {
        Meta::NameValue(key_value) => {
            if !key_value.path.is_ident(ident) {
                return None;
            }

            match &key_value.value {
                Expr::Lit(lit) => match &lit.lit {
                    syn::Lit::Str(lit_str) => Some(vec![lit_str.value()]),
                    _ => None,
                },
                _ => None,
            }
        }
        Meta::List(list) => {
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
                                .to_owned(),
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
        Meta::Path(_) => None,
    }
}
