extern crate base32;
extern crate crypto;
extern crate dirs;
extern crate oath;
extern crate rand;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use crypto::buffer::{BufferResult, ReadBuffer, WriteBuffer};
use crypto::digest::Digest;
use crypto::sha2::Sha256;
use crypto::{aes, blockmodes, buffer, symmetriccipher};

use rand::prelude::*;

use std::collections::HashMap;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::ErrorKind;
use std::io::Write;
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
                self.applications.insert(String::from(name), new_app);
                Ok(())
            }
        } else {
            Err(String::from("Couldn't decode secret key"))
        }
    }

    pub fn delete_application(&mut self, name: &str) -> Result<(), String> {
        if self.applications.remove(name).is_some() {
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
        if self.applications.is_empty() {
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
        self.database.save_applications(&self.applications);
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
    secret_fn: &'static dyn Fn() -> String,
}

const IV_SIZE: usize = 16;
const KEY_SIZE: usize = 32;
impl JsonDatabase {
    pub fn new(path: PathBuf, secret_fn: &'static dyn Fn() -> String) -> JsonDatabase {
        JsonDatabase {
            file_path: path,
            secret_fn,
        }
    }

    fn form_secret_key(input: &str) -> [u8; KEY_SIZE] {
        let mut sha = Sha256::new();
        sha.input_str(input);
        let mut res: [u8; KEY_SIZE] = [0; KEY_SIZE];
        sha.result(&mut res);
        res
    }

    fn read_database_file(&self) -> JsonDatabaseSchema {
        let data = match std::fs::read(&self.file_path) {
            Ok(d) => d,
            Err(ref err) if err.kind() == ErrorKind::NotFound => return Self::get_empty_schema(),
            Err(err) => panic!("There was a problem opening file: {:?}", err),
        };
        let decrypted_data =
            Self::decrypt_data(&data, &Self::form_secret_key((self.secret_fn)().as_str()));
        serde_json::from_str(decrypted_data.as_str())
            .expect("Couldn't parse JSON from database file")
    }

    fn decrypt_data(data: &[u8], key: &[u8]) -> String {
        let iv = &data[..IV_SIZE];
        String::from_utf8(Self::decrypt(&data[IV_SIZE..], key, iv).expect("Couldn't decrypt data"))
            .ok()
            .unwrap()
    }

    fn encrypt_data(data: &str, key: &[u8]) -> Vec<u8> {
        let iv = Self::create_iv();
        let encrypted_data =
            Self::encrypt(data.as_bytes(), key, &iv).expect("Couldn't encrypt data");
        [&iv, &encrypted_data[..]].concat()
    }

    fn create_iv() -> Vec<u8> {
        let mut iv = vec![0; IV_SIZE];
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut iv);
        iv
    }

    fn save_database_file(&self, content: JsonDatabaseSchema) {
        let mut file = match self.open_database_file_for_write() {
            Ok(f) => f,
            Err(ref err) if err.kind() == ErrorKind::NotFound => self
                .create_database_file()
                .expect("Couldn't create database file"),
            Err(err) => panic!("Couldn't open database file: {:?}", err),
        };
        let data = serde_json::to_string(&content).expect("Couldn't serialize data to JSON");
        let encrypted_data =
            Self::encrypt_data(&data, &Self::form_secret_key((self.secret_fn)().as_str()));
        file.write_all(&encrypted_data)
            .expect("Couldn't write data to database file");
    }

    // Encrypt a buffer with the given key and iv using
    // AES-256/CBC/Pkcs encryption.
    fn encrypt(
        data: &[u8],
        key: &[u8],
        iv: &[u8],
    ) -> Result<Vec<u8>, symmetriccipher::SymmetricCipherError> {
        // Create an encryptor instance of the best performing
        // type available for the platform.
        let mut encryptor =
            aes::cbc_encryptor(aes::KeySize::KeySize256, key, iv, blockmodes::PkcsPadding);

        // Each encryption operation encrypts some data from
        // an input buffer into an output buffer. Those buffers
        // must be instances of RefReaderBuffer and RefWriteBuffer
        // (respectively) which keep track of how much data has been
        // read from or written to them.
        let mut final_result = Vec::<u8>::new();
        let mut read_buffer = buffer::RefReadBuffer::new(data);
        let mut buffer = [0; 4096];
        let mut write_buffer = buffer::RefWriteBuffer::new(&mut buffer);

        // Each encryption operation will "make progress". "Making progress"
        // is a bit loosely defined, but basically, at the end of each operation
        // either BufferUnderflow or BufferOverflow will be returned (unless
        // there was an error). If the return value is BufferUnderflow, it means
        // that the operation ended while wanting more input data. If the return
        // value is BufferOverflow, it means that the operation ended because it
        // needed more space to output data. As long as the next call to the encryption
        // operation provides the space that was requested (either more input data
        // or more output space), the operation is guaranteed to get closer to
        // completing the full operation - ie: "make progress".
        //
        // Here, we pass the data to encrypt to the enryptor along with a fixed-size
        // output buffer. The 'true' flag indicates that the end of the data that
        // is to be encrypted is included in the input buffer (which is true, since
        // the input data includes all the data to encrypt). After each call, we copy
        // any output data to our result Vec. If we get a BufferOverflow, we keep
        // going in the loop since it means that there is more work to do. We can
        // complete as soon as we get a BufferUnderflow since the encryptor is telling
        // us that it stopped processing data due to not having any more data in the
        // input buffer.
        loop {
            let result = encryptor.encrypt(&mut read_buffer, &mut write_buffer, true)?;

            // "write_buffer.take_read_buffer().take_remaining()" means:
            // from the writable buffer, create a new readable buffer which
            // contains all data that has been written, and then access all
            // of that data as a slice.
            final_result.extend(
                write_buffer
                    .take_read_buffer()
                    .take_remaining()
                    .iter()
                    .copied(),
            );

            match result {
                BufferResult::BufferUnderflow => break,
                BufferResult::BufferOverflow => {}
            }
        }

        Ok(final_result)
    }

    // Decrypts a buffer with the given key and iv using
    // AES-256/CBC/Pkcs encryption.
    fn decrypt(
        encrypted_data: &[u8],
        key: &[u8],
        iv: &[u8],
    ) -> Result<Vec<u8>, symmetriccipher::SymmetricCipherError> {
        let mut decryptor =
            aes::cbc_decryptor(aes::KeySize::KeySize256, key, iv, blockmodes::PkcsPadding);

        let mut final_result = Vec::<u8>::new();
        let mut read_buffer = buffer::RefReadBuffer::new(encrypted_data);
        let mut buffer = [0; 4096];
        let mut write_buffer = buffer::RefWriteBuffer::new(&mut buffer);

        loop {
            let result = decryptor.decrypt(&mut read_buffer, &mut write_buffer, true)?;
            final_result.extend(
                write_buffer
                    .take_read_buffer()
                    .take_remaining()
                    .iter()
                    .copied(),
            );
            match result {
                BufferResult::BufferUnderflow => break,
                BufferResult::BufferOverflow => {}
            }
        }

        Ok(final_result)
    }

    fn create_database_file(&self) -> Result<File, std::io::Error> {
        let dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
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
            secret_bytes,
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
