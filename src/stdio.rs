use std::fmt::Arguments;
use std::io::{self, Write};
use std::process::ExitCode;

use crate::error::RcleanError;

pub(crate) fn write_line(args: Arguments<'_>) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    stdout.write_fmt(args)?;
    stdout.write_all(b"\n")
}

pub(crate) fn write_bytes(bytes: &[u8]) -> io::Result<()> {
    io::stdout().lock().write_all(bytes)
}

pub(crate) fn finish_output(
    status: ExitCode,
    result: Result<(), RcleanError>,
) -> Result<ExitCode, RcleanError> {
    match result {
        Ok(()) => Ok(status),
        Err(error) if is_broken_pipe(&error) => Ok(status),
        Err(error) => Err(error),
    }
}

pub(crate) fn continue_after_output(result: Result<(), RcleanError>) -> Result<bool, RcleanError> {
    match result {
        Ok(()) => Ok(true),
        Err(error) if is_broken_pipe(&error) => Ok(false),
        Err(error) => Err(error),
    }
}

pub(crate) fn is_broken_pipe(error: &RcleanError) -> bool {
    error.output_io_kind() == Some(io::ErrorKind::BrokenPipe)
}

macro_rules! outln {
    () => {
        $crate::stdio::write_line(format_args!(""))?
    };
    ($($arg:tt)*) => {
        $crate::stdio::write_line(format_args!($($arg)*))?
    };
}

pub(crate) use outln;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broken_pipe_preserves_the_supplied_status() {
        for code in [1, 3, 4] {
            let status = ExitCode::from(code);
            let result = Err(RcleanError::OutputIo(io::Error::from(
                io::ErrorKind::BrokenPipe,
            )));

            let Ok(actual) = finish_output(status, result) else {
                panic!("BrokenPipe must preserve status");
            };
            assert_eq!(actual, status);
        }
    }

    #[test]
    fn non_broken_output_error_remains_visible() {
        let result = Err(RcleanError::OutputIo(io::Error::from(
            io::ErrorKind::PermissionDenied,
        )));

        assert!(matches!(
            finish_output(ExitCode::SUCCESS, result),
            Err(RcleanError::OutputIo(error))
                if error.kind() == io::ErrorKind::PermissionDenied
        ));
    }
}
