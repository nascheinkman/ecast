use crate::input_handler::InputHandler;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt};
use tokio::task::JoinHandle;

pub fn cast_to_file(handler: &InputHandler, mut file: File) -> JoinHandle<()> {
    let receiver = handler.subscribe();
    tokio::spawn(async move {
        if receiver.is_none() {
            return;
        }
        let mut receiver = receiver.unwrap();
        loop {
            let line = receiver.recv().await;
            if line.is_err() {
                break;
            }
            let line = line.unwrap();
            let res = file.write_all(line.as_bytes()).await;
            if res.is_err() {
                break;
            }
        }
    })
}
