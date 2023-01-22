#!/bin/bash

podman rm -af
podman rmi -af
podman volume rm -a

sudo podman rm -af
sudo podman rmi -af
sudo podman volume rm -a
