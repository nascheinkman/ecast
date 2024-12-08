use tokio::io::AsyncBufReadExt;
use tokio::sync::{broadcast, oneshot};
use tokio::task::JoinHandle;

#[derive(Debug, Default)]
pub struct InputHandler {
    input_stream_handle: Option<JoinHandle<()>>,
    stop_signal: Option<oneshot::Sender<()>>,
    input_broadcast: Option<broadcast::Receiver<String>>,
}

impl InputHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn wait_for_close(mut self) {
        let signal = self.stop_signal.take();
        if signal.is_none() {
            return;
        }
        let mut signal = signal.unwrap();
        std::future::poll_fn(|cx| signal.poll_closed(cx)).await;
    }
    pub fn new_connect<T: AsyncBufReadExt + Unpin + Send + 'static>(mut input_stream: T) -> Self {
        let buf_size = 16;
        let (they_stop_send, my_stop_recv) = oneshot::channel();
        let (sender, input_broadcast) = broadcast::channel(buf_size);
        let handle = tokio::spawn(async move {
            let mut buffer = String::new();
            tokio::pin!(my_stop_recv);
            loop {
                let res = tokio::select! {
                    _ = (&mut my_stop_recv) => {
                        break
                    }
                    res = input_stream.read_line(&mut buffer) => {
                        res
                    }
                };
                if res.is_err() || res.unwrap() == 0 {
                    break;
                }
                let _ = sender.send(buffer.clone());
                buffer.clear();
            }
        });
        Self {
            input_stream_handle: Some(handle),
            stop_signal: Some(they_stop_send),
            input_broadcast: Some(input_broadcast),
        }
    }

    pub fn subscribe(&self) -> Option<broadcast::Receiver<String>> {
        let item = self
            .input_broadcast
            .as_ref()
            .map(|broadcast| broadcast.resubscribe());
        item
    }

    pub async fn stop(&mut self) {
        if let Some(stop_signal) = self.stop_signal.take() {
            let _ = stop_signal.send(());
        }
        if let Some(handle) = self.input_stream_handle.take() {
            let _ = handle.await;
        }
        self.input_broadcast.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn basic_construction() {
        use tokio::task::JoinSet;
        let cursor = std::io::Cursor::new(b"test\ntest\ntest");
        let input_handler = InputHandler::new_connect(cursor);
        let mut handles = JoinSet::new();

        for _ in 0..10 {
            let mut receiver = input_handler.subscribe().unwrap();
            handles.spawn(async move {
                let line = receiver.recv().await;
                assert_eq!(line.unwrap(), "test\n");
                let line = receiver.recv().await;
                assert_eq!(line.unwrap(), "test\n");
                let line = receiver.recv().await;
                assert_eq!(line.unwrap(), "test");
                let line = receiver.recv().await;
                assert!(line.is_err());
            });
        }
        handles.join_all().await;
    }
}
