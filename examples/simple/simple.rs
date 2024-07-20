use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use ipc_server::{client_send, IpcServer, IpcServerCommand};

/// The socket used for this example
pub const IPC_FD: &'static str = "ipc-server.sock";

#[derive(Parser)]
#[clap(name = "IPC Example")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the application in server mode.
    ///
    /// Listens for commands indefinitely at the socket address provided.
    Server,

    /// Initialize the application in client mode and sends the server a
    /// message.
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
        // Server mode
        Commands::Server => {
            // Initialize server
            let mut server = IpcServer::<ClientCommand>::new(IPC_FD).unwrap();

            // Application state
            let mut values = vec![];

            loop {
                // Context (external resources) required for the server to
                // process the command. In this case, it's just a &mut Vec<u64>,
                // but it can be a handle to a database, or some other larger
                // type or data structure
                let context = &mut values;

                // Handle messages with the context
                server.handle_new_messages(context).ok();

                // Here is where your application could do other work.
                std::thread::sleep(Duration::from_secs(1));
            }
        }

        // Client mode; send in the command
        Commands::Client { command } => client_send(command, &IPC_FD),
    }
}
