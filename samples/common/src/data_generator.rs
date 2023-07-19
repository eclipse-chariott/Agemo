// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Simple module that generates data for examples.

use std::time::SystemTime;

/// Example data as int.
pub fn get_data() -> i64 {
    let val = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards");
    val.as_millis() as i64
}
