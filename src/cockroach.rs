use crate::utils::get_kube_client;
use anyhow::Result;
use k8s_openapi::api::apps::v1::{StatefulSet, StatefulSetSpec, StatefulSetUpdateStrategy};
use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::api::core::v1::{
    Affinity, Container, ContainerPort, EnvVar, EnvVarSource, HTTPGetAction, PersistentVolumeClaim,
    PersistentVolumeClaimSpec, PersistentVolumeClaimVolumeSource, Pod, PodAffinityTerm,
    PodAntiAffinity, PodSpec, PodTemplateSpec, Probe, ResourceFieldSelector, ResourceRequirements,
    Volume, VolumeMount, VolumeResourceRequirements, WeightedPodAffinityTerm,
};
use k8s_openapi::api::policy::v1::{PodDisruptionBudget, PodDisruptionBudgetSpec};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{
    LabelSelector, LabelSelectorRequirement, ObjectMeta,
};
use k8s_openapi::{
    api::core::v1::{Service, ServicePort, ServiceSpec},
    apimachinery::pkg::util::intstr::IntOrString,
};
use kube::api::{DeleteParams, ListParams, PostParams, PropagationPolicy};
use kube::runtime::wait::{await_condition, conditions};
use kube::{Api, ResourceExt};
use std::collections::BTreeMap;

/// This function creates all of the services / stateful sets / deployments / and jobs necessary to
/// start a cockroachdb cluster.

pub async fn initialize_cockroach_db(namespace: Option<String>) -> Result<()> {
    let namespace = if let Some(name) = namespace {
        name
    } else {
        "default".to_string()
    };
    let client = get_kube_client(&namespace).await;
    let services: Api<Service> = Api::namespaced(client.clone(), &namespace);
    let pp = PostParams::default();
    let first_service_yaml = cockroach_first_service();
    let create_result = services.create(&pp, &first_service_yaml).await;
    match create_result {
        Ok(_) => println!("create cockroach_first_service successful"),
        Err(e) => return Err(e.into()),
    }

    let second_service_yaml = cockroach_second_service();
    let create_result = services.create(&pp, &second_service_yaml).await;
    match create_result {
        Ok(_) => println!("create cockroach_second_service successful"),
        Err(e) => return Err(e.into()),
    }
    let budget: Api<PodDisruptionBudget> = Api::default_namespaced(client.clone());
    let budget_yaml = cockroach_disruption_budget();
    let create_result = budget.create(&pp, &budget_yaml).await;
    match create_result {
        Ok(_) => println!("create cockroach_pod_disruption successful"),
        Err(e) => return Err(e.into()),
    }
    let stateset: Api<StatefulSet> = Api::default_namespaced(client.clone());
    let stateset_yaml = cockroach_statefulset();
    let create_result = stateset.create(&pp, &stateset_yaml).await;
    match create_result {
        Ok(_) => println!("create cockroach_stateful_set successful"),
        Err(e) => return Err(e.into()),
    }

    let job: Api<Job> = Api::default_namespaced(client.clone());
    let cluster_init_yaml = cockroach_cluster_init();
    let create_result = job.create(&pp, &cluster_init_yaml).await;
    match create_result {
        Ok(_) => println!("create cockroach_cluster_init successful"),
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

    let make_db_yaml = cockroach_make_db();
    let create_result = job.create(&pp, &make_db_yaml).await;
    match create_result {
        Ok(_) => println!("create cockroach_make_db successful"),
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
        let _wait = await_condition(jobs.clone(), &name, conditions::is_job_completed()).await?;
        println!("{} finished its job, sleep now sweet prince", &name);
        jobs.delete(&j.name_any(), &dp).await?;
        let _wait = await_condition(jobs.clone(), &name, conditions::is_deleted(&uid)).await?;
        println!("finished deleting {}!", &name);
    }

    Ok(())
}

/// This function deletes all of the services / stateful sets / deployments / and jobs necessary to
/// completely eradicate a cockroachdb cluster, N U C L E A R

pub async fn delete_cockroach_db(namespace: Option<String>) -> Result<()> {
    let namespace = if let Some(name) = namespace {
        name
    } else {
        "default".to_string()
    };
    let client = get_kube_client(&namespace).await;
    let jobs: Api<Job> = Api::namespaced(client.clone(), &namespace);
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
        println!("finished deleting {}!", &name);
    }

    let services: Api<Service> = Api::default_namespaced(client.clone());
    for j in services.list(&lp).await? {
        services.delete(&j.name_any(), &dp).await?;
        let name = j.name_any();
        let uid = j.uid().expect("no uid");
        let _wait = await_condition(services.clone(), &name, conditions::is_deleted(&uid)).await?;
        println!("finished deleting {}!", &name);
    }

    let budget: Api<PodDisruptionBudget> = Api::default_namespaced(client.clone());
    for j in budget.list(&lp).await? {
        budget.delete(&j.name_any(), &dp).await?;
        let name = j.name_any();
        let uid = j.uid().expect("no uid");
        let _wait = await_condition(budget.clone(), &name, conditions::is_deleted(&uid)).await?;
        println!("finished deleting {}!", &name);
    }

    let stateset: Api<StatefulSet> = Api::default_namespaced(client.clone());
    for j in stateset.list(&lp).await? {
        stateset.delete(&j.name_any(), &dp).await?;
        let name = j.name_any();
        let uid = j.uid().expect("no uid");
        let _wait = await_condition(stateset.clone(), &name, conditions::is_deleted(&uid)).await?;
        println!("finished deleting {}!", &name);
    }

    let pod: Api<Pod> = Api::default_namespaced(client.clone());
    for j in pod.list(&lp).await? {
        pod.delete(&j.name_any(), &dp).await?;
        let name = j.name_any();
        let uid = j.uid().expect("no uid");
        let _wait = await_condition(pod.clone(), &name, conditions::is_deleted(&uid)).await?;
        println!("finished deleting {}!", &name);
    }

    let pvc: Api<PersistentVolumeClaim> = Api::default_namespaced(client.clone());
    for j in pvc.list(&lp).await? {
        pvc.delete(&j.name_any(), &dp).await?;
        let name = j.name_any();
        let uid = j.uid().expect("no uid");
        let _wait = await_condition(pvc.clone(), &name, conditions::is_deleted(&uid)).await?;
        println!("finished deleting {}!", &name);
    }
    Ok(())
}

/// This function creates a cockroachdb service, part of the statefulset yaml.

fn cockroach_first_service() -> Service {
    Service {
        metadata: ObjectMeta {
            name: Some(String::from("cockroachdb-public")),
            labels: Some(BTreeMap::from([(
                String::from("app"),
                String::from("cockroachdb"),
            )])),
            ..Default::default()
        },
        spec: Some(ServiceSpec {
            selector: Some(BTreeMap::from([(
                String::from("app"),
                String::from("cockroachdb"),
            )])),
            ports: Some(vec![
                ServicePort {
                    name: Some(String::from("grpc")),
                    port: 26257,
                    target_port: Some(IntOrString::Int(26257)),
                    ..Default::default()
                },
                ServicePort {
                    name: Some(String::from("http")),
                    port: 8080,
                    target_port: Some(IntOrString::Int(8080)),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// This function also creates a cockroachdb service, part of the statefulset yaml.

fn cockroach_second_service() -> Service {
    Service {
        metadata: ObjectMeta {
            name: Some(String::from("cockroachdb")),
            annotations: Some(BTreeMap::from([
                (
                    String::from("service.alpha.kubernetes.io/tolerate-unready-endpoints"),
                    String::from("true"),
                ),
                (String::from("prometheus.io/scrape"), String::from("true")),
                (
                    String::from("prometheus.io/path"),
                    String::from("_status/vars"),
                ),
                (String::from("prometheus.io/port"), String::from("8080")),
            ])),
            labels: Some(BTreeMap::from([(
                String::from("app"),
                String::from("cockroachdb"),
            )])),
            ..Default::default()
        },
        spec: Some(ServiceSpec {
            selector: Some(BTreeMap::from([(
                String::from("app"),
                String::from("cockroachdb"),
            )])),
            publish_not_ready_addresses: Some(true),
            ports: Some(vec![
                ServicePort {
                    name: Some(String::from("grpc")),
                    port: 26257,
                    target_port: Some(IntOrString::Int(26257)),
                    ..Default::default()
                },
                ServicePort {
                    name: Some(String::from("http")),
                    port: 8080,
                    target_port: Some(IntOrString::Int(8080)),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        }),
        status: None,
    }
}

/// This function also creates the cockroachdb pod disruption budget, part of the statefulset yaml.

fn cockroach_disruption_budget() -> PodDisruptionBudget {
    PodDisruptionBudget {
        metadata: ObjectMeta {
            name: Some(String::from("cockroachdb-budget")),
            labels: Some(BTreeMap::from([(
                String::from("app"),
                String::from("cockroachdb"),
            )])),
            ..Default::default()
        },
        spec: Some(PodDisruptionBudgetSpec {
            selector: Some(LabelSelector {
                match_labels: Some(BTreeMap::from([(
                    String::from("app"),
                    String::from("cockroachdb"),
                )])),
                ..Default::default()
            }),
            max_unavailable: Some(IntOrString::Int(1)),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// This function also creates the cockroachdb statefulset, part of the statefulset yaml.

fn cockroach_statefulset() -> StatefulSet {
    StatefulSet {
        metadata: ObjectMeta {
            name: Some(String::from("cockroachdb")),
            labels: Some(BTreeMap::from([(
                String::from("app"),
                String::from("cockroachdb"),
            )])),
            ..Default::default()
        },
        spec: Some(StatefulSetSpec {
            service_name: String::from("cockroachdb"),
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: Some(BTreeMap::from([(
                        String::from("app"),
                        String::from("cockroachdb"),
                    )])),
                    ..Default::default()
                }),
                spec: Some(PodSpec {
                    affinity: Some(Affinity {
                        pod_anti_affinity: Some(PodAntiAffinity {
                            preferred_during_scheduling_ignored_during_execution: Some(vec![
                                WeightedPodAffinityTerm {
                                    weight: 100,
                                    pod_affinity_term: PodAffinityTerm {
                                        label_selector: Some(LabelSelector {
                                            match_expressions: Some(vec![
                                                LabelSelectorRequirement {
                                                    key: String::from("app"),
                                                    operator: String::from("In"),
                                                    values: Some(vec![String::from("cockroachdb")]),
                                                },
                                            ]),
                                            ..Default::default()
                                        }),
                                        topology_key: String::from("kubernetes.io/hostname"),
                                        ..Default::default()
                                    },
                                },
                            ]),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    containers: vec![Container {
                        name: String::from("cockroachdb"),
                        image: Some(String::from("cockroachdb/cockroach:latest")),
                        image_pull_policy: Some(String::from("IfNotPresent")),
                        resources: Some(ResourceRequirements {
                            requests: Some(BTreeMap::from([
                                (String::from("cpu"), Quantity(String::from("4"))),
                                (String::from("memory"), Quantity(String::from("8Gi"))),
                            ])),
                            limits: Some(BTreeMap::from([
                                (String::from("cpu"), Quantity(String::from("4"))),
                                (String::from("memory"), Quantity(String::from("8Gi"))),
                            ])),
                            ..Default::default()
                        }),
                        ports: Some(vec![
                            ContainerPort {
                                container_port: 26257,
                                name: Some(String::from("grpc")),
                                ..Default::default()
                            },
                            ContainerPort {
                                container_port: 8080,
                                name: Some(String::from("http")),
                                ..Default::default()
                            },
                        ]),
                        readiness_probe: Some(Probe {
                            http_get: Some(HTTPGetAction {
                                path: Some(String::from("/health?ready=1")),
                                port: IntOrString::String(String::from("http")),
                                ..Default::default()
                            }),
                            initial_delay_seconds: Some(10),
                            period_seconds: Some(5),
                            failure_threshold: Some(2),
                            ..Default::default()
                        }),
                        volume_mounts: Some(vec![VolumeMount {
                            name: String::from("datadir"),
                            mount_path: String::from("/cockroach/cockroach-data"),
                            ..Default::default()
                        }]),
                        env: Some(vec![
                            EnvVar {
                                name: String::from("COCKROACH_CHANNEL"),
                                value: Some(String::from("kubernetes-insecure")),
                                ..Default::default()
                            },
                            EnvVar {
                                name: String::from("GOMAXPROCS"),
                                value_from: Some(EnvVarSource {
                                    resource_field_ref: Some(ResourceFieldSelector {
                                        resource: String::from("limits.cpu"),
                                        divisor: Some(Quantity(String::from("1"))),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                            EnvVar {
                                name: String::from("MEMORY_LIMIT_MIB"),
                                value_from: Some(EnvVarSource {
                                    resource_field_ref: Some(ResourceFieldSelector {
                                        resource: String::from("limits.memory"),
                                        divisor: Some(Quantity(String::from("1Mi"))),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                        ]),
                        command: Some(vec![
                            String::from("/bin/bash"),
                            String::from("-ecx"),
                            String::from(
                                "exec /cockroach/cockroach start --logtostderr --insecure --advertise-host $(hostname -f) --http-addr 0.0.0.0 --join cockroachdb-0.cockroachdb,cockroachdb-1.cockroachdb,cockroachdb-2.cockroachdb --cache $(expr $MEMORY_LIMIT_MIB / 4)MiB --max-sql-memory $(expr $MEMORY_LIMIT_MIB / 4)MiB",
                            ),
                        ]),
                        ..Default::default()
                    }],
                    termination_grace_period_seconds: Some(60),
                    volumes: Some(vec![Volume {
                        name: String::from("datadir"),
                        persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
                            claim_name: String::from("datadir"),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }]),

                    ..Default::default()
                }),
            },
            selector: LabelSelector {
                match_labels: Some(BTreeMap::from([(
                    String::from("app"),
                    String::from("cockroachdb"),
                )])),
                ..Default::default()
            },
            replicas: Some(3),
            update_strategy: Some(StatefulSetUpdateStrategy {
                type_: Some(String::from("RollingUpdate")),
                ..Default::default()
            }),
            pod_management_policy: Some(String::from("Parallel")),
            volume_claim_templates: Some(vec![PersistentVolumeClaim {
                metadata: ObjectMeta {
                    name: Some(String::from("datadir")),
                    ..Default::default()
                },
                spec: Some(PersistentVolumeClaimSpec {
                    access_modes: Some(vec![String::from("ReadWriteOnce")]),
                    resources: Some(VolumeResourceRequirements {
                        requests: Some(BTreeMap::from([(
                            String::from("storage"),
                            Quantity(String::from("50Gi")),
                        )])),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// This function creates the cockroachdb cluster init job, part of the cluster-init yaml.

fn cockroach_cluster_init() -> Job {
    Job {
        metadata: ObjectMeta {
            name: Some(String::from("cluster-init")),
            labels: Some(BTreeMap::from([(
                String::from("app"),
                String::from("cockroachdb"),
            )])),
            ..Default::default()
        },
        spec: Some(JobSpec {
            template: PodTemplateSpec {
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: String::from("cluster-init"),
                        image: Some(String::from("cockroachdb/cockroach:latest")),
                        image_pull_policy: Some(String::from("IfNotPresent")),
                        command: Some(vec![
                            String::from("/cockroach/cockroach"),
                            String::from("init"),
                            String::from("--insecure"),
                            String::from("--host=cockroachdb-0.cockroachdb"),
                        ]),
                        ..Default::default()
                    }],
                    restart_policy: Some(String::from("OnFailure")),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// This is a custom job that creates `log_db` and `program_db` after cockroachdb has initialized.

fn cockroach_make_db() -> Job {
    Job {
        metadata: ObjectMeta {
            name: Some(String::from("make-db")),
            labels: Some(BTreeMap::from([(
                String::from("app"),
                String::from("cockroachdb"),
            )])),
            ..Default::default()
        },
        spec: Some(JobSpec {
            template: PodTemplateSpec {
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: String::from("cockroach-make-db"),
                        image: Some(String::from("cockroachdb/cockroach:latest")),
                        image_pull_policy: Some(String::from("IfNotPresent")),
                        command: Some(vec![
                            String::from("cockroach"),
                            String::from("sql"),
                            String::from("--insecure"),
                            String::from("--host=cockroachdb-0.cockroachdb"),
                            String::from(
                                "--execute=create database if not exists log_db; create database if not exists program_db;",
                            ),
                        ]),
                        ..Default::default()
                    }],
                    restart_policy: Some(String::from("OnFailure")),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        }),
        ..Default::default()
    }
}
