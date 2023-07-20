// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use std::{error::Error, path::Path};

use tonic_build::configure;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../proto/pubsub/v1/pubsub.proto")?;
    tonic_build::compile_protos("../proto/publisher/v1/publisher.proto")?;
    compile_external_protos(
        "../external/chariott/service_discovery/proto",
        "../external/chariott/service_discovery/proto/core/v1/service_registry.proto",
    )?;

    Ok(())
}

fn compile_external_protos(folder_path: &str, file_path: &str) -> Result<(), Box<dyn Error>> {
    configure().compile(&[Path::new(file_path)], &[Path::new(folder_path)])?;

    Ok(())
}
