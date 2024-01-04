// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

mod generate;
mod parse;
mod process;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

use generate::generate;
use process::process;

use self::parse::parse_input;

/// Implements the ConfigSource derive macro
///
/// # Arguments:
///
/// - `ts`: The token stream input
pub fn config_source(ts: TokenStream) -> TokenStream {
    // Parse token stream into input.
    let input: DeriveInput = parse_macro_input!(ts);

    // Parse input into Struct data.
    let data = parse_input(input);

    // Process the Struct data.
    let processed_data = process(data);

    // Generate the output code.
    generate(processed_data).into()
}
