// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

pub mod pubsub {
    pub mod v1 {
        tonic::include_proto!("pubsub");
    }
}

pub mod publisher {
    pub mod v1 {
        tonic::include_proto!("publisher");
    }
}

pub mod sample_publisher {
    pub mod v1 {
        tonic::include_proto!("sample_publisher");
    }
}

pub mod service_registry {
    pub mod v1 {
        tonic::include_proto!("service_registry");
    }
}
