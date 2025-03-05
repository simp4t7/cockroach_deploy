#[cfg(feature = "cli")]
use clap::Args;
use serde::{Deserialize, Serialize};
use toml_config_trait::TomlConfig;
use toml_config_trait::TomlConfigTrait;

#[derive(TomlConfig, Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "cli", derive(Args))]
pub struct CockroachConfig {
    /// namespace to use for this deployment. Defaults to `"default"`
    pub namespace: String,
    /// primary port used for grpc connections. Defaults to `26257`
    pub primary_port: i32,
    /// secondary port used for http connections. Defaults to `8080`
    pub secondary_port: i32,
    /// Names of all the databases to be created on setup. Also used to clear the databases`.
    /// Defaults to `test_db`
    pub database_names: Vec<String>,
    /// Number of replicas for this deployment. Defaults to `3`
    pub replicas: i32,
    /// Number of cpus for this deployment. Defaults to `"2"`. This value must be a String.
    pub cpus: String,
    /// Amount of RAM for this deployment. Defaults to `"8Gi"`. This value must be a String.
    ///
    /// <https://www.cockroachlabs.com/docs/stable/configure-cockroachdb-kubernetes>
    pub memory: String,
    pub storage: String,
}
impl CockroachConfig {
    pub fn create_db(&self) -> String {
        self.database_names.iter().fold("".to_string(), |acc, x| {
            acc + &format!("create database if not exists {}; ", x)
        })
    }

    pub fn delete_db(&self) -> String {
        self.database_names.iter().fold("".to_string(), |acc, x| {
            acc + &format!("drop database {}; ", x)
        })
    }

    pub fn db_urls(&self) -> Vec<String> {
        self.database_names
            .iter()
            .map(|x| {
                format!(
                    "postgresql://root@cockroachdb-0.cockroachdb:{}/{}?sslmode=disable",
                    &self.primary_port, x
                )
            })
            .collect::<Vec<String>>()
    }
}

impl Default for CockroachConfig {
    fn default() -> Self {
        Self {
            namespace: String::from("default"),
            primary_port: 26257,
            secondary_port: 8080,
            database_names: vec![String::from("test_db")],
            replicas: 3,
            cpus: String::from("2"),
            memory: String::from("8Gi"),
            storage: String::from("50Gi"),
        }
    }
}
