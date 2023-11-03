mod arm_embedded_gcc_install;
mod cmake_install;
mod cmake_lists_template_expender;
mod cmake_project_generator;
mod cproject_reader;
mod error;
mod gh_helper;
mod ninja_install;
mod openocd_install;
mod path_env;
mod simple_template;
use clap::{Parser, Subcommand};
use error::InstallError;
use std::{ffi::OsStr, fs, path::PathBuf};
use which::which_in_global;

use crate::{
    arm_embedded_gcc_install::install_arm_embedded_gcc, cmake_install::install_cmake,
    cmake_project_generator::CMakeProjectGeneratorParams, cproject_reader::read_cproject_file,
    ninja_install::install_ninja, openocd_install::install_openocd,
    path_env::check_path_env_permission,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Env {
        #[command(subcommand)]
        command: EnvCommands,
    },
    Tesseract {
        #[clap(short, long)]
        file: String,
    },
}

#[derive(Subcommand)]
enum EnvCommands {
    Check {},
    Up {},
}

fn check_tool<U>(name: &str, command: &str, path_var: Option<U>) -> bool
where
    U: AsRef<OsStr>,
{
    println!("====== {} ======", name);
    match which_in_global(command, path_var)
        .and_then(|mut i| i.next().ok_or(which::Error::CannotFindBinaryPath))
    {
        Ok(path) => {
            println!("Found: {}", path.display());
            true
        }
        Err(_) => {
            println!("Not found");
            false
        }
    }
}

fn check_and_install<I, U>(
    name: &str,
    command: &str,
    install: I,
    path_var: Option<U>,
) -> Result<(), InstallError>
where
    I: FnOnce() -> Result<(), InstallError>,
    U: AsRef<OsStr>,
{
    println!("====== {} ======", name);
    match which_in_global(command, path_var)
        .and_then(|mut i| i.next().ok_or(which::Error::CannotFindBinaryPath))
    {
        Ok(path) => {
            println!("Found: {}", path.display());
            Ok(())
        }
        Err(_) => {
            println!("Not found, installing {}...", name);

            if let Err(e) = check_path_env_permission() {
                if cfg!(not(debug_assertions)) {
                    println!("Permission denied to modify environment variables");
                    return Err(InstallError::IOFailed(e));
                }
                println!("Permission denied to modify environment variables, continue anyway in debug mode");
            }

            if let Err(e) = install() {
                println!("Error {}", e);
                Err(e)
            } else {
                Ok(())
            }
        }
    }
}

fn command_env_check() {
    let path_var = path_env::get_path_env();
    let mut okey = true;
    okey &= check_tool("Ninja", "ninja", path_var.as_ref());
    okey &= check_tool("OpenOCD", "openocd", path_var.as_ref());
    okey &= check_tool(
        "GNU Arm Embedded GCC",
        "arm-none-eabi-gcc",
        path_var.as_ref(),
    );
    okey &= check_tool("CMake", "cmake", path_var.as_ref());
    println!("====== Conclusion ======");
    if okey {
        println!("All good");
    } else {
        println!("Something went wrong");
        std::process::exit(1);
    }
}

fn command_env_up() {
    let path_var = path_env::get_path_env();
    let mut okey = true;
    okey &= check_and_install("Ninja", "ninja", install_ninja, path_var.as_ref()).is_ok();
    okey &= check_and_install("OpenOCD", "openocd", install_openocd, path_var.as_ref()).is_ok();
    okey &= check_and_install(
        "GNU Arm Embedded GCC",
        "arm-none-eabi-gcc",
        install_arm_embedded_gcc,
        path_var.as_ref(),
    )
    .is_ok();
    okey &= check_and_install("CMake", "cmake", install_cmake, path_var.as_ref()).is_ok();
    println!("====== Conclusion ======");
    if okey {
        println!("All good");
        println!("Note: You may need to reboot your computer to apply new environment variables")
    } else {
        println!("Something went wrong");
        std::process::exit(1);
    }
}

fn command_tesseract(file: &str) {
    let mut cproject_path = PathBuf::from(file);
    if !cproject_path.is_dir() {
        cproject_path.pop();
    }
    cproject_path.push(".cproject");
    let cproject_path = fs::canonicalize(cproject_path).expect("Failed to resolve .cproject");
    let info = read_cproject_file(cproject_path.as_path()).expect("Failed to read cproject file");
    println!("====== Info ======");
    println!("CProjectFile: {}", cproject_path.display());
    println!("Project name: {}", info.project_name);
    println!("Target MCU: {}", info.target_mcu);
    if let Some(mcpu) = &info.target_cpu {
        println!("Target CPU: {}", mcpu);
    } else {
        println!("Target CPU: Unknown");
    }
    println!("Linker script: {}", info.linker_script);
    println!("Defined symbols: {:?}", info.defined_symbols);
    println!("Include paths: {:?}", info.include_paths);
    println!("Source entries: {:?}", info.source_entries);

    println!("====== Generate CMake Project ======");
    cmake_project_generator::generate_cmake_project(
        &CMakeProjectGeneratorParams { info: &info },
        cproject_path.as_path(),
    )
    .expect("Failed to generate CMake project");

    println!("====== Expend CMakeLists Template ======");
    cmake_lists_template_expender::expend_cmake_lists_template(cproject_path.as_path(), &info);
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Env { command } => match command {
            EnvCommands::Check {} => {
                command_env_check();
            }
            EnvCommands::Up {} => {
                command_env_up();
            }
        },
        Commands::Tesseract { file } => {
            command_tesseract(file.as_str());
        }
    }
}
