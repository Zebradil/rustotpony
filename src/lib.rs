extern crate base32;
extern crate oath;
extern crate rand;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use std::collections::HashMap;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

const DATABASE_VERSION: u8 = 1;

pub struct RusTOTPony<DB: Database> {
    database: DB,
    applications: HashMap<String, GenApp>,
}

impl<DB: Database> RusTOTPony<DB> {
    pub fn new(db: DB) -> RusTOTPony<DB> {
        RusTOTPony {
            applications: db.get_applications(),
            database: db,
        }
    }

    pub fn create_application(
        &mut self,
        name: &str,
        username: &str,
        secret: &str,
    ) -> Result<(), String> {
        if let Some(secret_bytes) = GenApp::base32_to_bytes(secret) {
            let new_app = GenApp::new(name, username, secret, secret_bytes);
            if self.applications.contains_key(name) {
                Err(format!("Application with name '{}' already exists!", name))
            } else {
                &self.applications.insert(String::from(name), new_app);
                Ok(())
            }
        } else {
            return Err(String::from("Couldn't decode secret key"));
        }
    }

    pub fn delete_application(&mut self, name: &str) -> Result<(), String> {
        if let Some(_) = self.applications.remove(name) {
            Ok(())
        } else {
            Err(format!(
                "Application with the name '{}' doesn't exist",
                name
            ))
        }
    }

    pub fn rename_application(&mut self, name: &str, newname: &str) -> Result<(), String> {
        if let Some(app) = self.applications.get_mut(name) {
            app.name = String::from(newname);
            Ok(())
        } else {
            Err(format!("Application '{}' wasn't found", name))
        }
    }

    pub fn get_applications(&self) -> Result<&HashMap<String, GenApp>, String> {
        if self.applications.len() == 0 {
            Err(String::from("There are no applications"))
        } else {
            Ok(&self.applications)
        }
    }

    pub fn get_application(&self, name: &str) -> Result<&GenApp, String> {
        if let Some(app) = self.applications.get(name) {
            Ok(app)
        } else {
            Err(format!("Application '{}' wasn't found", name))
        }
    }

    pub fn delete_all_applications(&mut self) {
        self.applications = HashMap::new();
    }

    pub fn flush(&self) {
        &self.database.save_applications(&self.applications);
    }
}

pub trait Database {
    fn get_applications(&self) -> HashMap<String, GenApp>;
    fn save_applications(&self, applications: &HashMap<String, GenApp>);
}

impl Database for JsonDatabase {
    fn get_applications(&self) -> HashMap<String, GenApp> {
        let db_content = self.read_database_file();
        db_content.content.applications
    }

    fn save_applications(&self, applications: &HashMap<String, GenApp>) {
        let mut db_content = Self::get_empty_schema();
        db_content.content.applications = applications.clone();
        self.save_database_file(db_content);
    }
}

#[derive(Serialize, Deserialize)]
struct JsonDatabaseSchema {
    version: u8,
    content: DatabaseContentSchema,
}

#[derive(Serialize, Deserialize)]
struct DatabaseContentSchema {
    applications: HashMap<String, GenApp>,
}

pub struct JsonDatabase {
    file_path: PathBuf,
}

impl JsonDatabase {
    pub fn new(path: PathBuf) -> JsonDatabase {
        JsonDatabase { file_path: path }
    }

    fn read_database_file(&self) -> JsonDatabaseSchema {
        let file = match File::open(&self.file_path) {
            Ok(f) => f,
            Err(ref err) if err.kind() == ErrorKind::NotFound => return Self::get_empty_schema(),
            Err(err) => panic!("There was a problem opening file: {:?}", err),
        };
        serde_json::from_reader(file).expect("Couldn't parse JSON from database file")
    }

    fn save_database_file(&self, content: JsonDatabaseSchema) {
        let file = match self.open_database_file_for_write() {
            Ok(f) => f,
            Err(ref err) if err.kind() == ErrorKind::NotFound => self.create_database_file()
                .expect("Couldn't create database file"),
            Err(err) => panic!("Couldn't open database file: {:?}", err),
        };
        serde_json::to_writer(file, &content).expect("Couldn't write JSON data to database file");
    }

    fn create_database_file(&self) -> Result<File, std::io::Error> {
        let dir = std::env::home_dir().unwrap_or(PathBuf::from("."));
        if let Some(parent_dir) = Path::new(&self.file_path).parent() {
            let dir = dir.join(parent_dir);
            create_dir_all(dir)?;
        }
        self.open_database_file_for_write()
    }

    fn open_database_file_for_write(&self) -> Result<File, std::io::Error> {
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&self.file_path)
    }

    fn get_empty_schema() -> JsonDatabaseSchema {
        JsonDatabaseSchema {
            version: DATABASE_VERSION,
            content: DatabaseContentSchema {
                applications: HashMap::new(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GenApp {
    name: String,
    secret: String,
    username: String,
    secret_bytes: Vec<u8>,
}

impl GenApp {
    fn new(name: &str, username: &str, secret: &str, secret_bytes: Vec<u8>) -> Self {
        GenApp {
            name: String::from(name),
            secret: String::from(secret),
            username: String::from(username),
            secret_bytes: secret_bytes,
        }
    }

    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }

    pub fn get_secret(&self) -> &str {
        self.secret.as_str()
    }

    pub fn get_username(&self) -> &str {
        self.username.as_str()
    }

    pub fn get_code(&self) -> u64 {
        Self::totp(&self.secret_bytes)
    }

    fn base32_to_bytes(secret: &str) -> Option<Vec<u8>> {
        base32::decode(base32::Alphabet::RFC4648 { padding: false }, secret)
    }

    fn totp(secret_bytes: &[u8]) -> u64 {
        oath::totp_raw_now(&secret_bytes, 6, 0, 30, &oath::HashType::SHA1)
    }
}
