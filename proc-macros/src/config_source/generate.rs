// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use super::process::StructDataOutput;

/// Generate code for the ConfigSource derive macro.
///
/// # Arguments
/// * `struct_data` - Data gathered from a Struct.
pub(crate) fn generate(struct_data: StructDataOutput) -> TokenStream {
    // Define values for the code generation.
    let struct_name = struct_data.struct_name;
    let struct_entries = struct_data.struct_fields;

    // Define generics information for the code generation.
    let (impl_generics, type_generics, where_clause) = struct_data.struct_generics.split_for_impl();

    // Construct a list of entries from the fields of the Struct.
    let entries = struct_entries.into_iter().map(|entry| {
        let field_name = entry.name;
        let field_name_str = entry.name_str;

        // Code snippet changes based on whether the entry is an optional field.
        if entry.is_optional {
            quote! {
                (String::from(#field_name_str), (&self.#field_name).clone().map(|v| config::Value::from(v))),
            }
        } else {
            quote! {
                (String::from(#field_name_str), Some(config::Value::from((&self.#field_name).clone()))),
            }
        }
    });

    // Construct a code snippet that implements the `Source` Trait.
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
