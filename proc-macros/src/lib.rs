// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

mod config_source;

use proc_macro::TokenStream;

/// Derives `config::Source` Trait (from the config crate) for a Struct.
///
/// Note: The Struct must have named fields and the Type for each field must be convertable into a
/// `config::Value` from the `config` crate.
///
/// # Arguments
/// * `ts`: A token stream.
#[proc_macro_derive(ConfigSource)]
pub fn config_source(ts: TokenStream) -> TokenStream {
    config_source::config_source(ts)
}
