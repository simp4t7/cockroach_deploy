use anyhow::Result;
use k8s_openapi::api::core::v1::Namespace;
use kube::api::ObjectMeta;
use kube::{Api, Client, Config};
use log::info;

pub(crate) async fn init_namespace(namespace: &str) -> Result<Client> {
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
            info!(
                "{} already exists, skipping namespace creation",
                &namespace_str
            );
            return Ok(());
        }
        Err(_) => {
            namespaces
                .create(&Default::default(), &namespace_struct)
                .await?;
            info!("successfully created {} namespace", &namespace_str)
        }
    }

    Ok(())
}
