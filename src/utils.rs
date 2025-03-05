use anyhow::Result;
use k8s_openapi::api::core::v1::Namespace;
use kube::api::ObjectMeta;
use kube::{Api, Client, Config};
use log::info;
use sqlx::{Pool, Postgres, query};

#[cfg(feature = "cli")]
use std::{io::BufRead, io::BufReader, process::Command, process::Stdio};
#[cfg(feature = "cli")]
use tokio::sync::watch::Receiver;

use crate::cockroach::CockroachResources;
use crate::config::CockroachConfig;

pub(crate) async fn get_kube_client(namespace: &str) -> Client {
    init_namespace(namespace)
        .await
        .expect("init_namespace error")
}

async fn init_namespace(namespace: &str) -> Result<Client> {
    let client = Client::try_default().await?;
    check_namespace(&client, &namespace).await?;
    let mut config = Config::infer().await?;
    config.default_namespace = namespace.to_string();
    let client = Client::try_from(config)?;
    Ok(client)
}

async fn check_namespace(client: &Client, namespace_str: &str) -> Result<()> {
    let namespaces: Api<Namespace> = Api::all(client.clone());
    let namespace_struct = Namespace {
        metadata: ObjectMeta {
            name: Some(namespace_str.to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    match namespaces.get(namespace_str).await {
        Ok(_) => {
            println!(
                "{} already exists, skipping namespace creation",
                &namespace_str
            );
            return Ok(());
        }
        Err(_) => {
            namespaces
                .create(&Default::default(), &namespace_struct)
                .await?;
            println!("successfully created {} namespace", &namespace_str)
        }
    }

    Ok(())
}

impl CockroachResources {
    #[cfg(feature = "cli")]
    pub async fn cockroach_port_forward(&self, mut cockroach_rx: Receiver<bool>) -> Result<()> {
        let mut cmd = Command::new("kubectl")
            .arg("port-forward")
            .arg("--address=0.0.0.0")
            .arg("-n")
            .arg(self.config.namespace.clone())
            .arg("statefulset/cockroachdb")
            .arg(format!(
                "{}:{}",
                self.config.primary_port, self.config.primary_port
            ))
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = cmd.stdout.take().unwrap();
        let stdout_reader = BufReader::new(stdout);
        let stdout_lines = stdout_reader.lines();

        if cockroach_rx.wait_for(|val| *val).await.is_ok() {
            println!("killing the child (but it's a roach so it's okay)");
            cmd.kill().expect("error killing cockroach_child");
            return Ok(());
        }

        for line in stdout_lines {
            info!("{:?}", line?);
        }
        Ok(())
    }

    pub async fn refresh_database(&self) -> Result<()> {
        info!("Running: 'refresh_database'");
        for db in &self.config.database_names {
            let db_url = format!(
                "postgresql://root@cockroachdb-0.cockroachdb:{}/{}?sslmode=disable",
                &self.config.primary_port, db
            );
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
