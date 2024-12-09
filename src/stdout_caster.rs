use crate::input_handler::InputHandler;
use tokio::task::JoinHandle;

pub fn cast_to_stdout(handler: &InputHandler) -> JoinHandle<()> {
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
            print!("{}", line);
        }
    })
}
