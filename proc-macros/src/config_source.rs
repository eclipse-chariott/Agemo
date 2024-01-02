use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataStruct, Fields};
use syn::{DeriveInput, GenericArgument, Path, PathArguments, Type};

pub fn expand_struct_to_source(input: DeriveInput) -> TokenStream {
    let struct_name = input.ident;
    let fields = match input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => fields.named,
        _ => panic!("this derive macro only works on structs with named fields"),
    };

    let entries = fields.into_iter().map(|field| {
        let field_name = field.ident;
        let field_name_str = field_name.clone().unwrap().to_string();
        let field_type = field.ty;

        // Need to check if the value is an option and if so get the inner value if it is Some
        match option_inner_type(&field_type) {
            None => {
                quote! {
                    (String::from(#field_name_str), Some(Value::from((&self.#field_name).clone()))),
                }
            },
            Some(Some(_type)) => {
                quote! {
                    (String::from(#field_name_str), (&self.#field_name).clone().map(|v| Value::from(v))),
                }
            },
            _ => {
                quote! {
                    (String::from(#field_name_str), None),
                }
            }
        }
    });

    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        #[automatically_derived]
        impl #impl_generics config::Source #type_generics for #struct_name #where_clause {
            fn clone_into_box(&self) -> Box<dyn config::Source + Send + Sync> {
                Box::new((*self).clone())
            }

            fn collect(&self) -> Result<config::Map<String, config::Value>, config::ConfigError> {
                let mut entries: config::Map::<String, Option<config::Value>> = config::Map::from([#(#entries)*]);

                entries.retain(|_, v| v.is_some());
                let entries_w_values = entries.clone();
                let valid_entries: config::Map::<String, config::Value> = entries_w_values.iter().map(|(k, v)| (k.clone(), v.clone().unwrap())).collect();

                Ok(valid_entries.clone())
            }
        }
    }
}

// Checks if path is option
fn path_is_option(path: &Path) -> bool {
    path.leading_colon.is_none() && path.segments.len() == 1 && path.segments[0].ident == "Option"
}

// Gets the option argument within the angle brackets.
fn option_arg(path_args: &PathArguments) -> Option<&Type> {
    match path_args {
        PathArguments::AngleBracketed(bracket) => {
            if bracket.args.len() == 1 {
                return match &bracket.args[0] {
                    GenericArgument::Type(t) => Some(t),
                    _ => None,
                };
            }
            None
        }
        _ => None,
    }
}

// Gets the inner option type if the type is option.
fn option_inner_type(ty: &Type) -> Option<Option<&Type>> {
    match ty {
        Type::Path(typepath) if typepath.qself.is_none() && path_is_option(&typepath.path) => {
            let path_args = &typepath.path.segments[0].arguments;

            Some(option_arg(path_args))
        }

        _ => None,
    }
}
