## Containers

This repository provides several Dockerfiles to enable building of OCI container images. This
document has instructions for building and running the provided Dockerfiles in
[Docker](#docker-containers) and [Podman](#podman-containers). Refer to the
[Dockerfiles](#dockerfiles) section to select the appropriate Dockerfile.

### Dockerfiles

#### Pub Sub Service

- [Dockerfile.amd64](../Dockerfile.amd64) - Dockerfile used to build the `Pub Sub Service` for the
x86-64 architecture.
- [Dockerfile.arm64](../Dockerfile.arm64) - Dockerfile used to build the `Pub Sub Service` for the
aarch64 architecture.
- [Dockerfile.multi](../Dockerfile.multi) - Dockerfile used to build the `Pub Sub Service` for
multiple architectures based on the TARGETARCH argument.
- [Dockerfile_integrated.amd64](../Dockerfile_integrated.amd64) - Dockerfile used to build the
`Pub Sub Service` using
[Chariott Service Discovery](https://github.com/eclipse-chariott/chariott/blob/main/service_discovery/README.md)
with the [integrated configuration](../config/pub_sub_service_settings.integrated.yaml) for the
x86-64 architecture.
- [Dockerfile_integrated.arm64](../Dockerfile_integrated.arm64) - Dockerfile used to build the
`Pub Sub Service` using
[Chariott Service Discovery](https://github.com/eclipse-chariott/chariott/blob/main/service_discovery/README.md)
with the [integrated configuration](../config/pub_sub_service_settings.integrated.yaml) for the
aarch64 architecture.
- [Dockerfile_integrated.multi](../Dockerfile_integrated.multi) - Dockerfile used to build the
`Pub Sub Service` using
[Chariott Service Discovery](https://github.com/eclipse-chariott/chariott/blob/main/service_discovery/README.md)
with the [integrated configuration](../config/pub_sub_service_settings.integrated.yaml) for
multiple architectures based on the TARGETARCH argument.

#### Mosquitto MQTT Broker

- [Dockerfile.mosquitto.amd64](../Dockerfile.mosquitto.amd64) - Dockerfile used to build the
`Mosquitto MQTT Broker` with the appropriate configuration for the x86-64 architecture.
- [Dockerfile.mosquitto.arm64](../Dockerfile.mosquitto.arm64) - Dockerfile used to build the
`Mosquitto MQTT Broker` with the appropriate configuration for the aarch64 architecture.
- [Dockerfile.mosquitto.multi](../Dockerfile.mosquitto.multi) - Dockerfile used to build the
`Mosquitto MQTT Broker` with the appropriate configuration for multiple architectures based on the
TARGETARCH argument.

#### Sample Applications

- [Dockerfile.samples.amd64](../Dockerfile.samples.amd64) - Dockerfile used to build one of the
sample applications for the x86-64 architecture.
- [Dockerfile.samples.arm64](../Dockerfile.samples.arm64) - Dockerfile used to build one of the
sample applications for the aarch64 architecture.

>Note: The samples default configuration files are cloned from
[.agemo-samples/config](../.agemo-samples/config/), defined in the project's root.

### Docker Containers

#### Prequisites

[Install Docker](https://docs.docker.com/engine/install/)

#### Running in Docker

To run the service in a Docker container:

1. Run the following command in the project root directory to build the docker container from the
Dockerfile:

    ```shell
    docker build -t <image_name> -f <Dockerfile> (optional: --build-arg=APP_NAME=<project name>) .
    ```

    For example, to build an image for the `pub-sub-service` project:

    ```shell
    docker build -t pub_sub_service -f Dockerfile.amd64 .
    ```

    Or to build an image for the `chariott-publisher` sample for aarch64:

    ```shell
    docker build -t chariott_publisher -f Dockerfile.samples.arm64 --build-arg=APP_NAME=chariott-publisher .
    ```

    >Note: The build arg `APP_NAME` needs to be passed in for all sample applications to build the
    correct sample.

    Or to build a multi-platform image for `pub-sub-service` project and push it to a container
    registry:
    You must first create a new builder using the docker-container driver, which gives you access
    to more complex features like multi-platform build. See more information here:
    [multi-platform builds.](https://docs.docker.com/build/building/multi-platform/#cross-compilation)

    ```shell
    docker buildx create --name multibuilder --driver docker-container --use
    docker buildx build --platform=linux/amd64,linux/arm64 -f Dockerfile.multi -t <container_registry>/pub_sub_service_multi --push .
    ```

1. Once the container has been built, start the container in interactive mode with the following
command in the project root directory:

    ```shell
    docker run --name <container_name> --network=host -it --rm <image_name>
    ```

    For example, to run the `pub-sub-service` image built in step 1:

    ```shell
    docker run --name pub_sub_service --network=host -it --rm pub_sub_service
    ```

    >Note: A custom network is recommended when using a container for anything but testing.

1. To detach from the container, enter:

    <kbd>Ctrl</kbd> + <kbd>p</kbd>, <kbd>Ctrl</kbd> + <kbd>q</kbd>

1. To stop the container, enter:

    ```shell
    docker stop <container_name>
    ```

    For example, to stop the `pub_sub_service` container started in step 2:

    ```shell
    docker stop pub_sub_service
    ```

#### Running in Docker with overridden configuration

Follow the steps in [Running in Docker](#running-in-docker) to build the container.

1. To run the container with overridden configuration, create your config file and set an
environment variable called CONFIG_HOME to the path to the config file:

    ```shell
    export CONFIG_HOME={path to directory containing config file}
    ```

1. Then run the container with the following command:

    ```shell
    docker run -v ${CONFIG_HOME}:/mnt/config --name <container_name> --network=host -it --rm <image_name>
    ```

    For example, to run the `pub_sub_service` image with overridden configuration:

    ```shell
    docker run -v ${CONFIG_HOME}:/mnt/config --name pub_sub_service --network=host -it --rm pub_sub_service
    ```

### Podman Containers

#### Prequisites

[Install Podman](https://podman.io/docs/installation)

#### Running in Podman

To run the service in a Podman container:

1. Run the following command in the project root directory to build the podman container from the
Dockerfile:

    ```shell
    podman build -t <image_name> -f <Dockerfile> .
    ```

    For example, to build an image for the `pub-sub-service` project:

    ```shell
    podman build -t pub_sub_service -f Dockerfile.amd64 .
    ```

    Or to build an image for the `chariott-publisher` sample for aarch64:

    ```shell
    podman build -t chariott_publisher -f Dockerfile.samples.arm64 --build-arg=APP_NAME=chariott-publisher .
    ```

    >Note: The build arg `APP_NAME` needs to be passed in for all sample applications to build the
    correct sample.

1. Once the container has been built, start the container with the following command in the project
root directory:

    ```shell
    podman run --network=host <image_name>
    ```

    For example, to run the `pub-sub-service` image built in step 1:

    ```shell
    podman run --network=host pub_sub_service
    ```

    >Note: A custom network is recommended when using a container for anything but testing.

1. To stop the container, run:

    ```shell
    podman ps -f ancestor=<image_name> --format="{{.Names}}" | xargs podman stop
    ```

    For example, to stop the `pub_sub_service` container started in step 2:

    ```shell
    podman ps -f ancestor=localhost/pub_sub_service:latest --format="{{.Names}}" | xargs podman stop
    ```

#### Running in Podman with overridden configuration

Follow the steps in [Running in Podman](#running-in-podman) to build the container.

1. To run the container with overridden configuration, create your config file and set an
environment variable called CONFIG_HOME to the path to the config file:

    ```shell
    export CONFIG_HOME={path to directory containing config file}
    ```

1. Then run the container with the following command:

    ```shell
    podman run --mount=type=bind,src=${CONFIG_HOME},dst=/mnt/config,ro=true --network=host <image_name>
    ```

    For example, to run the `pub_sub_service` image with overridden configuration:

    ```shell
    podman run --mount=type=bind,src=${CONFIG_HOME},dst=/mnt/config,ro=true --network=host pub_sub_service
    ```
