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
