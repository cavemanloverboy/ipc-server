use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use ipc_server::{client_send, IpcServer, IpcServerCommand};

pub const IPC_FD: &'static str = "ipc-server.sock";

#[derive(Parser)]
#[clap(name = "IPC Example")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Server,
    Client {
        #[clap(subcommand)]
        command: ClientCommand,
    },
}

#[derive(Subcommand, Serialize, Deserialize, Debug)]
enum ClientCommand {
    Print { payload: String },
    Add { a: u64, b: u64 },
    Push { x: u64 },
}

impl IpcServerCommand for ClientCommand {
    type Response = ClientResponse;
    type Context<'a> = &'a mut Vec<u64>;
    fn process<'a, 'b>(self, context: &'b mut Self::Context<'a>) -> ClientResponse {
        match self {
            ClientCommand::Print { payload } => {
                println!("Print command received: {}", payload);
                ClientResponse::PrintAck
            }
            ClientCommand::Add { a, b } => {
                let c = a + b;
                println!("Add command received: {} + {} = {}", a, b, c);
                ClientResponse::Add { c }
            }
            ClientCommand::Push { x } => {
                println!("Push command received: {}", x);
                context.push(x);
                ClientResponse::Push { x }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum ClientResponse {
    /// A simple ack
    PrintAck,
    /// The sum result
    Add { c: u64 },
    /// The value pushed
    Push { x: u64 },
}

pub fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Server => {
            let mut server = IpcServer::<ClientCommand>::new(IPC_FD).unwrap();
            let mut values = vec![];
            loop {
                server.handle_new_messages(&mut values).ok();
                // Do other work
                std::thread::sleep(Duration::from_secs(1));
            }
        }
        Commands::Client { command } => client_send(command, &IPC_FD),
    }
}
