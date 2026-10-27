//! Folioh Docs' command-line host and documentation pipeline. Distributions
//! supply optional commands and plugins through the same public interfaces.

mod cli;
mod commands;
mod error;
mod pipeline;
mod signals;
mod ui;
mod workflows;
mod workspace_lock;

use std::io::Write;
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{ArgMatches, Command, CommandFactory, FromArgMatches};

use error::CliError;
use ui::text::{Colors, RED};

/// Constructs one plugin for each independent documentation build.
pub type PluginFactory = fn() -> Box<dyn folioh_plugins::Plugin>;

/// An additional command mounted by the distribution alongside Docs' commands.
pub struct CommandExtension {
    pub command: Command,
    pub run: fn(&ArgMatches, &mut dyn Write, bool) -> Result<(), String>,
}

fn command(extensions: &[CommandExtension]) -> Result<Command, clap::Error> {
    let mut command = cli::Cli::command();
    for extension in extensions {
        let name = extension.command.get_name();
        if command.find_subcommand(name).is_some() {
            return Err(command.error(
                ErrorKind::ArgumentConflict,
                format!("Command already registered: {name}"),
            ));
        }
        command = command.subcommand(extension.command.clone());
    }
    Ok(command)
}

fn dispatch_extension(
    matches: &ArgMatches,
    extensions: &[CommandExtension],
    output: &mut dyn Write,
    color: bool,
) -> Option<Result<(), String>> {
    let (name, matches) = matches.subcommand()?;
    extensions
        .iter()
        .find(|extension| extension.command.get_name() == name)
        .map(|extension| (extension.run)(matches, output, color))
}

/// Runs the Docs CLI with the distribution's optional extensions.
pub fn run(extensions: &[CommandExtension], plugins: &[PluginFactory]) -> ExitCode {
    let ui = ui::Ui::detect();
    let matches = match command(extensions).and_then(Command::try_get_matches) {
        Ok(matches) => matches,
        // A bare `folioh` (or a bare group) shows its help on stdout and
        // exits 2, as typer's `no_args_is_help` did; other usage errors
        // keep clap's stderr report.
        Err(err) if err.kind() == ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            return print_help(err.render(), &ui);
        }
        Err(err) => err.exit(),
    };
    // `--update` replaces the binary and runs nothing else, extension
    // commands included; a subcommand next to it is a usage error.
    if matches.get_flag("update") {
        if let Some((name, _)) = matches.subcommand() {
            let mut root = command(extensions).unwrap_or_else(|err| err.exit());
            root.error(
                ErrorKind::ArgumentConflict,
                format!("the argument '--update' cannot be used with the '{name}' command"),
            )
            .exit();
        }
        return report(commands::update::run(&ui), &ui);
    }
    let result = {
        let mut stdout = std::io::stdout().lock();
        let result =
            dispatch_extension(&matches, extensions, &mut stdout, ui.colors != Colors::Off);
        let _ = stdout.flush();
        result
    };
    let result = match result {
        Some(result) => result.map_err(CliError::message),
        None => match cli::Cli::from_arg_matches(&matches) {
            Ok(cli::Cli {
                command: Some(command),
                ..
            }) => commands::run(command, &ui, plugins),
            // Neither a subcommand nor `--update`: clap's
            // `arg_required_else_help` already answers this; kept whole.
            Ok(_) => {
                let mut root = command(extensions).unwrap_or_else(|err| err.exit());
                return print_help(root.render_help(), &ui);
            }
            Err(error) => error.exit(),
        },
    };
    report(result, &ui)
}

/// The help on stdout, coloured for the terminal, exit 2.
fn print_help(help: clap::builder::StyledStr, ui: &ui::Ui) -> ExitCode {
    if ui.colors == Colors::Off {
        print!("{help}");
    } else {
        print!("{}", help.ansi());
    }
    ExitCode::from(2)
}

/// One command's outcome as the process exit: errors in red, their code kept.
fn report(result: Result<(), CliError>, ui: &ui::Ui) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError::Exit(code)) => ExitCode::from(code),
        Err(err) => {
            ui.eprint_styled(RED, &err.to_string());
            ExitCode::from(err.exit_code())
        }
    }
}

#[cfg(test)]
mod tests;
