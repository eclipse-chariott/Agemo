<h1 align="center" style="font-weight: bold; margin-top: 20px; margin-bottom: 20px;">Pub Sub Service</h1>

<h3 align="center" style="font-weight: bold; margin-top: 20px; margin-bottom: 20px;">An in-vehicle dynamic pub sub service handler.</h3>

<p align="center">
    <a href="https://github.com/eclipse-chariott/Agemo/tags"><img alt="Version tag" src="https://img.shields.io/github/v/tag/eclipse-chariott/Agemo?label=version"></a>
    <a href="https://github.com/eclipse-chariott/Agemo/issues"><img alt="issues: N/A" src="https://img.shields.io/github/issues/eclipse-chariott/Agemo"></a>
    <a href="https://github.com/eclipse-chariott/Agemo/actions/workflows/rust-ci.yml"><img alt="build: N/A" src="https://img.shields.io/github/actions/workflow/status/eclipse-chariott/Agemo/rust-ci.yml"></a>
    <img src="https://img.shields.io/badge/status-maintained-green.svg" alt="status: maintained">
    <a href="https://github.com/eclipse-chariott/Agemo/blob/main/LICENSE"><img alt="license: MIT" src="https://img.shields.io/github/license/eclipse-chariott/Agemo"></a>
</p>

<p align="center">
  <a href="#getting-started">Getting Started</a> •
  <a href="#configuration-setup">Configuration Setup</a> •
  <a href="#running-the-service">Running the Service</a> •
  <a href="#running-in-a-container">Running in a Container</a>
</p>

</br>

## Introduction

The Pub Sub Service is a [gRPC](https://grpc.io) service that provides publish/subscribe
functionality for applications within the vehicle, including [Eclipse Ibeji](https://github.com/eclipse-ibeji/ibeji)
and [Eclipse Chariott](https://github.com/eclipse-chariott/chariott). The service can register with
Chariott, making it easily discoverable by other applications. The service provides the ability to
dynamically create and manage topics. Additionally, the service is designed to allow for the
replacement of the default messaging broker as long as the broker meets certain requirements (see
[Bring Your Own Broker](./docs/README.md#bring-your-own-broker)).

## Getting Started

### Prerequisites

This guide uses `apt` as the package manager in the examples. You may need to substitute your own
package manager in place of `apt` when going through these steps.

1. Install gcc:

    ```shell
    sudo apt install gcc
    ```

    > **NOTE**: Rust needs gcc's linker.

1. Install cmake:

    ```shell
    sudo apt install cmake
    ```

1. Install git and rust:

    ```shell
    sudo apt update
    sudo apt install -y git snapd
    sudo snap install rustup --classic
    ```

    > **NOTE**: The rust toolchain version is managed by the rust-toolchain.toml file, so once you
                install rustup there is no need to manually install a toolchain or set a default.

1. Install OpenSSL:

    ```shell
    sudo apt install pkg-config
    sudo apt install libssl-dev
    ```

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
git clone --recurse-submodules https://github.com/eclipse-chariott/Agemo
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

## Configuration Setup

There are two template files that must be created in `target/debug` and filled out before the
service can be run. Below is the minimal set of configuration needed to start the service:

### Constants Configuration File

[constants_settings.yaml](./pub-sub-service/template/constants_settings.yaml)

```yaml
#
# Constants Configuration
#

### Communication Constants

# Pub Sub Service topic deletion message.
topic_deletion_message: "TOPIC DELETED"

# Constant for gRPC kind.
grpc_kind: "grpc+proto"

# Constant for mqtt kind.
mqtt_v5_kind: "mqtt_v5"

# Constant for the Pub Sub service API reference.
pub_sub_reference: "pubsub.v1.pubsub.proto"

# Retry interval for connections.
retry_interval_secs: 5

###
```

>**NOTE**: For most use cases, this file doesn't need to be modified and can be copied as-is from
           the `/template` directory to the `/target/debug` directory.

### Pub Sub Service Configuration File

[pub_sub_service_settings.yaml](./pub-sub-service/template/pub_sub_service_settings.yaml)

```yaml
#
# Pub Sub Service Settings
#

# The IP address and port number that the Pub Sub Service listens on for requests.
# Example: "0.0.0.0:80"
pub_sub_authority: "0.0.0.0:50051"

# The URI of the messaging service used to facilitate publish and subscribe functionality.
# Example: "mqtt://0.0.0.0:1883"
messaging_uri: "mqtt://0.0.0.0:1883"

# The URI that the Chariott Service listens on for requests.
# Example: "http://0.0.0.0:4243"
# chariott_uri: <<value>>

# The namespace of the Pub Sub Service.
# Example: "sdv.pubsub"
# namespace: <<value>>

# The name of the Pub Sub Service.
# Example: "dynamic.pubsub"
# name: <<value>>

# The version of the Pub Sub Service.
# This is gathered from the cargo.toml file, but can be overwritten here if uncommented.
# Example: "0.1.0"
# version: <<value>>
```

> **NOTE**: The commented out configuration settings enable Chariott communication within the
            Pub Sub Service. See
            [Running With Chariott](./pub-sub-service/README.md#running-with-chariott) for more
            information.

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
cargo run -p pub-sub-service
```

### Interacting with the service

The service implements two gRPC methods defined in [pubsub.proto](./proto/pubsub/v1/pubsub.proto).
To create a topic, execute the below command in another terminal window:

```shell
grpcurl -proto ./proto/pubsub/v1/pubsub.proto -plaintext -d @ [::1]:50051 pubsub.PubSub/CreateTopic <<EOF
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
  "brokerUri": "mqtt://0.0.0.0:1883",
  "brokerProtocol": "mqtt"
}
```

> **NOTE**: The service provides the generated topic name and the broker information to directly
            connect to.

This created topic could then be deleted with the following command:

```shell
grpcurl -proto ./proto/pubsub/v1/pubsub.proto -plaintext -d @ [::1]:50051 pubsub.PubSub/DeleteTopic <<EOF
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
see more full featured examples in
[Running the Simple Samples](./samples/README.md#running-the-simple-samples).

## Running in a Container

See below for instructions on how to run the service in a container. Currently, there is support
for both Docker and Podman containers. Both variations expect that the other steps have been
followed above to configure the service and start the MQTT broker.

### Docker

#### Prequisites

Install Docker: [Docker Installation](https://docs.docker.com/engine/install/)

#### Running in Docker

To run the service in a Docker container:

1. Copy the [docker.env](./container/template/docker.env) template from the
[container](./container/) directory into the project root directory. This file should already be
set up with out any modification needed.

1. Run the following command in the project root directory to build the docker container from the
Dockerfile:

    ```shell
    docker build -t pub_sub_service -f Dockerfile .
    ```

1. Once the container has been built, start the container in interactive mode with the following
command in the project root directory:

    ```shell
    docker run --name pub_sub_service -p 50051:50051 --env-file=docker.env --add-host=host.docker.internal:host-gateway -it --rm pub_sub_service
    ```

1. To detach from the container, enter:

    ```shell
    Ctrl-p Ctrl-q
    ```

1. To stop the container, enter:

    ```shell
    docker stop pub_sub_service
    ```

### Podman

#### Prequisites

Install Podman: [Podman Installation](https://podman.io/docs/installation)

#### Running in Podman

To run the service in a Podman container:

1. Copy the [podman.env](./container/template/podman.env) template from the
[container](./container/) directory into the project root directory. This file should already be
set up with out any modification needed.

1. Run the following command in the project root directory to build the podman container from the
Dockerfile:

    ```shell
    podman build -t pub_sub_service:latest -f Dockerfile .
    ```

1. Once the container has been built, start the container with the following command in the project
root directory:

    ```shell
    podman run -p 50051:50051 --env-file=podman.env --network=slirp4netns:allow_host_loopback=true localhost/pub_sub_service
    ```

1. To stop the container, find the container with:

    ```shell
    podman ps
    ```

    Then run:

    ```shell
    podman stop <container_name>
    ```

#### Notes

1. By default, podman does not recognize docker images for dockerfile. To fix this, one can add the
`docker.io` registry to `/etc/containers/registries.conf` by changing the following field:

    ```conf
    unqualified-search-registries = ["docker.io"]
    ```

## Trademarks

This project may contain trademarks or logos for projects, products, or services. Authorized use of Microsoft
trademarks or logos is subject to and must follow
[Microsoft's Trademark & Brand Guidelines](https://www.microsoft.com/en-us/legal/intellectualproperty/trademarks/usage/general).
Use of Microsoft trademarks or logos in modified versions of this project must not cause confusion or imply Microsoft sponsorship.
Any use of third-party trademarks or logos are subject to those third-party's policies.
