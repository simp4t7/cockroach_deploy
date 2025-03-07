use anyhow::Result;
#[cfg(feature = "cli")]
use clap::Args;
use log::info;
use scopeguard::defer;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, query};
use std::{io::BufRead, io::BufReader, process::Command, process::Stdio};
use tokio::sync::watch::{Receiver, channel};
use toml_config_trait::TomlConfig;
use toml_config_trait::TomlConfigTrait;

#[derive(TomlConfig, Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "cli", derive(Args))]
///This is the crux of the crate and where all of the details of the deployment are defined and
///stored.
pub struct CockroachConfig {
    /// namespace to use for this deployment. Defaults to `"default"`
    pub namespace: String,
    /// primary port used for grpc connections. Defaults to `26257`
    pub primary_port: i32,
    /// secondary port used for http connections. Defaults to `8080`
    pub secondary_port: i32,
    /// Names of all the databases to be created on setup. Also used to clear the databases.
    /// Defaults to `test_db`
    pub database_names: Vec<String>,
    /// Number of replicas for this deployment. Defaults to `3`
    pub replicas: i32,
    /// Number of cpus for this deployment. Defaults to `"2"`. This value must be a String.
    pub cpus: String,
    /// Amount of RAM for this deployment. Defaults to `"8Gi"`. This value must be a String.
    /// <https://www.cockroachlabs.com/docs/stable/configure-cockroachdb-kubernetes>
    pub memory: String,
    pub storage: String,
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

impl CockroachConfig {
    ///This function generates a new `CockroachConfig` from a path to a valid toml file.
    pub fn new_from_path(config_path: &str) -> Result<Self> {
        let config = CockroachConfig::read_from_path(config_path.into())?;
        Ok(config)
    }

    ///Write a default `CockroachConfig` to a path and return that config.
    pub fn write_default(config_path: &str) -> Result<Self> {
        let cockroach_config = CockroachConfig::default();
        cockroach_config.write_to_path(config_path.into())?;
        Ok(cockroach_config)
    }

    ///Create the SQL String used to initialize the cluster with databases
    pub(crate) fn create_db(&self) -> String {
        self.database_names.iter().fold("".to_string(), |acc, x| {
            acc + &format!("create database if not exists {}; ", x)
        })
    }

    async fn cockroach_port_forward(&self, mut cockroach_rx: Receiver<bool>) -> Result<()> {
        let mut cmd = Command::new("kubectl")
            .arg("port-forward")
            .arg("--address=0.0.0.0")
            .arg("-n")
            .arg(self.namespace.clone())
            .arg("statefulset/cockroachdb")
            .arg(format!("{}:{}", self.primary_port, self.primary_port))
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = cmd.stdout.take().unwrap();
        let stdout_reader = BufReader::new(stdout);
        let stdout_lines = stdout_reader.lines();

        if cockroach_rx.wait_for(|val| *val).await.is_ok() {
            info!("terminating process");
            cmd.kill().expect("error killing cockroach_child");
            return Ok(());
        }

        for line in stdout_lines {
            info!("{:?}", line?);
        }
        Ok(())
    }

    ///This function clears the cluster by removing and recreating all of the databases listed in
    ///the config toml. It's basically a fresh start without having to nuke and redeploy the whole
    ///cluster.
    pub async fn clear_database(&self) -> Result<()> {
        info!("Running: 'refresh_database'");

        let (tx, rx) = channel(false);
        let c = self.clone();
        tokio::task::spawn(async move {
            c.cockroach_port_forward(rx)
                .await
                .expect("cockroach_port_forward error");
        });
        defer!(tx.send(true).expect("send err"););

        for db in &self.database_names {
            info!("db: {}", &db);
            let db_url = format!(
                "postgresql://root@localhost:{}/{}?sslmode=disable",
                &self.primary_port, db
            );
            info!("db_url: {}", &db_url);
            let db_conn: Pool<Postgres> = Pool::connect_lazy(&db_url)?;
            let create_db = format!("create database if not exists {};", &db);
            let delete_db = format!("drop database {};", &db);
            query(&delete_db).execute(&db_conn).await?;
            query(&create_db).execute(&db_conn).await?;
        }

        info!("refresh_database -> Success!");
        Ok(())
    }
}
