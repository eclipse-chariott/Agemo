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
            let field_name_str = field_name.to_string();
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

#[cfg(test)]
mod config_source_process_tests {
    use quote::format_ident;
    use std::panic::catch_unwind;
    use syn::{parse_quote, punctuated::Punctuated, token::Comma, Field, TypePath};

    use crate::config_source::process::path_is_option;

    use super::*;

    #[test]
    fn path_is_option_type() {
        let option_string: TypePath = parse_quote!(Option<String>);
        assert!(path_is_option(&option_string.path));

        let option_u64: TypePath = parse_quote!(Option<u64>);
        assert!(path_is_option(&option_u64.path));

        let option_bool: TypePath = parse_quote!(Option<bool>);
        assert!(path_is_option(&option_bool.path));
    }

    #[test]
    fn path_is_not_option_type() {
        let string_type: TypePath = parse_quote!(String);
        assert!(!path_is_option(&string_type.path));

        let u64_type: TypePath = parse_quote!(u64);
        assert!(!path_is_option(&u64_type.path));

        let bool_type: TypePath = parse_quote!(bool);
        assert!(!path_is_option(&bool_type.path));
    }

    #[test]
    fn type_is_option() {
        let option_string_type: Type = parse_quote!(Option<String>);
        assert!(is_option(&option_string_type));

        let option_u64_type: Type = parse_quote!(Option<u64>);
        assert!(is_option(&option_u64_type));

        let option_bool_type: Type = parse_quote!(Option<bool>);
        assert!(is_option(&option_bool_type));
    }

    #[test]
    fn type_is_not_option() {
        let string_type: Type = parse_quote!(String);
        assert!(!is_option(&string_type));

        let u64_type: Type = parse_quote!(u64);
        assert!(!is_option(&u64_type));

        let bool_type: Type = parse_quote!(bool);
        assert!(!is_option(&bool_type));
    }

    #[test]
    fn can_process_struct_data_with_optional_fields() {
        let struct_name = format_ident!("Foo");
        let struct_generics = Generics::default();

        let field_a: Field = parse_quote!(field_a: Option<String>);
        let field_b: Field = parse_quote!(field_b: Option<u64>);
        let field_c: Field = parse_quote!(field_c: Option<bool>);

        // Create Punctuated list for input data.
        let mut fields = Punctuated::<Field, Comma>::new();
        fields.push_value(field_a.clone());
        fields.push_punct(Comma::default());
        fields.push_value(field_b.clone());
        fields.push_punct(Comma::default());
        fields.push_value(field_c.clone());
        fields.push_punct(Comma::default());

        let struct_data = StructData {
            struct_name: struct_name.clone(),
            struct_fields: fields,
            struct_generics: struct_generics.clone(),
        };

        let output = process(struct_data);

        assert_eq!(output.struct_name, struct_name);
        assert_eq!(output.struct_generics, struct_generics);
        assert_eq!(output.struct_fields.len(), 3);

        // Check that each of the fields is present.
        let mut field_iter = output.struct_fields.into_iter();
        let expected_field_a_name = field_a.ident.expect("Field_A ident should be present.");
        let expected_field_b_name = field_b.ident.expect("Field_B ident should be present.");
        let expected_field_c_name = field_c.ident.expect("Field_C ident should be present.");

        assert!(
            field_iter.any(|field| {
                field.name.eq(&expected_field_a_name)
                    && field.name_str.eq(&expected_field_a_name.to_string())
                    && field.is_optional
            }),
            "expected Field_A did not match processed Field_A"
        );

        assert!(
            field_iter.any(|field| {
                field.name.eq(&expected_field_b_name)
                    && field.name_str.eq(&expected_field_b_name.to_string())
                    && field.is_optional
            }),
            "expected Field_B did not match processed Field_B"
        );

        assert!(
            field_iter.any(|field| {
                field.name.eq(&expected_field_c_name)
                    && field.name_str.eq(&expected_field_c_name.to_string())
                    && field.is_optional
            }),
            "expected Field_C did not match processed Field_C"
        );
    }

    #[test]
    fn can_process_struct_data_with_non_optional_fields() {
        let struct_name = format_ident!("Foo");
        let struct_generics = Generics::default();

        let field_a: Field = parse_quote!(field_a: String);
        let field_b: Field = parse_quote!(field_b: u64);
        let field_c: Field = parse_quote!(field_c: bool);

        // Create Punctuated list for input data.
        let mut fields = Punctuated::<Field, Comma>::new();
        fields.push_value(field_a.clone());
        fields.push_punct(Comma::default());
        fields.push_value(field_b.clone());
        fields.push_punct(Comma::default());
        fields.push_value(field_c.clone());
        fields.push_punct(Comma::default());

        let struct_data = StructData {
            struct_name: struct_name.clone(),
            struct_fields: fields,
            struct_generics: struct_generics.clone(),
        };

        let output = process(struct_data);

        assert_eq!(output.struct_name, struct_name);
        assert_eq!(output.struct_generics, struct_generics);
        assert_eq!(output.struct_fields.len(), 3);

        // Check that each of the fields is present.
        let mut field_iter = output.struct_fields.into_iter();
        let expected_field_a_name = field_a.ident.expect("Field_A ident should be present.");
        let expected_field_b_name = field_b.ident.expect("Field_B ident should be present.");
        let expected_field_c_name = field_c.ident.expect("Field_C ident should be present.");

        assert!(
            field_iter.any(|field| {
                field.name.eq(&expected_field_a_name)
                    && field.name_str.eq(&expected_field_a_name.to_string())
                    && !field.is_optional
            }),
            "expected Field_A did not match processed Field_A"
        );

        assert!(
            field_iter.any(|field| {
                field.name.eq(&expected_field_b_name)
                    && field.name_str.eq(&expected_field_b_name.to_string())
                    && !field.is_optional
            }),
            "expected Field_B did not match processed Field_B"
        );

        assert!(
            field_iter.any(|field| {
                field.name.eq(&expected_field_c_name)
                    && field.name_str.eq(&expected_field_c_name.to_string())
                    && !field.is_optional
            }),
            "expected Field_C did not match processed Field_C"
        );
    }

    #[test]
    fn can_process_struct_data_with_mixed_fields() {
        let struct_name = format_ident!("Foo");
        let struct_generics = Generics::default();

        let field_a: Field = parse_quote!(field_a: String);
        let field_b: Field = parse_quote!(field_b: Option<u64>);
        let field_c: Field = parse_quote!(field_c: bool);

        // Create Punctuated list for input data.
        let mut fields = Punctuated::<Field, Comma>::new();
        fields.push_value(field_a.clone());
        fields.push_punct(Comma::default());
        fields.push_value(field_b.clone());
        fields.push_punct(Comma::default());
        fields.push_value(field_c.clone());
        fields.push_punct(Comma::default());

        let struct_data = StructData {
            struct_name: struct_name.clone(),
            struct_fields: fields,
            struct_generics: struct_generics.clone(),
        };

        let output = process(struct_data);

        assert_eq!(output.struct_name, struct_name);
        assert_eq!(output.struct_generics, struct_generics);
        assert_eq!(output.struct_fields.len(), 3);

        // Check that each of the fields is present.
        let mut field_iter = output.struct_fields.into_iter();
        let expected_field_a_name = field_a.ident.expect("Field_A ident should be present.");
        let expected_field_b_name = field_b.ident.expect("Field_B ident should be present.");
        let expected_field_c_name = field_c.ident.expect("Field_C ident should be present.");

        // Is a non optional field.
        assert!(
            field_iter.any(|field| {
                field.name.eq(&expected_field_a_name)
                    && field.name_str.eq(&expected_field_a_name.to_string())
                    && !field.is_optional
            }),
            "expected Field_A did not match processed Field_A"
        );

        // Is an optional field.
        assert!(
            field_iter.any(|field| {
                field.name.eq(&expected_field_b_name)
                    && field.name_str.eq(&expected_field_b_name.to_string())
                    && field.is_optional
            }),
            "expected Field_B did not match processed Field_B"
        );

        // Is a non optional field.
        assert!(
            field_iter.any(|field| {
                field.name.eq(&expected_field_c_name)
                    && field.name_str.eq(&expected_field_c_name.to_string())
                    && !field.is_optional
            }),
            "expected Field_C did not match processed Field_C"
        );
    }

    #[test]
    fn panic_with_malformed_field_data() {
        let struct_name = format_ident!("Foo");
        let struct_generics = Generics::default();

        let field_a: Field = parse_quote!(field_a: String);
        // Malformed Field entry with no name.
        let field_b: Field = parse_quote!(Option<u64>);
        let field_c: Field = parse_quote!(field_c: bool);

        // Create Punctuated list for input data.
        let mut fields = Punctuated::<Field, Comma>::new();
        fields.push_value(field_a.clone());
        fields.push_punct(Comma::default());
        fields.push_value(field_b.clone());
        fields.push_punct(Comma::default());
        fields.push_value(field_c.clone());
        fields.push_punct(Comma::default());

        let struct_data = StructData {
            struct_name: struct_name.clone(),
            struct_fields: fields,
            struct_generics: struct_generics.clone(),
        };

        let result = catch_unwind(|| process(struct_data));
        assert!(result.is_err());
    }
}
