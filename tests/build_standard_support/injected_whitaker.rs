//! The direct-Whitaker half of the injected-runner checks: canned `make -n` text whose `lint` target
//! runs a Whitaker command, fed through the injected runner so no process runs.

use super::{
    config::{Pin, THREADS_FLAG},
    development::development_problems_expecting,
    injected::{canned, ensure, linker_flag},
    make::{Host, MakeRunner, Target},
};

/// A fake runner whose `lint` target runs a compliant Clippy command beside the given Whitaker command
/// line; every other target runs a compliant `cargo test`.
fn whitaker_line_make(target: Target<'_>, host: Host, whitaker: &str) -> Result<String, String> {
    let linker = linker_flag(host);
    let compliant =
        format!("RUSTFLAGS=\"${{RUSTFLAGS:+$RUSTFLAGS }}-D warnings {THREADS_FLAG}{linker}\"");
    if target.name() == "lint" {
        canned(format_args!(
            "{compliant} cargo clippy --all-targets\n{whitaker}\n"
        ))
    } else {
        canned(format_args!("{compliant} cargo test\n"))
    }
}

/// A fake runner whose Whitaker command is isolated: it assigns only the warning policy.
fn isolated_whitaker_make(target: Target<'_>, host: Host) -> Result<String, String> {
    whitaker_line_make(target, host, "RUSTFLAGS=\"-D warnings\" whitaker --all")
}

/// A fake runner whose Whitaker command carries the frontend flag alone.
fn frontend_whitaker_make(target: Target<'_>, host: Host) -> Result<String, String> {
    whitaker_line_make(
        target,
        host,
        &format!("RUSTFLAGS=\"-D warnings {THREADS_FLAG}\" whitaker --all"),
    )
}

/// A fake runner whose Whitaker command carries the linker flag alone.
fn linker_whitaker_make(target: Target<'_>, host: Host) -> Result<String, String> {
    whitaker_line_make(
        target,
        host,
        &format!(
            "RUSTFLAGS=\"-D warnings{}\" whitaker --all",
            linker_flag(host)
        ),
    )
}

/// A fake runner whose Whitaker command carries the development flags.
fn contaminated_whitaker_make(target: Target<'_>, host: Host) -> Result<String, String> {
    let flags = format!("-D warnings {THREADS_FLAG}{}", linker_flag(host));
    whitaker_line_make(
        target,
        host,
        &format!("RUSTFLAGS=\"{flags}\" whitaker --all"),
    )
}

/// A fake runner whose Whitaker command inherits the caller's `RUSTFLAGS`.
fn inheriting_whitaker_make(target: Target<'_>, host: Host) -> Result<String, String> {
    whitaker_line_make(
        target,
        host,
        "RUSTFLAGS=\"${RUSTFLAGS:+$RUSTFLAGS }-D warnings\" whitaker --all",
    )
}

/// A fake runner whose Whitaker command inherits the caller's `RUSTFLAGS` by direct expansion.
fn direct_inheriting_whitaker_make(target: Target<'_>, host: Host) -> Result<String, String> {
    whitaker_line_make(
        target,
        host,
        "RUSTFLAGS=\"$RUSTFLAGS -D warnings\" whitaker --all",
    )
}

/// A fake runner whose Whitaker command assigns an empty `RUSTFLAGS`, dropping the warning policy.
fn empty_whitaker_make(target: Target<'_>, host: Host) -> Result<String, String> {
    whitaker_line_make(target, host, "RUSTFLAGS=\"\" whitaker --all")
}

/// Scenario: a direct Whitaker command, whose installer-managed toolchain must not inherit the
/// repository's development flags (Concordat `rust-build-defaults` BD-008).
///
/// Invariant: an isolated command that assigns only the warning policy raises nothing, and a command
/// that carries the frontend or linker flag, inherits the caller's flags or drops the warning policy is
/// refused by target name.
#[test]
fn a_direct_whitaker_command_must_stay_isolated_from_the_development_flags() -> Result<(), String> {
    for host in [Host::Linux, Host::Darwin] {
        let (isolated, _) =
            development_problems_expecting(isolated_whitaker_make, host, Pin::Nightly, &[])?;
        ensure(
            isolated.is_empty(),
            "an isolated Whitaker command was refused",
        )?;
    }
    ensure_contaminated_whitaker_refused()
}

/// Checks that Whitaker commands carrying the development flags, inheriting the caller's flags or
/// dropping the warning policy are each refused for the `make lint` target with the matching reason.
fn ensure_contaminated_whitaker_refused() -> Result<(), String> {
    let refused = [
        (
            contaminated_whitaker_make as MakeRunner,
            "development flags",
        ),
        (frontend_whitaker_make, "development flags"),
        (linker_whitaker_make, "development flags"),
        (inheriting_whitaker_make, "inherit the caller's RUSTFLAGS"),
        (
            direct_inheriting_whitaker_make,
            "inherit the caller's RUSTFLAGS",
        ),
        (empty_whitaker_make, "without -D warnings"),
    ];
    for (runner, reason) in refused {
        let (problems, _) = development_problems_expecting(runner, Host::Linux, Pin::Nightly, &[])?;
        ensure(
            problems
                .iter()
                .any(|problem| problem.contains("`make lint`") && problem.contains(reason)),
            "a Whitaker command that breaks the isolation passed",
        )?;
    }
    Ok(())
}
