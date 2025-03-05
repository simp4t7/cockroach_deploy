use crate::config::CockroachConfig;
use crate::utils::get_kube_client;
use anyhow::Result;
use k8s_openapi::api::apps::v1::StatefulSet;
use k8s_openapi::api::batch::v1::Job;
use k8s_openapi::api::core::v1::Service;
use k8s_openapi::api::core::v1::{PersistentVolumeClaim, Pod};
use k8s_openapi::api::policy::v1::PodDisruptionBudget;
use kube::api::{DeleteParams, ListParams, PostParams, PropagationPolicy};
use kube::runtime::wait::{await_condition, conditions};
use kube::{Api, ResourceExt};
use log::info;
use std::fs;
use toml_config_trait::TomlConfigTrait;

/// This function creates all of the services / stateful sets / deployments / and jobs necessary to
/// start a cockroachdb cluster.

#[derive(Clone)]
pub struct CockroachResources {
    pub first_service: Service,
    pub second_service: Service,
    pub pod_disruption_budget: PodDisruptionBudget,
    pub stateful_set: StatefulSet,
    pub cluster_init: Job,
    pub make_db: Job,
    pub config: CockroachConfig,
}

impl CockroachResources {
    pub fn new(config_path: &str) -> Result<Self> {
        if !fs::exists(&config_path)? {
            CockroachConfig::default().write_to_path(config_path.into())?;
        }
        let config = CockroachConfig::read_from_path(config_path.into())?;
        Ok(Self {
            first_service: config.cockroach_first_service(),
            second_service: config.cockroach_second_service(),
            pod_disruption_budget: CockroachConfig::cockroach_pod_disruption_budget(),
            stateful_set: config.cockroach_stateful_set(),
            cluster_init: CockroachConfig::cockroach_cluster_init(),
            make_db: config.cockroach_make_db(),
            config,
        })
    }

    /// This function deletes all of the services / stateful sets / deployments / and jobs necessary to
    /// completely eradicate a cockroachdb cluster, N U C L E A R

    pub async fn delete_cockroach(&self) -> Result<()> {
        let client = get_kube_client(&self.config.namespace).await;
        let jobs: Api<Job> = Api::namespaced(client.clone(), &self.config.namespace);
        let lp = ListParams::default().labels("app=cockroachdb");
        let dp = DeleteParams {
            propagation_policy: Some(PropagationPolicy::Foreground),
            ..Default::default()
        };

        for j in jobs.list(&lp).await? {
            jobs.delete(&j.name_any(), &dp).await?;
            let name = j.name_any();
            let uid = j.uid().expect("no uid");
            let _wait = await_condition(jobs.clone(), &name, conditions::is_deleted(&uid)).await?;
            info!("finished deleting {}!", &name);
        }

        let services: Api<Service> = Api::default_namespaced(client.clone());
        for j in services.list(&lp).await? {
            services.delete(&j.name_any(), &dp).await?;
            let name = j.name_any();
            let uid = j.uid().expect("no uid");
            let _wait =
                await_condition(services.clone(), &name, conditions::is_deleted(&uid)).await?;
            info!("finished deleting {}!", &name);
        }

        let budget: Api<PodDisruptionBudget> = Api::default_namespaced(client.clone());
        for j in budget.list(&lp).await? {
            budget.delete(&j.name_any(), &dp).await?;
            let name = j.name_any();
            let uid = j.uid().expect("no uid");
            let _wait =
                await_condition(budget.clone(), &name, conditions::is_deleted(&uid)).await?;
            info!("finished deleting {}!", &name);
        }

        let stateset: Api<StatefulSet> = Api::default_namespaced(client.clone());
        for j in stateset.list(&lp).await? {
            stateset.delete(&j.name_any(), &dp).await?;
            let name = j.name_any();
            let uid = j.uid().expect("no uid");
            let _wait =
                await_condition(stateset.clone(), &name, conditions::is_deleted(&uid)).await?;
            info!("finished deleting {}!", &name);
        }

        let pod: Api<Pod> = Api::default_namespaced(client.clone());
        for j in pod.list(&lp).await? {
            pod.delete(&j.name_any(), &dp).await?;
            let name = j.name_any();
            let uid = j.uid().expect("no uid");
            let _wait = await_condition(pod.clone(), &name, conditions::is_deleted(&uid)).await?;
            info!("finished deleting {}!", &name);
        }

        let pvc: Api<PersistentVolumeClaim> = Api::default_namespaced(client.clone());
        for j in pvc.list(&lp).await? {
            pvc.delete(&j.name_any(), &dp).await?;
            let name = j.name_any();
            let uid = j.uid().expect("no uid");
            let _wait = await_condition(pvc.clone(), &name, conditions::is_deleted(&uid)).await?;
            info!("finished deleting {}!", &name);
        }
        Ok(())
    }

    pub async fn init_cockroach(&self) -> Result<()> {
        let client = get_kube_client(&self.config.namespace).await;
        let services: Api<Service> = Api::namespaced(client.clone(), &self.config.namespace);
        let pp = PostParams::default();
        let first_service_yaml = &self.first_service;
        let create_result = services.create(&pp, &first_service_yaml).await;
        match create_result {
            Ok(_) => info!("create cockroach_first_service successful"),
            Err(e) => return Err(e.into()),
        }

        let second_service_yaml = &self.second_service;
        let create_result = services.create(&pp, &second_service_yaml).await;
        match create_result {
            Ok(_) => info!("create cockroach_second_service successful"),
            Err(e) => return Err(e.into()),
        }
        let budget: Api<PodDisruptionBudget> = Api::default_namespaced(client.clone());
        let budget_yaml = &self.pod_disruption_budget;
        let create_result = budget.create(&pp, &budget_yaml).await;
        match create_result {
            Ok(_) => info!("create cockroach_pod_disruption successful"),
            Err(e) => return Err(e.into()),
        }
        let stateset: Api<StatefulSet> = Api::default_namespaced(client.clone());
        let stateset_yaml = &self.stateful_set;
        let create_result = stateset.create(&pp, &stateset_yaml).await;
        match create_result {
            Ok(_) => info!("create cockroach_stateful_set successful"),
            Err(e) => return Err(e.into()),
        }

        let job: Api<Job> = Api::default_namespaced(client.clone());
        let cluster_init_yaml = &self.cluster_init;
        let create_result = job.create(&pp, &cluster_init_yaml).await;
        match create_result {
            Ok(_) => info!("create cockroach_cluster_init successful"),
            Err(e) => return Err(e.into()),
        }

        let jobs: Api<Job> = Api::default_namespaced(client.clone());
        let cluster_init = jobs.get("cluster-init").await?;
        let _wait = await_condition(
            jobs.clone(),
            &cluster_init.name_any(),
            conditions::is_job_completed(),
        )
        .await?;

        let make_db_yaml = &self.make_db;
        let create_result = job.create(&pp, &make_db_yaml).await;
        match create_result {
            Ok(_) => info!("create cockroach_make_db successful"),
            Err(e) => return Err(e.into()),
        }

        let lp = ListParams::default().labels("app=cockroachdb");
        let dp = DeleteParams {
            propagation_policy: Some(PropagationPolicy::Foreground),
            grace_period_seconds: Some(5),
            ..Default::default()
        };

        let jobs: Api<Job> = Api::default_namespaced(client.clone());
        for j in jobs.list(&lp).await? {
            let name = j.name_any();
            let uid = j.uid().expect("no uid");
            let _wait =
                await_condition(jobs.clone(), &name, conditions::is_job_completed()).await?;
            info!("{} finished its job, sleep now sweet prince", &name);
            jobs.delete(&j.name_any(), &dp).await?;
            let _wait = await_condition(jobs.clone(), &name, conditions::is_deleted(&uid)).await?;
            info!("finished deleting {}!", &name);
        }

        Ok(())
    }
}
