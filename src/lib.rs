extern crate age;
extern crate base32;
extern crate crypto;
extern crate dirs;
extern crate rand;
extern crate serde_json;
extern crate sha2;
extern crate totp_lite;

#[macro_use]
extern crate serde_derive;

use crypto::buffer::{BufferResult, ReadBuffer, WriteBuffer};
use crypto::{aes, blockmodes, buffer, symmetriccipher};
use sha2::{Digest, Sha256};

use totp_lite::{totp_custom, Sha1, DEFAULT_STEP};

use rand::prelude::*;

use std::collections::HashMap;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::ErrorKind;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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

macro_rules! impl_database_trait {
    ($type:ty) => {
        impl Database for $type {
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
    };
}

impl_database_trait!(JsonDatabase);
impl_database_trait!(AgeJsonDatabase);

#[derive(Serialize, Deserialize)]
pub struct JsonDatabaseSchema {
    version: u8,
    content: DatabaseContentSchema,
}

#[derive(Serialize, Deserialize)]
struct DatabaseContentSchema {
    applications: HashMap<String, GenApp>,
}

pub struct JsonDatabase {
    file_path: PathBuf,
    secret: String,
}

pub struct AgeJsonDatabase {
    file_path: PathBuf,
    secret: String,
}

const IV_SIZE: usize = 16;
const KEY_SIZE: usize = 32;
pub trait JsonDatabaseTrait {
    fn get_file_path(&self) -> &PathBuf;
    fn get_secret(&self) -> String;

    fn new(path: PathBuf, secret: String) -> Self;

    fn encrypt_data(data: &str, key: &str) -> Vec<u8>;

    fn decrypt_data(data: &[u8], key: &str) -> String;

    fn read_database_file(&self) -> JsonDatabaseSchema {
        let data = match std::fs::read(self.get_file_path()) {
            Ok(d) => d,
            Err(ref err) if err.kind() == ErrorKind::NotFound => return Self::get_empty_schema(),
            Err(err) => panic!("There was a problem opening file: {:?}", err),
        };
        let decrypted_data = Self::decrypt_data(&data, self.get_secret().as_str());
        serde_json::from_str(decrypted_data.as_str())
            .expect("Couldn't parse JSON from database file")
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
        let encrypted_data = Self::encrypt_data(&data, self.get_secret().as_str());
        file.write_all(&encrypted_data)
            .expect("Couldn't write data to database file");
    }

    fn create_database_file(&self) -> Result<File, std::io::Error> {
        let dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        if let Some(parent_dir) = Path::new(&self.get_file_path()).parent() {
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
            .open(self.get_file_path())
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

macro_rules! impl_json_database_trait {
    ($type:ty) => {
        impl JsonDatabaseTrait for $type {
            fn new(path: PathBuf, secret: String) -> Self {
                Self {
                    file_path: path,
                    secret,
                }
            }

            fn get_file_path(&self) -> &PathBuf {
                &self.file_path
            }

            fn get_secret(&self) -> String {
                self.secret.clone()
            }

            fn encrypt_data(data: &str, key: &str) -> Vec<u8> {
                <$type>::encrypt_data(data, key)
            }

            fn decrypt_data(data: &[u8], key: &str) -> String {
                <$type>::decrypt_data(data, key)
            }
        }
    };
}

impl_json_database_trait!(JsonDatabase);
impl_json_database_trait!(AgeJsonDatabase);

impl JsonDatabase {
    fn form_secret_key(input: &str) -> [u8; KEY_SIZE] {
        let mut hasher = Sha256::new();
        hasher.update(input);
        hasher.finalize().into()
    }

    fn encrypt_data(data: &str, key: &str) -> Vec<u8> {
        let key = Self::form_secret_key(key);
        let iv = Self::create_iv();
        let encrypted_data =
            Self::encrypt(data.as_bytes(), &key, &iv).expect("Couldn't encrypt data");
        [&iv, &encrypted_data[..]].concat()
    }

    fn decrypt_data(data: &[u8], key: &str) -> String {
        let key = Self::form_secret_key(key);
        let iv = &data[..IV_SIZE];
        String::from_utf8(Self::decrypt(&data[IV_SIZE..], &key, iv).expect("Couldn't decrypt data"))
            .ok()
            .unwrap()
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
}

impl AgeJsonDatabase {
    fn encrypt_data(data: &str, key: &str) -> Vec<u8> {
        let encryptor =
            age::Encryptor::with_user_passphrase(age::secrecy::Secret::new(key.to_owned()));

        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted).unwrap();
        writer.write_all(data.as_bytes()).unwrap();
        writer.finish().unwrap();

        encrypted
    }

    fn decrypt_data(data: &[u8], key: &str) -> String {
        let decryptor = match age::Decryptor::new(data).unwrap() {
            age::Decryptor::Passphrase(d) => d,
            _ => unreachable!(),
        };

        let mut decrypted = vec![];
        let mut reader = decryptor
            .decrypt(&age::secrecy::Secret::new(key.to_owned()), None)
            .unwrap();
        reader.read_to_end(&mut decrypted).unwrap();

        String::from_utf8(decrypted).unwrap()
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

    pub fn get_code(&self) -> String {
        Self::totp(&self.secret_bytes)
    }

    fn base32_to_bytes(secret: &str) -> Option<Vec<u8>> {
        base32::decode(base32::Alphabet::Rfc4648 { padding: false }, secret)
    }

    fn totp(secret_bytes: &[u8]) -> String {
        let seconds: u64 = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        totp_custom::<Sha1>(DEFAULT_STEP, 6, secret_bytes, seconds)
    }
}
