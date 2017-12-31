extern crate clap;
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
    Cli::new().run();
}

struct Cli {
    app: RusTOTPony<JsonDatabase>,
}

impl Cli {
    fn new() -> Self {
        let db = JsonDatabase::new(Self::get_database_path());
        Self {
            app: RusTOTPony::new(db),
        }
    }

    fn run(&mut self) {
        match self.get_cli_api_matches().subcommand() {
            ("dash", Some(_)) => {
                self.show_dashboard();
            }
            ("list", Some(_)) => {
                self.show_applications_list(false);
            }
            ("show-all", Some(_)) => {
                self.show_applications_list(true);
            }
            ("show", Some(sub_app)) => {
                let app_name: &str = sub_app
                    .value_of("APPNAME")
                    .expect("Couldn't read APPNAME for 'show' command");
                self.show_application(app_name);
            }
            ("add", Some(sub_app)) => {
                let app_name: &str = sub_app
                    .value_of("APPNAME")
                    .expect("Couldn't read APPNAME for 'add' command");
                let key: &str = sub_app.value_of("USERNAME").unwrap_or("");
                self.create_application(app_name, key);
            }
            ("delete", Some(sub_app)) => {
                let app_name: &str = sub_app
                    .value_of("APPNAME")
                    .expect("Couldn't read APPNAME for 'delete' command");
                self.delete_application(app_name);
            }
            ("rename", Some(sub_app)) => {
                let app_name: &str = sub_app
                    .value_of("APPNAME")
                    .expect("Couldn't read APPNAME for 'rename' command");
                let new_name: &str = sub_app
                    .value_of("NEWNAME")
                    .expect("Couldn't read NEWNAME for 'rename' command");
                self.rename_application(app_name, new_name);
            }
            ("eradicate", Some(_)) => {
                self.eradicate_database();
            }
            _ => {
                self.show_applications_list(true);
            }
        }
    }

    fn get_cli_api_matches(&self) -> clap::ArgMatches<'static> {
        App::new("Main")
            .version("0.1.0")
            .author("German Lashevich <german.lashevich@gmail.com>")
            .about("TOTP code generator written with Rust")
            .subcommand(
                SubCommand::with_name("dash").about("Shows realtime dashboard with all generators"),
            )
            .subcommand(SubCommand::with_name("list").about("Lists all generators"))
            .subcommand(
                SubCommand::with_name("show-all")
                    .about("Shows all generators with their's current values"),
            )
            .subcommand(
                SubCommand::with_name("show")
                    .about("Shows generator with it's current value")
                    .arg(Arg::with_name("APPNAME").required(true)),
            )
            .subcommand(
                SubCommand::with_name("add")
                    .about("Adds new generator")
                    .arg(Arg::with_name("APPNAME").required(true))
                    .arg(Arg::with_name("USERNAME")),
            )
            .subcommand(
                SubCommand::with_name("delete")
                    .about("Deletes generator")
                    .arg(Arg::with_name("APPNAME").required(true)),
            )
            .subcommand(
                SubCommand::with_name("rename")
                    .about("Renames generator")
                    .arg(Arg::with_name("APPNAME").required(true))
                    .arg(Arg::with_name("NEWNAME").required(true)),
            )
            .subcommand(SubCommand::with_name("eradicate").about("Deletes all generators"))
            .get_matches()
    }

    fn get_database_path() -> PathBuf {
        let home = std::env::home_dir().unwrap_or(PathBuf::from("."));
        home.join(Path::new(CONFIG_PATH))
    }

    fn show_dashboard(&self) {
        match self.app.get_applications() {
            Ok(apps) => {
                let mut is_first_iteration = true;
                let lines_count = apps.len() + 1;
                println!("Welcome to RusTOTPony realtime dashboard! Press ^C to quit.");
                loop {
                    if is_first_iteration {
                        is_first_iteration = false;
                    } else {
                        print!("\x1B[{}A", lines_count);
                    }
                    Self::print_progress_bar();
                    for (_, app) in apps {
                        println!{"{:06} {}", app.get_code(), app.get_name()};
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

    fn show_applications_list(&self, _: bool) {
        // TODO Create Table structure with HashMap as follows and metadata about columns - width, titles, names
        let mut output_table: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut applications_count = 0;
        let apps = match self.app.get_applications() {
            Ok(v) => v,
            Err(e) => {
                println!("{}", e);
                return;
            }
        };
        for (_, application) in apps {
            applications_count += 1;
            output_table
                .entry("name")
                .or_insert(Vec::new())
                .push(application.get_name());
            output_table
                .entry("key")
                .or_insert(Vec::new())
                .push(application.get_secret());
            output_table
                .entry("username")
                .or_insert(Vec::new())
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

    fn show_application(&self, name: &str) {
        println!("{:?}", self.app.get_application(name));
    }

    fn create_application(&mut self, name: &str, username: &str) {
        let secret = rpassword::prompt_password_stdout("Enter your secret code: ").unwrap();
        match self.app.create_application(name, username, &secret) {
            Ok(_) => {
                self.app.flush();
                println!("New application created: {}", name)
            }
            Err(err) => println!("{} Aborting…", err),
        }
    }

    fn delete_application(&mut self, name: &str) {
        match self.app.delete_application(name) {
            Ok(_) => {
                self.app.flush();
                println!("Application '{}' successfully deleted", name)
            }
            Err(err) => println!("Couldn't delete application '{}': {}", name, err),
        };
    }

    fn rename_application(&mut self, name: &str, newname: &str) {
        match self.app.rename_application(name, newname) {
            Ok(_) => {
                self.app.flush();
                println!(
                    "Application '{}' successfully renamed to '{}'",
                    name, newname
                )
            }
            Err(err) => println!("Couldn't rename application '{}': {}", name, err),
        };
    }

    fn eradicate_database(&mut self) {
        self.app.delete_all_applications();
        self.app.flush();
        println!("Done.");
    }
}
