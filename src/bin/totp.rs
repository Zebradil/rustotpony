extern crate clap;
extern crate ctrlc;
extern crate dirs;
extern crate rpassword;
extern crate rustotpony;

use clap::{Parser, Subcommand};
use rustotpony::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CONFIG_PATH: &str = ".rustotpony/db.json";

#[derive(Parser)]
#[command(name = "🐴 RusTOTPony")]
#[command(author, about, version, long_about = None)]
#[command(help_template = "\
{before-help}{name} {version}

{about-with-newline}
{usage-heading} {usage}
{all-args}

By {author-with-newline}{after-help}\
")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show realtime dashboard with all generators
    Dash {},
    /// List all generators
    List {},
    /// Add a new generator
    Add {
        /// Name of the generator
        name: String,
        /// Name of the user
        username: Option<String>,
    },
    /// Delete a generator
    Delete {
        /// Name of the generator
        name: String,
    },
    /// Rename a generator
    Rename {
        /// Current name of the generator
        name: String,
        /// New name of the generator
        new_name: String,
    },
    /// Delete all generators
    Eradicate {},
    /// Export all generators as JSON
    Export {},
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Dash {}) => {
            show_dashboard();
        }
        Some(Commands::List {}) => {
            show_applications_list(false);
        }
        Some(Commands::Add { name, username }) => {
            create_application(name, username.as_deref().unwrap_or(""));
        }
        Some(Commands::Delete { name }) => {
            delete_application(name);
        }
        Some(Commands::Rename { name, new_name }) => {
            rename_application(name, new_name);
        }
        Some(Commands::Eradicate {}) => {
            eradicate_database();
        }
        Some(Commands::Export {}) => {
            export_database();
        }
        _ => {
            show_dashboard();
        }
    }
}

fn app() -> RusTOTPony<JsonDatabase> {
    let db = JsonDatabase::new(get_database_path(), &get_secret);
    RusTOTPony::new(db)
}

fn get_secret() -> String {
    rpassword::prompt_password("Enter your database pass: ").unwrap()
}

fn get_database_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(Path::new(CONFIG_PATH))
}

fn show_dashboard() {
    match app().get_applications() {
        Ok(apps) => {
            let mut is_first_iteration = true;
            let lines_count = apps.len() + 1;
            println!("Welcome to RusTOTPony realtime dashboard! Press ^C to quit.");
            ctrlc::set_handler(move || {
                print!("\x1B[{}A\x1B[0G\x1B[0J", lines_count + 1);
                println!("I won't tell anyone about this 🤫");
                std::process::exit(0);
            })
            .expect("Error setting Ctrl-C handler");
            // Prepare sorted keys for displaying apps in order
            let mut keys: Vec<String> = apps.keys().cloned().collect();
            keys.sort();
            loop {
                if is_first_iteration {
                    is_first_iteration = false;
                } else {
                    print!("\x1B[{}A", lines_count);
                }
                print_progress_bar();
                for key in keys.iter() {
                    let app = &apps[key];
                    println! {"{} {}", app.get_code(), app.get_name()};
                }
                thread::sleep(Duration::from_millis(100));
            }
        }
        Err(err) => println!("{}", err),
    }
}

fn print_progress_bar() {
    let width = 60;
    let now = SystemTime::now();
    let since_the_epoch = now.duration_since(UNIX_EPOCH).unwrap();
    let in_ms =
        since_the_epoch.as_secs() * 1000 + since_the_epoch.subsec_nanos() as u64 / 1_000_000;
    let step = in_ms % 30_000;
    let idx = step * width / 30_000;
    println!("[{:60}]", "=".repeat(idx as usize));
}

fn show_applications_list(_: bool) {
    // TODO Create Table structure with HashMap as follows and metadata about columns - width, titles, names
    let app = app();
    let mut output_table: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut applications_count = 0;
    let apps = match app.get_applications() {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };
    for application in apps.values() {
        applications_count += 1;
        output_table
            .entry("name")
            .or_insert_with(Vec::new)
            .push(application.get_name());
        output_table
            .entry("key")
            .or_insert_with(Vec::new)
            .push(application.get_secret());
        output_table
            .entry("username")
            .or_insert_with(Vec::new)
            .push(application.get_username());
    }
    let name_max_length = output_table["name"]
        .iter()
        .fold("name".len(), |max, val| std::cmp::max(max, val.len()));
    let key_max_length = output_table["key"]
        .iter()
        .fold("key".len(), |max, val| std::cmp::max(max, val.len()));
    let username_max_length = output_table["username"]
        .iter()
        .fold("username".len(), |max, val| std::cmp::max(max, val.len()));
    let header_row_delimiter = format!(
        "+-{}-+-{}-+-{}-+",
        "-".repeat(name_max_length),
        "-".repeat(key_max_length),
        "-".repeat(username_max_length)
    );

    println!("{}", header_row_delimiter);
    println!(
        "| {name:<name_width$} | {key: <key_width$} | {username: <username_width$} |",
        name_width = name_max_length,
        name = "name",
        key_width = key_max_length,
        key = "key",
        username_width = username_max_length,
        username = "username"
    );
    println!("{}", header_row_delimiter);

    for i in 0..applications_count {
        let name = &output_table["name"][i];
        let key = &output_table["key"][i];
        let username = &output_table["username"][i];
        println!(
            "| {name:<name_width$} | {key: <key_width$} | {username: <username_width$} |",
            name_width = name_max_length,
            name = name,
            key_width = key_max_length,
            key = key,
            username_width = username_max_length,
            username = username
        );
    }
    println!("{}", header_row_delimiter);
}

fn create_application(name: &str, username: &str) {
    let secret = rpassword::prompt_password("Enter your secret code: ").unwrap();
    let mut app = app();
    match app.create_application(name, username, &secret) {
        Ok(_) => {
            app.flush();
            println!("New application created: {}", name)
        }
        Err(err) => println!("{} Aborting…", err),
    }
}

fn delete_application(name: &str) {
    let mut app = app();
    match app.delete_application(name) {
        Ok(_) => {
            app.flush();
            println!("Application '{}' successfully deleted", name)
        }
        Err(err) => println!("Couldn't delete application '{}': {}", name, err),
    };
}

fn rename_application(name: &str, newname: &str) {
    let mut app = app();
    match app.rename_application(name, newname) {
        Ok(_) => {
            app.flush();
            println!(
                "Application '{}' successfully renamed to '{}'",
                name, newname
            )
        }
        Err(err) => println!("Couldn't rename application '{}': {}", name, err),
    };
}

fn eradicate_database() {
    let mut app = app();
    app.delete_all_applications();
    app.flush();
    println!("Done.");
}

/// Export database as JSON
fn export_database() {
    let app = app();
    let apps = match app.get_applications() {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };
    let json = serde_json::to_string_pretty(&apps).unwrap();
    println!("{}", json);
}
