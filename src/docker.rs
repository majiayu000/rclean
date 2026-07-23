mod command;

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::RcleanError;
use crate::model::Safety;
use crate::stdio::outln;

use command::{run_docker_command, status_from_command_error};

/// `docker system df` walks every image, container, and volume layer,
/// so multi-second runs are normal on a populated daemon rather than a
/// sign of trouble -- 7.15s measured against Docker 29.5.3 (spec:
/// `specs/GH350/product.md`). The previous 5s default therefore failed
/// on ordinary machines and made the user find `--timeout` before they
/// could get any answer.
///
/// Must stay in sync with the `--timeout` clap default in
/// `cli::DockerReportArgs`; `default_timeout_matches_cli_default`
/// pins them together.
pub(crate) const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone)]
pub struct DockerReportOptions {
    pub docker_bin: Option<PathBuf>,
    pub timeout: Duration,
}

impl Default for DockerReportOptions {
    fn default() -> Self {
        Self {
            docker_bin: None,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerReport {
    pub schema_version: u32,
    pub generated_at: String,
    pub status: DockerStatus,
    pub summary: DockerSummary,
    pub resources: Vec<DockerResourceReport>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DockerStatus {
    Available {
        #[serde(rename = "serverVersion")]
        server_version: Option<String>,
    },
    Unavailable {
        reason: String,
    },
    PermissionDenied {
        reason: String,
    },
    TimedOut {
        command: String,
        #[serde(rename = "timeoutMs")]
        timeout_ms: u128,
    },
    Error {
        reason: String,
    },
}

impl DockerStatus {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available { .. })
    }

    fn human_label(&self) -> &'static str {
        match self {
            Self::Available { .. } => "available",
            Self::Unavailable { .. } => "unavailable",
            Self::PermissionDenied { .. } => "permission-denied",
            Self::TimedOut { .. } => "timed-out",
            Self::Error { .. } => "error",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerSummary {
    pub resources: usize,
    pub caution_resources: usize,
    pub report_only_resources: usize,
    pub blocked_resources: usize,
    pub selected_resources: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerResourceReport {
    pub resource_id: String,
    pub label: String,
    pub safety: Safety,
    pub selected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reclaimable: Option<String>,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockerResourceKind {
    BuildCache,
    DanglingImage,
    StoppedAnonymousContainer,
    TaggedImage,
    NamedContainer,
    Volume,
    Network,
    RunningOrInUse,
    PermissionDenied,
    MissingDuringRevalidation,
    StorageDirectory,
    Unknown,
}

pub fn classify_resource(kind: DockerResourceKind) -> Safety {
    match kind {
        DockerResourceKind::BuildCache
        | DockerResourceKind::DanglingImage
        | DockerResourceKind::StoppedAnonymousContainer => Safety::Caution,
        DockerResourceKind::TaggedImage
        | DockerResourceKind::NamedContainer
        | DockerResourceKind::Volume
        | DockerResourceKind::Network => Safety::ReportOnly,
        DockerResourceKind::RunningOrInUse
        | DockerResourceKind::PermissionDenied
        | DockerResourceKind::MissingDuringRevalidation
        | DockerResourceKind::StorageDirectory
        | DockerResourceKind::Unknown => Safety::Blocked,
    }
}

pub fn safety_taxonomy() -> &'static [(DockerResourceKind, Safety)] {
    &[
        (DockerResourceKind::BuildCache, Safety::Caution),
        (DockerResourceKind::DanglingImage, Safety::Caution),
        (
            DockerResourceKind::StoppedAnonymousContainer,
            Safety::Caution,
        ),
        (DockerResourceKind::TaggedImage, Safety::ReportOnly),
        (DockerResourceKind::NamedContainer, Safety::ReportOnly),
        (DockerResourceKind::Volume, Safety::ReportOnly),
        (DockerResourceKind::Network, Safety::ReportOnly),
        (DockerResourceKind::RunningOrInUse, Safety::Blocked),
        (DockerResourceKind::PermissionDenied, Safety::Blocked),
        (
            DockerResourceKind::MissingDuringRevalidation,
            Safety::Blocked,
        ),
        (DockerResourceKind::StorageDirectory, Safety::Blocked),
        (DockerResourceKind::Unknown, Safety::Blocked),
    ]
}

#[derive(Debug, Clone)]
pub enum DockerDoctorStatus {
    Available { server_version: Option<String> },
    Skipped { reason: String },
}

pub fn probe_for_doctor(timeout: Duration) -> DockerDoctorStatus {
    let program = docker_program(None);
    match probe(&program, timeout) {
        DockerStatus::Available { server_version } => {
            DockerDoctorStatus::Available { server_version }
        }
        status => DockerDoctorStatus::Skipped {
            reason: status_reason(&status),
        },
    }
}

pub fn report(options: DockerReportOptions) -> DockerReport {
    debug_assert!(
        safety_taxonomy()
            .iter()
            .all(|(kind, safety)| classify_resource(*kind) == *safety)
    );

    let program = docker_program(options.docker_bin.as_deref());
    let timeout = normalized_timeout(options.timeout);
    let generated_at = chrono::Utc::now().to_rfc3339();

    let probe_status = probe(&program, timeout);
    let server_version = match probe_status {
        DockerStatus::Available { server_version } => server_version,
        status => {
            return DockerReport {
                schema_version: 1,
                generated_at,
                status,
                summary: DockerSummary::default(),
                resources: Vec::new(),
            };
        }
    };

    let resources = match collect_resources(&program, timeout) {
        Ok(resources) => resources,
        Err(status) => {
            return DockerReport {
                schema_version: 1,
                generated_at,
                status,
                summary: DockerSummary::default(),
                resources: Vec::new(),
            };
        }
    };
    let summary = summarize(&resources);

    DockerReport {
        schema_version: 1,
        generated_at,
        status: DockerStatus::Available { server_version },
        summary,
        resources,
    }
}

pub fn print_report(report: &DockerReport) -> Result<(), RcleanError> {
    outln!("Docker: {}", report.status.human_label());
    match &report.status {
        DockerStatus::Available { server_version } => {
            if let Some(version) = server_version {
                outln!("Server: {version}");
            }
        }
        status => {
            outln!("Reason: {}", status_reason(status));
            // Never reuse the empty-result sentence here. The probe
            // failed, so rclean knows nothing about reclaimable space;
            // claiming there is none is a wrong answer, not a cautious
            // one (AGENTS.md: no silent degradation).
            outln!(
                "Docker was not queried successfully, so nothing can be reported about reclaimable space. Retry with a longer --timeout."
            );
            return Ok(());
        }
    }

    if report.resources.is_empty() {
        outln!("No Docker cleanup resources reported.");
        return Ok(());
    }

    outln!(
        "{:<28} {:<12} {:>8} {:>14} Reclaimable",
        "Resource",
        "Safety",
        "Count",
        "Size"
    );
    outln!("{}", "-".repeat(84));
    for resource in &report.resources {
        outln!(
            "{:<28} {:<12} {:>8} {:>14} {}",
            resource.resource_id,
            resource.safety,
            resource
                .total_count
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            resource.size.as_deref().unwrap_or("-"),
            resource.reclaimable.as_deref().unwrap_or("-"),
        );
    }
    outln!();
    outln!("Docker report is inspect-only. rclean does not delete Docker resources.");
    Ok(())
}

fn docker_program(override_path: Option<&Path>) -> PathBuf {
    if let Some(path) = override_path {
        return path.to_path_buf();
    }
    std::env::var_os("RCLEAN_DOCKER_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("docker"))
}

fn normalized_timeout(timeout: Duration) -> Duration {
    if timeout.is_zero() {
        DEFAULT_TIMEOUT
    } else {
        timeout
    }
}

fn probe(program: &Path, timeout: Duration) -> DockerStatus {
    let args = ["version", "--format", "{{json .Server}}"];
    match run_docker_command(program, &args, timeout) {
        Ok(output) => DockerStatus::Available {
            server_version: parse_server_version(&output.stdout),
        },
        Err(err) => status_from_command_error(err),
    }
}

fn collect_resources(
    program: &Path,
    timeout: Duration,
) -> Result<Vec<DockerResourceReport>, DockerStatus> {
    let system_df = run_docker_command(
        program,
        &["system", "df", "--format", "{{json .}}"],
        timeout,
    )
    .map_err(status_from_command_error)?;
    let mut resources = parse_system_df(&system_df.stdout)?;

    let dangling_images = count_json_lines(
        program,
        &[
            "image",
            "ls",
            "--filter",
            "dangling=true",
            "--format",
            "{{json .}}",
        ],
        timeout,
    )?;
    resources.push(count_resource(
        "docker.dangling_images",
        "dangling images",
        DockerResourceKind::DanglingImage,
        dangling_images,
        "daemon-reported dangling images; report-only in this release",
    ));

    let stopped_containers = count_json_lines(
        program,
        &[
            "container",
            "ls",
            "--all",
            "--filter",
            "status=exited",
            "--format",
            "{{json .}}",
        ],
        timeout,
    )?;
    resources.push(count_resource(
        "docker.stopped_containers",
        "stopped containers",
        DockerResourceKind::NamedContainer,
        stopped_containers,
        "stopped containers may be named or user-managed; never selected by default",
    ));

    let networks = count_json_lines(
        program,
        &["network", "ls", "--format", "{{json .}}"],
        timeout,
    )?;
    resources.push(count_resource(
        "docker.networks",
        "networks",
        DockerResourceKind::Network,
        networks,
        "Docker networks are daemon-managed resources; never selected by default",
    ));

    Ok(resources)
}

fn parse_system_df(raw: &str) -> Result<Vec<DockerResourceReport>, DockerStatus> {
    let mut resources = Vec::new();
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let row: SystemDfRow =
            serde_json::from_str(line).map_err(|source| DockerStatus::Error {
                reason: format!("docker system df returned malformed JSON: {source}"),
            })?;
        resources.push(resource_from_system_df(row));
    }
    Ok(resources)
}

fn resource_from_system_df(row: SystemDfRow) -> DockerResourceReport {
    let (resource_id, kind, reason) = match row.type_name.as_str() {
        "Build Cache" => (
            "docker.build_cache",
            DockerResourceKind::BuildCache,
            "Docker build cache is daemon-owned; report-only until deletion has a separate contract",
        ),
        "Images" => (
            "docker.images",
            DockerResourceKind::TaggedImage,
            "Docker image totals may include tagged or named images; never selected by default",
        ),
        "Containers" => (
            "docker.containers",
            DockerResourceKind::NamedContainer,
            "Docker container totals may include named or user-managed containers",
        ),
        "Local Volumes" => (
            "docker.local_volumes",
            DockerResourceKind::Volume,
            "Docker volumes may contain user data; never selected by default",
        ),
        _ => (
            "docker.unknown",
            DockerResourceKind::Unknown,
            "unknown Docker resource type; blocked from cleanup",
        ),
    };

    DockerResourceReport {
        resource_id: resource_id.to_string(),
        label: row.type_name,
        safety: classify_resource(kind),
        selected: false,
        total_count: row.total_count.as_deref().and_then(parse_count),
        active_count: row.active.as_deref().and_then(parse_count),
        size: row.size.filter(|value| !value.trim().is_empty()),
        reclaimable: row.reclaimable.filter(|value| !value.trim().is_empty()),
        reasons: vec![reason.to_string()],
        warnings: Vec::new(),
    }
}

fn count_resource(
    resource_id: &str,
    label: &str,
    kind: DockerResourceKind,
    count: u64,
    reason: &str,
) -> DockerResourceReport {
    DockerResourceReport {
        resource_id: resource_id.to_string(),
        label: label.to_string(),
        safety: classify_resource(kind),
        selected: false,
        total_count: Some(count),
        active_count: None,
        size: None,
        reclaimable: None,
        reasons: vec![reason.to_string()],
        warnings: Vec::new(),
    }
}

fn count_json_lines(program: &Path, args: &[&str], timeout: Duration) -> Result<u64, DockerStatus> {
    let output = run_docker_command(program, args, timeout).map_err(status_from_command_error)?;
    let mut count = 0_u64;
    for line in output.stdout.lines().filter(|line| !line.trim().is_empty()) {
        serde_json::from_str::<serde_json::Value>(line).map_err(|source| DockerStatus::Error {
            reason: format!("docker command returned malformed JSON: {source}"),
        })?;
        count += 1;
    }
    Ok(count)
}

fn summarize(resources: &[DockerResourceReport]) -> DockerSummary {
    let mut summary = DockerSummary {
        resources: resources.len(),
        ..DockerSummary::default()
    };
    for resource in resources {
        match resource.safety {
            Safety::Caution => summary.caution_resources += 1,
            Safety::ReportOnly => summary.report_only_resources += 1,
            Safety::Blocked => summary.blocked_resources += 1,
            Safety::Safe | Safety::Unknown => {}
        }
    }
    summary
}

#[derive(Debug, Deserialize)]
struct SystemDfRow {
    #[serde(rename = "Type")]
    type_name: String,
    #[serde(rename = "TotalCount")]
    total_count: Option<String>,
    #[serde(rename = "Active")]
    active: Option<String>,
    #[serde(rename = "Size")]
    size: Option<String>,
    #[serde(rename = "Reclaimable")]
    reclaimable: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ServerVersion {
    #[serde(rename = "Version")]
    version: Option<String>,
}

fn parse_server_version(raw: &str) -> Option<String> {
    serde_json::from_str::<ServerVersion>(raw.trim())
        .ok()
        .and_then(|server| server.version)
        .filter(|version| !version.trim().is_empty())
}

fn parse_count(raw: &str) -> Option<u64> {
    raw.trim().parse().ok()
}

fn status_reason(status: &DockerStatus) -> String {
    match status {
        DockerStatus::Available { .. } => "Docker daemon is available".to_string(),
        DockerStatus::Unavailable { reason }
        | DockerStatus::PermissionDenied { reason }
        | DockerStatus::Error { reason } => reason.clone(),
        DockerStatus::TimedOut {
            command,
            timeout_ms,
        } => format!("{command} timed out after {timeout_ms}ms"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The clap default is what actually applies on the CLI path and
    /// `DEFAULT_TIMEOUT` is what applies to
    /// `DockerReportOptions::default()`. If they drift, `--help`
    /// advertises one bound while some call sites use another.
    #[test]
    fn default_timeout_matches_cli_default() {
        use clap::Parser as _;

        let cli = crate::cli::Cli::parse_from(["rclean", "docker", "report"]);
        let crate::cli::Commands::Docker(args) = cli.command.expect("docker command") else {
            panic!("expected the docker subcommand");
        };
        let crate::cli::DockerCommands::Report(report) = args.command;

        let parsed = crate::parse::parse_timeout_duration(&report.timeout)
            .expect("the clap default must be a parseable duration");
        assert_eq!(
            parsed,
            DEFAULT_TIMEOUT,
            "--timeout default ({}) and docker::DEFAULT_TIMEOUT ({}s) disagree",
            report.timeout,
            DEFAULT_TIMEOUT.as_secs()
        );
    }

    #[test]
    fn docker_taxonomy_keeps_no_resource_safe() {
        for kind in [
            DockerResourceKind::BuildCache,
            DockerResourceKind::DanglingImage,
            DockerResourceKind::StoppedAnonymousContainer,
            DockerResourceKind::TaggedImage,
            DockerResourceKind::NamedContainer,
            DockerResourceKind::Volume,
            DockerResourceKind::Network,
            DockerResourceKind::RunningOrInUse,
            DockerResourceKind::PermissionDenied,
            DockerResourceKind::MissingDuringRevalidation,
            DockerResourceKind::StorageDirectory,
            DockerResourceKind::Unknown,
        ] {
            assert_ne!(classify_resource(kind), Safety::Safe);
        }
    }

    #[test]
    fn stale_or_permission_denied_resources_are_blocked() {
        assert_eq!(
            classify_resource(DockerResourceKind::MissingDuringRevalidation),
            Safety::Blocked
        );
        assert_eq!(
            classify_resource(DockerResourceKind::PermissionDenied),
            Safety::Blocked
        );
    }

    #[test]
    fn volumes_and_named_resources_are_report_only() {
        assert_eq!(
            classify_resource(DockerResourceKind::Volume),
            Safety::ReportOnly
        );
        assert_eq!(
            classify_resource(DockerResourceKind::Network),
            Safety::ReportOnly
        );
        assert_eq!(
            classify_resource(DockerResourceKind::NamedContainer),
            Safety::ReportOnly
        );
        assert_eq!(
            classify_resource(DockerResourceKind::TaggedImage),
            Safety::ReportOnly
        );
    }

    #[test]
    fn parses_system_df_rows_into_conservative_resources() {
        let raw = r#"{"Type":"Images","TotalCount":"2","Active":"1","Size":"1GB","Reclaimable":"500MB (50%)"}
{"Type":"Build Cache","TotalCount":"3","Active":"0","Size":"2GB","Reclaimable":"2GB"}"#;

        let resources = parse_system_df(raw).unwrap();

        assert_eq!(resources.len(), 2);
        assert_eq!(resources[0].resource_id, "docker.images");
        assert_eq!(resources[0].safety, Safety::ReportOnly);
        assert_eq!(resources[1].resource_id, "docker.build_cache");
        assert_eq!(resources[1].safety, Safety::Caution);
        assert!(!resources[1].selected);
    }
}
