use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    unimplemented,
};

use bytes::Bytes;
use mini_redis::{Command, Connection, Frame};
use once_cell::sync::Lazy;
use tokio::net::{TcpListener, TcpStream};

type Db = Mutex<HashMap<String, Bytes>>;

static DB: Lazy<Db> = Lazy::new(|| Mutex::new(HashMap::new()));

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        process(socket).await;
    }
}

async fn process(socket: TcpStream) {
    use mini_redis::Command::{self, Get, Set};

    let mut connection = Connection::new(socket);

    while let Some(frame) = connection.read_frame().await.unwrap() {
        println!("GOT: {:?}", frame);

        if let Ok(db) = DB.lock().as_mut() {
            let response = match Command::from_frame(frame).unwrap() {
                Set(cmd) => {
                    db.insert(cmd.key().to_string(), cmd.value().clone());
                    Frame::Simple("OK".to_string())
                }
                Get(cmd) => {
                    if let Some(value) = db.get(cmd.key()) {
                        Frame::Bulk(value.clone())
                    } else {
                        Frame::Null
                    }
                }
                cmd => unimplemented!(),
            };
            connection.write_frame(&response).await.unwrap();
        }
    }
}
