use std::cmp::Reverse;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::cli::RestoreArgs;
use crate::error::RcleanError;
use crate::model::format_bytes;
use crate::stdio::outln;

use super::{Graveyard, GraveyardError, ManifestRecord};

pub fn run_restore(args: RestoreArgs) -> Result<ExitCode, RcleanError> {
    let yard = Graveyard::open(super::default_root());
    if let Some(id) = args.id.as_deref() {
        return run_explicit_restore(&yard, id, args.to.as_deref(), args.dry_run);
    }

    if !(io::stdin().is_terminal() && io::stdout().is_terminal()) {
        return Err(GraveyardError::Generic(
            "restore without --id requires an interactive terminal; use `rclean restore --id <ID>`"
                .to_string(),
        )
        .into());
    }

    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    run_interactive_restore(&yard, args.dry_run, &mut input, &mut output)
}

fn run_explicit_restore(
    yard: &Graveyard,
    id: &str,
    override_target: Option<&Path>,
    dry_run: bool,
) -> Result<ExitCode, RcleanError> {
    if dry_run {
        let record = yard
            .list()?
            .into_iter()
            .find(|record| record.id == id)
            .ok_or_else(|| GraveyardError::GraveNotFound(id.to_string()))?;
        let target = override_target.unwrap_or(&record.original_path);
        outln!(
            "dry-run: would attempt to restore {}: {} -> {} ({})",
            record.id,
            record.grave_path.join("payload").display(),
            target.display(),
            format_bytes(record.size_bytes)
        );
        return Ok(ExitCode::SUCCESS);
    }

    let record = yard.restore_by_id(id, override_target)?;
    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    writeln!(
        stderr,
        "restored {} -> {}",
        record.id,
        record.original_path.display()
    )?;
    Ok(ExitCode::SUCCESS)
}

fn run_interactive_restore<R: BufRead, W: Write>(
    yard: &Graveyard,
    dry_run: bool,
    input: &mut R,
    output: &mut W,
) -> Result<ExitCode, RcleanError> {
    let mut records = yard.list()?;
    sort_newest_first(&mut records);
    if records.is_empty() {
        writeln!(output, "No active graves.")?;
        return Ok(ExitCode::from(3));
    }

    print_numbered_records(&records, output)?;
    write!(output, "Select to restore (for example 1,3 or all), or q: ")?;
    output.flush()?;

    let mut selection = String::new();
    input.read_line(&mut selection)?;
    let Some(indices) = select_record_indices(&selection, records.len())? else {
        return Ok(ExitCode::from(3));
    };
    if indices.is_empty() {
        return Ok(ExitCode::from(3));
    }
    let selected = indices
        .into_iter()
        .map(|index| records[index].clone())
        .collect::<Vec<_>>();
    print_selected_records(&selected, dry_run, output)?;

    if dry_run {
        return Ok(ExitCode::SUCCESS);
    }

    let total = total_size(&selected);
    crate::clean::confirm_prompt_with_io(
        &format!(
            "Confirm: restore {} graves ({})?",
            selected.len(),
            format_bytes(total)
        ),
        "restore cancelled",
        input,
        output,
    )?;

    let result = restore_selected(yard, &selected);
    print_restore_result(&result, output)?;
    if result.skipped.is_empty() && result.failed.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn print_numbered_records<W: Write>(records: &[ManifestRecord], output: &mut W) -> io::Result<()> {
    writeln!(output, "Active graves ({}):", records.len())?;
    writeln!(
        output,
        "  {:>2}  {:<22} {:<20} {:>10}  Original",
        "#", "Id", "Deleted (UTC)", "Size"
    )?;
    for (index, record) in records.iter().enumerate() {
        writeln!(
            output,
            "  {:>2}. {:<22} {:<20} {:>10}  {}",
            index + 1,
            record.id,
            record.deleted_at.format("%Y-%m-%d %H:%M:%S"),
            format_bytes(record.size_bytes),
            record.original_path.display()
        )?;
    }
    Ok(())
}

fn print_selected_records<W: Write>(
    records: &[ManifestRecord],
    dry_run: bool,
    output: &mut W,
) -> io::Result<()> {
    let prefix = if dry_run {
        "dry-run: would attempt to restore"
    } else {
        "Selected to restore"
    };
    writeln!(
        output,
        "{prefix}: {} graves ({})",
        records.len(),
        format_bytes(total_size(records))
    )?;
    for record in records {
        writeln!(
            output,
            "  - {}: {} -> {} ({})",
            record.id,
            record.grave_path.join("payload").display(),
            record.original_path.display(),
            format_bytes(record.size_bytes)
        )?;
    }
    Ok(())
}

fn total_size(records: &[ManifestRecord]) -> u64 {
    records.iter().fold(0_u64, |total, record| {
        total.saturating_add(record.size_bytes)
    })
}

fn select_record_indices(input: &str, count: usize) -> Result<Option<Vec<usize>>, RcleanError> {
    let selection = input.trim();
    if selection.is_empty() || selection.eq_ignore_ascii_case("q") {
        return Ok(None);
    }
    let normalized = if selection.eq_ignore_ascii_case("all") {
        "a"
    } else {
        selection
    };
    Ok(Some(crate::clean::parse_selection(normalized, count)?))
}

#[derive(Debug, Default)]
struct RestoreBatchResult {
    restored: Vec<ManifestRecord>,
    skipped: Vec<(ManifestRecord, String)>,
    failed: Vec<(ManifestRecord, String)>,
}

fn restore_selected(yard: &Graveyard, selected: &[ManifestRecord]) -> RestoreBatchResult {
    let mut result = RestoreBatchResult::default();
    for record in selected {
        match yard.restore_by_id(&record.id, None) {
            Ok(restored) => result.restored.push(restored),
            Err(error) => {
                let reason = error.to_string();
                if matches!(
                    error,
                    GraveyardError::RestoreTargetExists { .. }
                        | GraveyardError::RestoreTargetParentIsSymlink { .. }
                        | GraveyardError::GraveNotFound(_)
                ) {
                    result.skipped.push((record.clone(), reason));
                } else {
                    result.failed.push((record.clone(), reason));
                }
            }
        }
    }
    result
}

fn print_restore_result<W: Write>(result: &RestoreBatchResult, output: &mut W) -> io::Result<()> {
    writeln!(
        output,
        "Restore result: {} restored, {} skipped, {} failed",
        result.restored.len(),
        result.skipped.len(),
        result.failed.len()
    )?;
    for record in &result.restored {
        writeln!(
            output,
            "  restored {} -> {}",
            record.id,
            record.original_path.display()
        )?;
    }
    for (record, reason) in &result.skipped {
        writeln!(
            output,
            "  skipped {} -> {}: {}",
            record.id,
            record.original_path.display(),
            reason
        )?;
    }
    for (record, reason) in &result.failed {
        writeln!(
            output,
            "  failed {} -> {}: {}",
            record.id,
            record.original_path.display(),
            reason
        )?;
    }
    Ok(())
}

fn sort_newest_first(records: &mut [ManifestRecord]) {
    records.sort_by_key(|record| Reverse(record.deleted_at));
}

pub fn filter_records(
    records: Vec<ManifestRecord>,
    older_than: Option<Duration>,
) -> Vec<ManifestRecord> {
    filter_records_at(records, older_than, Utc::now())
}

fn filter_records_at(
    records: Vec<ManifestRecord>,
    older_than: Option<Duration>,
    now: DateTime<Utc>,
) -> Vec<ManifestRecord> {
    let Some(older_than) = older_than else {
        return records;
    };
    records
        .into_iter()
        .filter(|record| {
            now.signed_duration_since(record.deleted_at)
                .to_std()
                .is_ok_and(|age| age > older_than)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};

    use chrono::TimeZone;
    use tempfile::TempDir;

    use super::*;
    use crate::graveyard::GraveInput;

    fn bury_fixture(yard: &Graveyard, path: &Path) -> ManifestRecord {
        fs::create_dir_all(path).unwrap();
        fs::write(path.join("blob"), b"abc").unwrap();
        yard.bury(GraveInput {
            original_path: path,
            size_bytes: 3,
            plan_id: None,
            rule_id: "node.node_modules",
            category: "deps",
            safety_at_delete: "safe",
            risk_score_at_delete: 0.0,
            tool_version: "test",
        })
        .unwrap()
        .record
    }

    fn manifest_record_fixture(id: &str, deleted_at: DateTime<Utc>) -> ManifestRecord {
        ManifestRecord {
            schema_version: 1,
            id: id.to_string(),
            deleted_at,
            expires_at: deleted_at,
            original_path: PathBuf::from(format!("/tmp/{id}")),
            size_bytes: 1,
            plan_id: None,
            rule_id: "node.node_modules".to_string(),
            category: "deps".to_string(),
            safety_at_delete: "safe".to_string(),
            risk_score_at_delete: 0.0,
            tool_version: "test".to_string(),
            grave_path: PathBuf::from(id),
        }
    }

    #[test]
    fn selection_accepts_numbers_ranges_all_and_cancel() {
        assert_eq!(
            select_record_indices("1,3-4,3", 4).unwrap(),
            Some(vec![0, 2, 3])
        );
        assert_eq!(select_record_indices("a", 3).unwrap(), Some(vec![0, 1, 2]));
        assert_eq!(
            select_record_indices("ALL", 3).unwrap(),
            Some(vec![0, 1, 2])
        );
        assert_eq!(select_record_indices("q", 3).unwrap(), None);
        assert_eq!(select_record_indices("", 3).unwrap(), None);
        assert!(select_record_indices("0", 3).is_err());
        assert!(select_record_indices("3-1", 3).is_err());
    }

    #[test]
    fn newest_first_sort_is_stable_for_equal_timestamps() {
        let old = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let new = Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();
        let mut records = vec![
            manifest_record_fixture("old", old),
            manifest_record_fixture("new-first", new),
            manifest_record_fixture("new-second", new),
        ];
        sort_newest_first(&mut records);
        assert_eq!(
            records
                .iter()
                .map(|record| record.id.as_str())
                .collect::<Vec<_>>(),
            ["new-first", "new-second", "old"]
        );
    }

    #[test]
    fn filter_older_than_excludes_fresh_and_future_records() {
        let now = Utc.with_ymd_and_hms(2026, 7, 26, 0, 0, 0).unwrap();
        let records = vec![
            manifest_record_fixture("old", now - chrono::Duration::days(31)),
            manifest_record_fixture("exact", now - chrono::Duration::days(30)),
            manifest_record_fixture("fresh", now - chrono::Duration::days(1)),
            manifest_record_fixture("future", now + chrono::Duration::days(1)),
        ];
        let filtered =
            filter_records_at(records, Some(Duration::from_secs(30 * 24 * 60 * 60)), now);
        assert_eq!(
            filtered
                .iter()
                .map(|record| record.id.as_str())
                .collect::<Vec<_>>(),
            ["old"]
        );
    }

    #[test]
    fn batch_restore_continues_after_a_guarded_skip() {
        let temp = TempDir::new().unwrap();
        let yard = Graveyard::open(temp.path().join("graveyard"));
        let restorable_path = temp.path().join("restorable");
        let occupied_path = temp.path().join("occupied");
        let restorable = bury_fixture(&yard, &restorable_path);
        let occupied = bury_fixture(&yard, &occupied_path);
        fs::create_dir(&occupied_path).unwrap();

        let result = restore_selected(&yard, &[occupied.clone(), restorable.clone()]);

        assert_eq!(result.restored.len(), 1);
        assert_eq!(result.restored[0].id, restorable.id);
        assert_eq!(result.skipped.len(), 1);
        assert_eq!(result.skipped[0].0.id, occupied.id);
        assert!(result.failed.is_empty());
        assert!(restorable_path.is_dir());
        assert!(occupied_path.is_dir());
        assert_eq!(yard.list().unwrap().len(), 1);
    }

    #[test]
    fn interactive_partial_restore_reports_all_buckets_and_exits_one() {
        let temp = TempDir::new().unwrap();
        let yard = Graveyard::open(temp.path().join("graveyard"));
        let restorable_path = temp.path().join("restorable");
        let occupied_path = temp.path().join("occupied");
        bury_fixture(&yard, &restorable_path);
        bury_fixture(&yard, &occupied_path);
        fs::create_dir(&occupied_path).unwrap();
        let mut input = Cursor::new(b"all\ny\n".to_vec());
        let mut output = Vec::new();

        let status = run_interactive_restore(&yard, false, &mut input, &mut output).unwrap();

        assert_eq!(status, ExitCode::FAILURE);
        assert!(restorable_path.is_dir());
        assert!(occupied_path.is_dir());
        assert_eq!(yard.list().unwrap().len(), 1);
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("1 restored, 1 skipped, 0 failed"));
        assert!(text.contains("restored "));
        assert!(text.contains("skipped "));
    }

    #[test]
    fn interactive_cancel_and_empty_graveyard_return_exit_three_without_writing() {
        let temp = TempDir::new().unwrap();
        let yard = Graveyard::open(temp.path().join("graveyard"));
        let target = temp.path().join("target");
        bury_fixture(&yard, &target);
        let mut cancel_input = Cursor::new(b"q\n".to_vec());
        let mut cancel_output = Vec::new();

        let cancelled =
            run_interactive_restore(&yard, false, &mut cancel_input, &mut cancel_output).unwrap();
        assert_eq!(cancelled, ExitCode::from(3));
        assert!(!target.exists());
        assert_eq!(yard.list().unwrap().len(), 1);

        let empty = Graveyard::open(temp.path().join("empty-graveyard"));
        let mut empty_input = Cursor::new(Vec::<u8>::new());
        let mut empty_output = Vec::new();
        let empty_status =
            run_interactive_restore(&empty, false, &mut empty_input, &mut empty_output).unwrap();
        assert_eq!(empty_status, ExitCode::from(3));
        assert_eq!(
            String::from_utf8(empty_output).unwrap(),
            "No active graves.\n"
        );
    }

    #[test]
    fn interactive_dry_run_selects_without_restoring_or_confirming() {
        let temp = TempDir::new().unwrap();
        let yard = Graveyard::open(temp.path().join("graveyard"));
        let target = temp.path().join("target");
        bury_fixture(&yard, &target);
        let mut input = Cursor::new(b"all\n".to_vec());
        let mut output = Vec::new();

        let status = run_interactive_restore(&yard, true, &mut input, &mut output).unwrap();

        assert_eq!(status, ExitCode::SUCCESS);
        assert!(!target.exists());
        assert_eq!(yard.list().unwrap().len(), 1);
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("would attempt to restore"));
        assert!(!text.contains("[y/N]"));
    }

    #[test]
    fn interactive_restore_requires_confirmation_before_writing() {
        let temp = TempDir::new().unwrap();
        let yard = Graveyard::open(temp.path().join("graveyard"));
        let target = temp.path().join("target");
        bury_fixture(&yard, &target);
        let mut input = Cursor::new(b"1\nn\n".to_vec());
        let mut output = Vec::new();

        let error = run_interactive_restore(&yard, false, &mut input, &mut output)
            .expect_err("declined confirmation must cancel");

        assert!(error.to_string().contains("restore cancelled"));
        assert!(!target.exists());
        assert_eq!(yard.list().unwrap().len(), 1);
    }

    #[test]
    fn interactive_restore_moves_selected_record_after_confirmation() {
        let temp = TempDir::new().unwrap();
        let yard = Graveyard::open(temp.path().join("graveyard"));
        let target = temp.path().join("target");
        bury_fixture(&yard, &target);
        let mut input = Cursor::new(b"1\ny\n".to_vec());
        let mut output = Vec::new();

        let status = run_interactive_restore(&yard, false, &mut input, &mut output).unwrap();

        assert_eq!(status, ExitCode::SUCCESS);
        assert!(target.is_dir());
        assert!(yard.list().unwrap().is_empty());
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("1 restored, 0 skipped, 0 failed")
        );
    }
}
