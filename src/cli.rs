use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "duplicate",
    version,
    about = "Run N copies of a command and show split-screen output.",
    trailing_var_arg = true,
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Number of instances to run
    #[arg(short = 'n', long = "n", default_value_t = 2)]
    pub n: usize,

    /// Command to run (e.g. ls, curl, rustlens)
    pub cmd: String,

    /// Remaining args (common args + per-instance args)
    pub args: Vec<String>,
}