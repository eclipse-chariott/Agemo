// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use proc_macro2::Ident;
use syn::{Generics, Path, Type};

use super::parse::StructData;

/// Represents data gathered from a Struct.
pub(crate) struct StructDataOutput {
    /// The identifier of the Struct.
    pub struct_name: Ident,
    /// A vector of fields for the Struct.
    pub struct_fields: Vec<FieldEntry>,
    /// The generics associated with the Struct.
    pub struct_generics: Generics,
}

/// Represents a single named field in a Struct.
pub(crate) struct FieldEntry {
    /// The identifier of the field.
    pub name: Ident,
    /// The identifier of the field as a string.
    pub name_str: String,
    /// Whether the field is optional.
    pub is_optional: bool,
}

/// Process the data for the ConfigSource derive macro.
/// This method collects the relevent struct values for generation.
///
/// # Arguments
/// * `data` - Parsed Struct data.
pub(crate) fn process(data: StructData) -> StructDataOutput {
    // Process fields from Struct.
    let struct_fields: Vec<FieldEntry> = data
        .struct_fields
        .into_iter()
        .map(|field| {
            let field_name = field.ident.unwrap();
            // Get the field name as a string. Will be used as a key in the code generation step.
            let field_name_str = field_name.clone().to_string();
            // Determine if field is optional. Relevant for the code generation step.
            let is_optional = is_option(&field.ty);

            FieldEntry {
                name: field_name,
                name_str: field_name_str,
                is_optional,
            }
        })
        .collect();

    StructDataOutput {
        struct_name: data.struct_name,
        struct_fields,
        struct_generics: data.struct_generics,
    }
}

/// Helper method to determine if a Type is of type `Option`.
///
/// # Arguments
/// * `path` - Path to check.
fn path_is_option(path: &Path) -> bool {
    path.leading_colon.is_none() && path.segments.len() == 1 && path.segments[0].ident == "Option"
}

/// Determines if the provided Type is of type `Option`.
///
/// # Arguments
/// * `ty` - Struct field type to check.
fn is_option(ty: &Type) -> bool {
    matches!(ty, Type::Path(typepath) if typepath.qself.is_none() && path_is_option(&typepath.path))
}
