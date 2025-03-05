use std::collections::BTreeMap;

use k8s_openapi::api::apps::v1::{StatefulSet, StatefulSetSpec, StatefulSetUpdateStrategy};
use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::api::core::v1::{
    Affinity, Container, ContainerPort, EnvVar, EnvVarSource, HTTPGetAction, PersistentVolumeClaim,
    PersistentVolumeClaimSpec, PersistentVolumeClaimVolumeSource, PodAffinityTerm, PodAntiAffinity,
    PodSpec, PodTemplateSpec, Probe, ResourceFieldSelector, ResourceRequirements, Service,
    ServicePort, ServiceSpec, Volume, VolumeMount, VolumeResourceRequirements,
    WeightedPodAffinityTerm,
};
use k8s_openapi::api::policy::v1::{PodDisruptionBudget, PodDisruptionBudgetSpec};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, LabelSelectorRequirement};
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::api::ObjectMeta;

use crate::config::CockroachConfig;

impl CockroachConfig {
    /// This function creates a cockroachdb service, part of the statefulset yaml.
    /// https://github.com/cockroachdb/cockroach/blob/master/cloud/kubernetes/cockroachdb-statefulset.yaml
    pub(crate) fn cockroach_first_service(&self) -> Service {
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
                        port: self.primary_port,
                        target_port: Some(IntOrString::Int(self.primary_port)),
                        ..Default::default()
                    },
                    ServicePort {
                        name: Some(String::from("http")),
                        port: self.secondary_port,
                        target_port: Some(IntOrString::Int(self.secondary_port)),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// This function also creates a cockroachdb service, part of the statefulset yaml.
    pub(crate) fn cockroach_second_service(&self) -> Service {
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
                    (
                        String::from("prometheus.io/port"),
                        self.secondary_port.to_string(),
                    ),
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
                        port: self.primary_port,
                        target_port: Some(IntOrString::Int(self.primary_port)),
                        ..Default::default()
                    },
                    ServicePort {
                        name: Some(String::from("http")),
                        port: self.secondary_port,
                        target_port: Some(IntOrString::Int(self.secondary_port)),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            }),
            status: None,
        }
    }

    /// This function also creates the cockroachdb pod disruption budget, part of the statefulset yaml.
    pub(crate) fn cockroach_pod_disruption_budget() -> PodDisruptionBudget {
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
    pub(crate) fn cockroach_stateful_set(&self) -> StatefulSet {
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
                                                        values: Some(vec![String::from(
                                                            "cockroachdb",
                                                        )]),
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
                                    (String::from("cpu"), Quantity(self.cpus.clone())),
                                    (String::from("memory"), Quantity(self.memory.clone())),
                                ])),
                                limits: Some(BTreeMap::from([
                                    (String::from("cpu"), Quantity(self.cpus.clone())),
                                    (String::from("memory"), Quantity(self.memory.clone())),
                                ])),
                                ..Default::default()
                            }),
                            ports: Some(vec![
                                ContainerPort {
                                    container_port: self.primary_port,
                                    name: Some(String::from("grpc")),
                                    ..Default::default()
                                },
                                ContainerPort {
                                    container_port: self.secondary_port,
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
                replicas: Some(self.replicas),
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
                                Quantity(self.storage.clone()),
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
    pub(crate) fn cockroach_cluster_init() -> Job {
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

    pub(crate) fn cockroach_make_db(&self) -> Job {
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
                                String::from(format!("--execute={}", &self.create_db())),
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
}
