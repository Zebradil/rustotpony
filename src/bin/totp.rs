extern crate clap;
extern crate ctrlc;
extern crate dirs;
extern crate rpassword;
extern crate rustotpony;

use clap::{App, Arg, SubCommand};
use rustotpony::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CONFIG_PATH: &str = ".rustotpony/db.json";

fn main() {
    Cli::run();
}

struct Cli {}

impl Cli {
    fn app() -> RusTOTPony<JsonDatabase> {
        let db = JsonDatabase::new(Self::get_database_path(), &Self::get_secret);
        RusTOTPony::new(db)
    }

    fn get_secret() -> String {
        rpassword::prompt_password_stdout("Enter your database pass: ").unwrap()
    }

    // fn get_secret_from_storage() -> String { }

    fn run() {
        match Self::get_cli_api_matches().subcommand() {
            ("dash", Some(_)) => {
                Self::show_dashboard();
            }
            ("list", Some(_)) => {
                Self::show_applications_list(false);
            }
            // ("show-all", Some(_)) => {
            //     Self::show_applications_list(true);
            // }
            // ("show", Some(sub_app)) => {
            //     let app_name: &str = sub_app
            //         .value_of("APPNAME")
            //         .expect("Couldn't read APPNAME for 'show' command");
            //     Self::show_application(app_name);
            // }
            ("add", Some(sub_app)) => {
                let app_name: &str = sub_app
                    .value_of("APPNAME")
                    .expect("Couldn't read APPNAME for 'add' command");
                let key: &str = sub_app.value_of("USERNAME").unwrap_or("");
                Self::create_application(app_name, key);
            }
            ("delete", Some(sub_app)) => {
                let app_name: &str = sub_app
                    .value_of("APPNAME")
                    .expect("Couldn't read APPNAME for 'delete' command");
                Self::delete_application(app_name);
            }
            ("rename", Some(sub_app)) => {
                let app_name: &str = sub_app
                    .value_of("APPNAME")
                    .expect("Couldn't read APPNAME for 'rename' command");
                let new_name: &str = sub_app
                    .value_of("NEWNAME")
                    .expect("Couldn't read NEWNAME for 'rename' command");
                Self::rename_application(app_name, new_name);
            }
            ("eradicate", Some(_)) => {
                Self::eradicate_database();
            }
            _ => {
                Self::show_dashboard();
            }
        }
    }

    fn get_cli_api_matches() -> clap::ArgMatches<'static> {
        App::new("🐴  RusTOTPony")
            .version(env!("CARGO_PKG_VERSION"))
            .author("German Lashevich <german.lashevich@gmail.com>")
            .about("CLI manager of one-time password generators aka Google Authenticator")
            .subcommand(
                SubCommand::with_name("dash").about("Show realtime dashboard with all generators"),
            )
            .subcommand(SubCommand::with_name("list").about("List all generators"))
            // .subcommand(
            //     SubCommand::with_name("show-all")
            //         .about("Shows all generators with their's current values"),
            // )
            // .subcommand(
            //     SubCommand::with_name("show")
            //         .about("Shows generator with it's current value")
            //         .arg(Arg::with_name("APPNAME").required(true)),
            // )
            .subcommand(
                SubCommand::with_name("add")
                    .about("Add a new generator")
                    .arg(Arg::with_name("APPNAME").required(true))
                    .arg(Arg::with_name("USERNAME")),
            )
            .subcommand(
                SubCommand::with_name("delete")
                    .about("Delete generator")
                    .arg(Arg::with_name("APPNAME").required(true)),
            )
            .subcommand(
                SubCommand::with_name("rename")
                    .about("Rename generator")
                    .arg(Arg::with_name("APPNAME").required(true))
                    .arg(Arg::with_name("NEWNAME").required(true)),
            )
            .subcommand(SubCommand::with_name("eradicate").about("Delete all generators"))
            .after_help("Try `totp help [SUBCOMMAND]` to see help for the given subcommand")
            .get_matches()
    }

    fn get_database_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(Path::new(CONFIG_PATH))
    }

    fn show_dashboard() {
        match Self::app().get_applications() {
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
                    Self::print_progress_bar();
                    for key in keys.iter() {
                        let app = &apps[key];
                        println! {"{:06} {}", app.get_code(), app.get_name()};
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
        let app = Self::app();
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

    fn show_application(name: &str) {
        println!("{:?}", Self::app().get_application(name));
    }

    fn create_application(name: &str, username: &str) {
        let secret = rpassword::prompt_password_stdout("Enter your secret code: ").unwrap();
        let mut app = Self::app();
        match app.create_application(name, username, &secret) {
            Ok(_) => {
                app.flush();
                println!("New application created: {}", name)
            }
            Err(err) => println!("{} Aborting…", err),
        }
    }

    fn delete_application(name: &str) {
        let mut app = Self::app();
        match app.delete_application(name) {
            Ok(_) => {
                app.flush();
                println!("Application '{}' successfully deleted", name)
            }
            Err(err) => println!("Couldn't delete application '{}': {}", name, err),
        };
    }

    fn rename_application(name: &str, newname: &str) {
        let mut app = Self::app();
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
        let mut app = Self::app();
        app.delete_all_applications();
        app.flush();
        println!("Done.");
    }
}
