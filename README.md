# cockroach_deploy
Basic library for deploying a simple local cockroach cluster with an optional CLI included.

[<img src="https://img.shields.io/crates/v/bottom.svg?style=flat-square" alt="crates.io">](https://crates.io/crates/cockroach_deploy)
[<img src="https://img.shields.io/badge/docs-stable-66c2a5?style=flat-square&labelColor=555555&logoColor=white" alt="Docs">](https://docs.rs/cockroach_deploy)


## Installation & Usage

```cargo add cockroach_deploy``` or ```git clone git@github.com:simp4t7/cockroach_deploy.git```

if using the CLI:

```cargo install cockroach_deploy -F cli```

## Basic Usage

The main purpose for this library is quickly deploying a basic [cockroachdb](https://www.cockroachlabs.com/) cluster locally.

You will need a local Kubernetes cluster running for this to work. I have only tested this with [minikube](https://minikube.sigs.k8s.io/docs/), but others may work as well.

Additionally, you will need a local config file for setting cockroachdb variables. If you don't provide a config toml file, one will automatically be generated with default values.

## Features


