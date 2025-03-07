# cockroach_deploy
Basic library for deploying a simple local cockroach cluster with an optional CLI included.

[<img src="https://img.shields.io/crates/v/bottom.svg?style=flat-square" alt="crates.io">](https://crates.io/crates/cockroach_deploy)
[<img src="https://img.shields.io/badge/docs-stable-66c2a5?style=flat-square&labelColor=555555&logoColor=white" alt="Docs">](https://docs.rs/cockroach_deploy)

## About

The main purpose for this library is quickly deploying a basic [cockroachdb](https://www.cockroachlabs.com/) cluster locally.
You will need a local Kubernetes cluster running for this to work. I have only tested this with [minikube](https://minikube.sigs.k8s.io/docs/), but others may work as well.
Additionally, you will need a local config file for setting cockroachdb variables.

## Installation & Usage

if using the CLI:

```cargo install cockroach_deploy```

usage:

```cockroach_deploy_cli --help```

if using the library:

```cargo add cockroach_deploy``` 

## Basic Usage

```Rust
let config = CockroachConfig::new_from_config("cockroach_config.toml")?;
config.init_cockroach()?;
config.delete_cockroach()?;

```

## Additional Note

This is pretty niche, and adapated for my use-case so your mileage may vary. If you are interested and struggling to get it to work, open an issue and I'll do what I can to help adapt it.


