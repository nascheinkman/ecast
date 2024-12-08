use ecast::file_caster::cast_to_file;
use ecast::input_handler::InputHandler;
use ecast::stdout_caster::cast_to_stdout;
use ecast::tcp_caster::TcpCaster;
use ecast::grpc_caster::grpc_cast_server;
use tokio::fs::File;
use tokio::io::stdin;
use tokio::io::BufReader;
use std::net::SocketAddr;
use clap::Parser;

/// Broadcast stdin to any multiple of the supported protocols
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Log stdin to a file at the specified path
    #[arg(short, long)]
    files: Option<Vec<String>>,
    /// Echo stdin back to stdout
    #[arg(short, long, action)]
    echo: bool,
    /// Create a TCP server at the given address that allows any client to stream the data
    #[arg(short, long, action)]
    tcp_address: Option<SocketAddr>,
    /// Creates a GRPC server with reflection at the given address that allows any client to stream
    /// the data.
    #[arg(short, long, action)]
    grpc_address: Option<SocketAddr>
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let mut handles = vec![];
    let files = args.files.unwrap_or_default();
    let mut file_handles = vec![];
    for file in files {
        let log_file = File::create(file).await?;
        file_handles.push(log_file);
    }
    let input_handler = InputHandler::new_connect(BufReader::new(stdin()));
    for file in file_handles.drain(..) {
        let handle = cast_to_file(&input_handler, file);
        handles.push(handle);
    }
    if args.echo {
        let handle = cast_to_stdout(&input_handler);
        handles.push(handle);
    }
    if let Some(addr) = args.grpc_address {
        let handle = grpc_cast_server(input_handler.subscribe().unwrap(), addr);
        handles.push(handle);
    }
    let mut tcp_server = None;
    if let Some(addr) = args.tcp_address {
        tcp_server = Some(TcpCaster::new(input_handler.subscribe().unwrap(), addr).await?);
    }
    input_handler.wait_for_close().await;
    if let Some(tcp_server) = tcp_server {
        tcp_server.disconnect().await;
    }
    Ok(())
}
