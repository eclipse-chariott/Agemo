# Pub Sub Service

- [Introduction](#introduction)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Cloning the Repo](#cloning-the-repo)
  - [Building](#building)
  - [Running the Tests](#running-the-tests)
- [Running the Service](#running-the-service)
  - [Start the messaging broker](#start-the-messaging-broker)
  - [Start the Pub Sub Service](#start-the-pub-sub-service)
  - [Interacting with the service](#interacting-with-the-service)
- [Trademarks](#trademarks)

## Introduction

The Pub Sub Service is a [gRPC](https://grpc.io) service that provides publish/subscribe
functionality for applications within the vehicle, including [Eclipse Ibeji](https://github.com/eclipse-ibeji/ibeji)
and [Eclipse Chariott](https://github.com/eclipse-chariott/chariott). The service can register with
Chariott, making it easily discoverable by other applications. The service allows for integration
of a different messaging broker that meets certain
[requirements](./docs/README.md#bring-your-own-broker). The other feature that
the service provides is dynamic topic management capabilities.

## Getting Started

### Prerequisites

This guide uses `apt` as the package manager in the examples. You may need to substitute your own
package manager in place of `apt` when going through these steps.

1. Install gcc:

    ```shell
    sudo apt install gcc
    ```

    > **NOTE**: Rust needs gcc's linker.

1. Install git and rust:

    ```shell
    sudo apt update
    sudo apt install -y git snapd
    sudo snap install rustup --classic
    ```

    > **NOTE**: The rust toolchain version is managed by the rust-toolchain.toml file, so once you
                install rustup there is no need to manually install a toolchain or set a default.

1. Install Protobuf Compiler:

    ```shell
    sudo apt install -y protobuf-compiler
    ```

    > **NOTE**: The protobuf compiler is needed for building the project.

1. Install the default messaging broker:

    A messaging broker is required to use this service, and currently the service was developed
    with the [Mosquitto](https://github.com/eclipse/mosquitto) MQTT messaging broker, please see
    this [section](./pub-sub-service/README.md#messaging-broker-requirements) for more information
    on how to integrate a different broker.

    To install the broker, refer to <https://mosquitto.org/download/>.

### Cloning the Repo

The repo has a submodule [chariott](https://github.com/eclipse-chariott/chariott) that provides
proto files for Chariott integration. To ensure that these files are included, please use the
following command when cloning the repo:

```shell
git clone --recurse-submodules https://github.com/eclipse-chariott/pub_sub_service
```

### Building

Run the following in the enlistment's root directory to build everything in the workspace once you
have installed the prerequisites:

```shell
cargo build
```

### Running the Tests

After successfully building the service, you can run all of the unit tests. To do this go to the
enlistment's root directory and run:

```shell
cargo test
```

## Running the Service

Below are the steps to run the Pub Sub Service in its most simple form. The service is gRPC based,
and the quickest way to interact with the services is through the use of the
[grpcurl](http://github.com/fullstorydev/grpcurl) command line tool.

### Start the messaging broker

The messaging broker must be started first, this can be done with the following command from the
enlistment's root in a terminal window:

```shell
mosquitto -c ./pub-sub-service/src/connectors/mosquitto.conf
```

### Start the Pub Sub Service

Then start up the Pub Sub Service project with the following command from the enlistment's root
in a separate terminal window:

```shell
cargo run --bin pub-sub-service
```

### Interacting with the service

The service implements two gRPC methods defined in [pubsub.proto](./proto/pubsub.proto). To create
a topic, execute the below command in another terminal window:

```shell
grpcurl -proto ./proto/pubsub.proto -plaintext -d @ [::1]:50051 pubsub.PubSub/CreateTopic <<EOF
{
  "publisherId": "simple_publisher_call",
  "managementCallback": "https://example_management.address",
  "managementProtocol": "grpc"
}
EOF
```

An example of an expected response would look like:

```shell
{
  "generatedTopic": "09285f6c-9a86-49db-9159-0d91f8f4d3bb",
  "brokerEndpoint": "mqtt://localhost:1883",
  "brokerProtocol": "mqtt"
}
```

> **NOTE**: The service provides the generated topic name and the broker information to directly
            connect to.

This created topic could then be deleted with the following command:

```shell
grpcurl -proto ./proto/pubsub.proto -plaintext -d @ [::1]:50051 pubsub.PubSub/DeleteTopic <<EOF
{
  "topic": "09285f6c-9a86-49db-9159-0d91f8f4d3bb"
}
EOF
```

The expected response is an empty set of brackets:

```shell
{

}
```

These two methods are used by a publisher to dynamically manage a topic. Please refer to this
[documentation](./docs/README.md) for more information on how to the service is utilized. You can
see more full featured examples [here](./samples/README.md#running-the-simple-samples).

## Trademarks

This project may contain trademarks or logos for projects, products, or services. Authorized use of Microsoft
trademarks or logos is subject to and must follow
[Microsoft's Trademark & Brand Guidelines](https://www.microsoft.com/en-us/legal/intellectualproperty/trademarks/usage/general).
Use of Microsoft trademarks or logos in modified versions of this project must not cause confusion or imply Microsoft sponsorship.
Any use of third-party trademarks or logos are subject to those third-party's policies.
