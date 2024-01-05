// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use proc_macro2::Ident;
use syn::{punctuated::Punctuated, token::Comma, Field};
use syn::{Data, DataStruct, DeriveInput, Fields, Generics};

/// Represents a Struct.
pub(crate) struct StructData {
    /// The identifier of the Struct.
    pub struct_name: Ident,
    /// List of fields of the Struct.
    pub struct_fields: Punctuated<Field, Comma>,
    /// The generics associated with the Struct.
    pub struct_generics: Generics,
}

/// Parse input data for the ConfigSource derive macro.
/// Will panic if input is not gathered from a Struct.
///
/// # Arguments
/// * `input` - Parsed derive macro input.
pub(crate) fn parse_input(input: DeriveInput) -> StructData {
    let struct_name = input.ident;
    let struct_generics = input.generics;

    // Processes input data into Struct Fields. Panics if data is not from a Struct.
    let struct_fields = match input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => fields.named,
        _ => panic!("this derive macro only works on structs with named fields"),
    };

    StructData {
        struct_name,
        struct_fields,
        struct_generics,
    }
}

#[cfg(test)]
mod config_source_parse_tests {
    use quote::quote;
    use std::panic::catch_unwind;

    use super::*;

    #[test]
    fn can_parse_struct() {
        let struct_tok = quote! {
            pub struct Foo {
                pub bar: String,
                pub baz: Option<String>,
            }
        };

        // Parses token stream into DeriveInput for test.
        let derive_input = syn::parse2::<DeriveInput>(struct_tok).unwrap();

        let output = parse_input(derive_input.clone());

        assert_eq!(output.struct_name, derive_input.ident);
        assert_eq!(output.struct_generics, derive_input.generics);
    }

    #[test]
    fn parse_panics_with_non_struct_type() {
        let enum_tok = quote! {
            pub enum Foo {
                Bar(String),
                Baz(Option<String>),
            }
        };

        // Parses token stream into DeriveInput for test.
        let derive_input = syn::parse2::<DeriveInput>(enum_tok).unwrap();

        let result = catch_unwind(|| parse_input(derive_input));
        assert!(result.is_err());
    }

    #[test]
    fn parse_panics_with_non_named_fields() {
        let unit_struct_tok = quote! {
            pub struct Foo;
        };

        // Parses token stream into DeriveInput for test.
        let derive_input = syn::parse2::<DeriveInput>(unit_struct_tok).unwrap();

        let result = catch_unwind(|| parse_input(derive_input));
        assert!(result.is_err());
    }
}
