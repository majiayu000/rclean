use std::process::Command;

use crate::model::{ApfsContainerUsage, DiskAttribution, DiskContributor, VolumeUsage};

#[cfg(target_os = "macos")]
pub(crate) fn collect_disk_attribution() -> Option<DiskAttribution> {
    let mut warnings = Vec::new();

    let apfs_container = match run_output("diskutil", &["apfs", "list"]) {
        Ok(output) => parse_apfs_container(&output),
        Err(err) => {
            warnings.push(err);
            None
        }
    };

    let system_volume = match run_output("df", &["-k", "/"]) {
        Ok(output) => parse_df_usage(&output, "System volume", "/").or_else(|| {
            warnings.push("failed to parse `df -k /` output".to_string());
            None
        }),
        Err(err) => {
            warnings.push(err);
            None
        }
    };

    let data_volume = match run_output("df", &["-k", "/System/Volumes/Data"]) {
        Ok(output) => {
            parse_df_usage(&output, "Data volume", "/System/Volumes/Data").or_else(|| {
                warnings.push("failed to parse `df -k /System/Volumes/Data` output".to_string());
                None
            })
        }
        Err(err) => {
            warnings.push(err);
            None
        }
    };

    Some(DiskAttribution {
        platform: "macos".to_string(),
        apfs_container,
        system_volume,
        data_volume,
        data_contributors: data_contributors(&mut warnings),
        warnings,
    })
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn collect_disk_attribution() -> Option<DiskAttribution> {
    None
}

#[cfg(target_os = "macos")]
fn data_contributors(warnings: &mut Vec<String>) -> Vec<DiskContributor> {
    let mut paths = vec![
        ("Applications", "/Applications".to_string()),
        ("Library", "/Library".to_string()),
        ("Homebrew", "/opt/homebrew".to_string()),
        ("private var", "/private/var".to_string()),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        paths.insert(0, ("user home", home.to_string_lossy().to_string()));
    }

    paths
        .into_iter()
        .map(|(label, path)| DiskContributor {
            label: label.to_string(),
            bytes: du_kib(&path, warnings),
            path,
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn du_kib(path: &str, warnings: &mut Vec<String>) -> Option<u64> {
    let output = match run_output("du", &["-sk", path]) {
        Ok(output) => output,
        Err(err) => {
            warnings.push(err);
            return None;
        }
    };
    parse_du_kib(&output).or_else(|| {
        warnings.push(format!("failed to parse `du -sk {path}` output"));
        None
    })
}

#[cfg(target_os = "macos")]
fn run_output(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|err| format!("failed to run {program}: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "command failed: {program} {:?}: {}",
            args,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_df_usage(output: &str, label: &str, path: &str) -> Option<VolumeUsage> {
    let line = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .nth(1)?;
    let columns: Vec<&str> = line.split_whitespace().collect();
    if columns.len() < 4 {
        return None;
    }
    let total_bytes = columns.get(1)?.parse::<u64>().ok()?.checked_mul(1024)?;
    let used_bytes = columns.get(2)?.parse::<u64>().ok()?.checked_mul(1024)?;
    let available_bytes = columns.get(3)?.parse::<u64>().ok()?.checked_mul(1024)?;
    Some(VolumeUsage {
        label: label.to_string(),
        path: path.to_string(),
        total_bytes,
        used_bytes,
        available_bytes,
    })
}

fn parse_apfs_container(output: &str) -> Option<ApfsContainerUsage> {
    let capacity_bytes = parse_labeled_bytes(output, "Capacity Ceiling");
    let used_bytes = parse_labeled_bytes(output, "Capacity In Use By Volumes");
    let free_bytes = parse_labeled_bytes(output, "Capacity Not Allocated");
    (capacity_bytes.is_some() || used_bytes.is_some() || free_bytes.is_some()).then_some(
        ApfsContainerUsage {
            capacity_bytes,
            used_bytes,
            free_bytes,
        },
    )
}

fn parse_labeled_bytes(output: &str, label: &str) -> Option<u64> {
    output
        .lines()
        .find(|line| line.contains(label))
        .and_then(parse_parenthesized_bytes)
}

fn parse_parenthesized_bytes(line: &str) -> Option<u64> {
    let (_, after_open) = line.rsplit_once('(')?;
    let (bytes, _) = after_open.split_once(" Bytes")?;
    bytes.trim().parse().ok()
}

fn parse_du_kib(output: &str) -> Option<u64> {
    output
        .split_whitespace()
        .next()?
        .parse::<u64>()
        .ok()?
        .checked_mul(1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_df_k_output() {
        let output = "\
Filesystem   1024-blocks      Used Available Capacity iused ifree %iused  Mounted on
/dev/disk3s5    494384792 376100000  71841000    84%  12345  6789    1%   /System/Volumes/Data
";
        let usage = parse_df_usage(output, "Data volume", "/System/Volumes/Data").unwrap();
        assert_eq!(usage.used_bytes, 376100000 * 1024);
        assert_eq!(usage.available_bytes, 71841000 * 1024);
    }

    #[test]
    fn parses_apfs_container_bytes() {
        let output = "\
    Capacity Ceiling (Size): 494.4 GB (494384795648 Bytes)
    Capacity In Use By Volumes: 422.5 GB (422542893056 Bytes)
    Capacity Not Allocated: 71.8 GB (71841894400 Bytes)
";
        let usage = parse_apfs_container(output).unwrap();
        assert_eq!(usage.capacity_bytes, Some(494384795648));
        assert_eq!(usage.used_bytes, Some(422542893056));
        assert_eq!(usage.free_bytes, Some(71841894400));
    }

    #[test]
    fn parses_du_kib_output() {
        assert_eq!(parse_du_kib("12345\t/Users/me\n"), Some(12345 * 1024));
    }
}
