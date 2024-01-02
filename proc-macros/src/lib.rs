// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

mod config_source;
use config_source::expand_struct_to_source;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Implements Config Source
#[proc_macro_derive(ConfigSource)]
pub fn config_source(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);

    expand_struct_to_source(input).into()
}
