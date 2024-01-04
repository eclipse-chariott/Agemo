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
///
/// # Examples
///
/// Given a Struct:
///
///     #[[derive(ConfigSource)]]
///     pub struct CmdOptions {
///         pub endpoint: String,
///         pub log_level: Option<String>,
///     }
///
/// The `ConfigSource` derive macro will implement `config::Source`:
///
///     impl config::Source for CmdOptions {
///
///         fn clone_into_box(&self) -> Box<dyn config::Source + Send + Sync> {
///             Box::new((*self).clone())
///         }
///
///         fn collect(&self) -> Result<config::Map<String, config::Value>, config::ConfigError> {
///             let mut entries: config::Map::<String, Option<config::Value>> = config::Map::from([
///                 (
///                     String::from("endpoint"),
///                     (&self.endpoint).clone().map(|v| config::Value::from(v))
///                 ),
///                 (
///                     String::from("log_level"),
///                     Some(config::Value::from((&self.log_level).clone()))
///                 ),
///             ]);
///
///             entries.retain(|_, v| v.is_some());
///             let entries_w_values = entries.clone();
///             let valid_entries: config::Map::<String, config::Value> = entries_w_values.iter().map(|(k, v)| (k.clone(), v.clone().unwrap())).collect();
///
///             Ok(valid_entries.clone())
///     }
/// }
///
/// This allows for Structs to be used as a `Source` for configuration through the `config` crate.
#[proc_macro_derive(ConfigSource)]
pub fn config_source(ts: TokenStream) -> TokenStream {
    config_source::config_source(ts)
}
