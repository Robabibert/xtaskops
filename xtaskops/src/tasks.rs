//!
//! Complete xtask tasks such as `docs`, `ci` and others
//!
use crate::ops::{clean_files, get_clean_directory, get_workspace_root, nearest_cargo_dir};
use anyhow::{Context, Result as AnyResult};
use derive_builder::Builder;
use duct::cmd;
use std::fs::create_dir_all;

///
/// Run cargo docs in watch mode
///
/// # Errors
/// Fails if any command fails
///
pub fn docs() -> AnyResult<()> {
    cmd!("cargo", "watch", "-s", "cargo doc --no-deps").run()?;
    Ok(())
}

/// Build a CI run
#[derive(Builder)]
#[builder(setter(into))]
pub struct CI {
    /// run with nightly
    /// default: on
    #[builder(default = "false")]
    pub nightly: bool,

    /// turn all clippy lints on: pedantic, nursery, 2018-idioms
    /// default: on
    #[builder(default = "true")]
    pub clippy_max: bool,
}

impl CIBuilder {
    /// Runs this builder
    ///
    /// # Errors
    ///
    /// This function will return an error if run failed
    pub fn run(&self) -> AnyResult<()> {
        let t = self.build()?;
        let mut check_args = vec!["fmt", "--all", "--", "--check"];
        if t.nightly {
            check_args.insert(0, "+nightly");
        }

        let mut clippy_args = vec!["clippy", "--", "-D", "warnings"];
        if t.clippy_max {
            clippy_args.extend([
                "-W",
                "clippy::pedantic",
                "-W",
                "clippy::nursery",
                "-W",
                "rust-2018-idioms",
            ]);
        }

        cmd("cargo", check_args.as_slice()).run()?;
        cmd("cargo", clippy_args.as_slice()).run()?;
        cmd!("cargo", "test").run()?;
        cmd!("cargo", "test", "--doc").run()?;
        Ok(())
    }
}

///
/// Run typical CI tasks in series: `fmt`, `clippy`, and tests
///
/// # Errors
/// Fails if any command fails
///
pub fn ci() -> AnyResult<()> {
    CIBuilder::default().run()
}

fn cobertura_total_coverage(filename: &str) -> AnyResult<()> {
    let total_coverage: f32 = cmd!(
        "xmllint",
        "--xpath",
        "string(//coverage/@line-rate)",
        filename
    )
    .read()?
    .parse()?;
    println!("Coverage: {:.2}%", total_coverage);
    Ok(())
}

///
/// Run coverage
///
/// # Errors
/// Fails if any command fails
///
pub fn coverage(fmt: &str) -> AnyResult<()> {
    let project_root = nearest_cargo_dir()?;
    let workspace_root = get_workspace_root()?;

    let coverage_dir = project_root.join("coverage");
    get_clean_directory(&coverage_dir)?;

    let profile_files = coverage_dir.join("cargo-test-%p-%m.profraw");
    let binary_folder = workspace_root.join("target");
    let source_dir = project_root.join("src");

    cmd!("cargo", "test", "--all-features")
        .env("CARGO_TARGET_DIR", binary_folder.clone())
        .env("RUSTFLAGS", "-Cinstrument-coverage")
        .env("LLVM_PROFILE_FILE", profile_files.as_path())
        .run()?;

    println!("ok.");

    if fmt == "profraw" {
        return Ok(());
    }
    println!("=== generating report ===");
    let output_folder = match fmt {
        "html" | "lcov" | "cobertura" | "covdir" => Ok(coverage_dir.clone()),

        _ => Err(anyhow::Error::msg(format!(
            "Please provide a valid output file format found : {fmt}"
        ))),
    }?;

    create_dir_all(output_folder.clone())?;
    cmd!(
        "grcov",
        coverage_dir,
        "--binary-path",
        binary_folder,
        "--source-dir",
        source_dir,
        "--output-types",
        fmt,
        "--branch",
        "--ignore-not-existing",
        "--ignore",
        "../*",
        "--ignore",
        "/*",
        "--ignore",
        "xtask/*",
        "-o",
        output_folder,
    )
    .run()?;
    println!("ok.");

    println!("=== cleaning up ===");
    clean_files("**/*.profraw")?;
    //clean_files("**/*.profraw")?;
    println!("ok.");

    Ok(())
}

/// Build a powerset test
#[derive(Builder)]
#[builder(setter(into))]
pub struct Powerset {
    /// powerset depth
    #[builder(default = "2")]
    pub depth: i32,

    /// dont run with no feature at all
    #[builder(default = "false")]
    pub exclude_no_default_features: bool,
}

impl PowersetBuilder {
    /// Builds and runs a powerset test
    ///
    /// # Errors
    ///
    /// This function will return an error if run failed
    pub fn run(&self) -> AnyResult<()> {
        let t = self.build()?;
        let depth = format!("{}", t.depth);
        let mut common = vec![
            "--workspace",
            "--exclude",
            "xtask",
            "--feature-powerset",
            "--depth",
            &depth,
        ];
        if t.exclude_no_default_features {
            common.push("--exclude-no-default-features");
        }
        cmd(
            "cargo",
            &[
                &["hack", "clippy"],
                common.as_slice(),
                &["--", "-D", "warnings"],
            ]
            .concat(),
        )
        .run()?;
        cmd("cargo", &[&["hack"], common.as_slice(), &["test"]].concat()).run()?;
        cmd(
            "cargo",
            &[&["hack", "test"], common.as_slice(), &["--doc"]].concat(),
        )
        .run()?;
        Ok(())
    }
}

///
/// Perform a CI build with powerset of features
///
/// # Errors
/// Errors if one of the commands failed
///
pub fn powerset() -> AnyResult<()> {
    PowersetBuilder::default().run()
}

///
/// Show biggest crates in release build
///
/// # Errors
/// Errors if the command failed
///
pub fn bloat_deps(package: &str) -> AnyResult<()> {
    cmd!("cargo", "bloat", "--release", "--crates", "-p", package).run()?;
    Ok(())
}

///
/// Show crate build times
///
/// # Errors
/// Errors if the command failed
///
pub fn bloat_time(package: &str) -> AnyResult<()> {
    cmd!("cargo", "bloat", "--time", "-j", "1", "-p", package).run()?;
    Ok(())
}

///
/// Watch changes and after every change: `cargo check`, followed by `cargo test`
/// If `cargo check` fails, tests will not run.
///
/// # Errors
/// Errors if the command failed
///
pub fn dev() -> AnyResult<()> {
    cmd!("cargo", "watch", "-x", "check", "-x", "test").run()?;
    Ok(())
}

///
/// Instal cargo tools
///
/// # Errors
/// Errors if one of the commands failed
///
pub fn install() -> AnyResult<()> {
    cmd!("cargo", "install", "cargo-watch").run()?;
    cmd!("cargo", "install", "cargo-hack").run()?;
    cmd!("cargo", "install", "cargo-bloat").run()?;
    cmd!("rustup", "component", "add", "llvm-tools-preview").run()?;
    cmd!("cargo", "install", "grcov").run()?;
    Ok(())
}

/// Set up a main for your xtask. Uses clap.
/// To customize, look at this function's source and copy it to your
/// own xtask project.
///
/// # Errors
///
/// This function will return an error if any command failed
#[cfg(feature = "clap")]
pub fn main() -> AnyResult<()> {
    use clap::{AppSettings, Arg, Command};
    let cli = Command::new("xtask")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            Command::new("coverage").arg(
                Arg::new("fmt")
                    .short('f')
                    .long("format")
                    .help("choose the format in which the coverage files are generated.\n Valid options are [html,lcov,profraw]")
                    .takes_value(true),
            ),
        ).subcommand(
            Command::new("cobertura_total_coverage").arg(
                Arg::new("file")
                    .short('f')
                    .long("file")
                    .help("Set cobertura file")
                    .takes_value(true),
            ),
        )
        .subcommand(Command::new("vars"))
        .subcommand(Command::new("ci"))
        .subcommand(Command::new("powerset"))
        .subcommand(
            Command::new("bloat-deps").arg(
                Arg::new("package")
                    .short('p')
                    .long("package")
                    .help("package to build")
                    .required(true)
                    .takes_value(true),
            ),
        )
        .subcommand(
            Command::new("bloat-time").arg(
                Arg::new("package")
                    .short('p')
                    .long("package")
                    .help("package to build")
                    .required(true)
                    .takes_value(true),
            ),
        )
        .subcommand(Command::new("docs"));
    let matches = cli.get_matches();

    let root = crate::ops::root_dir();
    let res = match matches.subcommand() {
        Some(("coverage", sm)) => crate::tasks::coverage(
            sm.get_one::<String>("fmt")
                .context("please provide an output file format")?,
        ),
        Some(("cobertura_total_coverage", sm)) => crate::tasks::cobertura_total_coverage(
            sm.get_one::<String>("file")
                .context("please provide an input file ")?,
        ),
        Some(("vars", _)) => {
            println!("root: {root:?}");
            Ok(())
        }
        Some(("ci", _)) => crate::tasks::ci(),
        Some(("docs", _)) => crate::tasks::docs(),
        Some(("powerset", _)) => crate::tasks::powerset(),
        Some(("bloat-deps", sm)) => crate::tasks::bloat_deps(
            sm.get_one::<String>("package")
                .context("please provide a package with -p")?,
        ),
        Some(("bloat-time", sm)) => crate::tasks::bloat_time(
            sm.get_one::<String>("package")
                .context("please provide a package with -p")?,
        ),
        _ => unreachable!("unreachable branch"),
    };
    res
}
