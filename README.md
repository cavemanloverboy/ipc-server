# `ipc-server`: lazy and asynchronous IPC server

This library allows independent client and server processes on the same machine to pass messages via Unix sockets that are readable and writable exclusively by the current user. This form of inter-process communication (IPC) is suitable for run time configurable variables by an admin, and is in fact inspired by solana's admin rpc service.

Unlike other servers, this server is lazy and asynchronous. There is no dedicated server thead that is in a busy loop or awaiting messages. Instead, the server type exposes a poll method that you can integrate within some loop in your application.

## Overview

The three relevant components of this system are the client, the server, and the messages passed between them. We will go over these components in reverse order.

See `examples/simple/simple.rs` for a fully self-contained and self-explanatory example. We will refer to this example throughout the rest of the README.

### Messages

This server uses `bincode` for the serialization and deserialization of messages. First implement the `IpcServerCommand` for your message

```rust
pub trait IpcServerCommand: Serialize + for<'a> Deserialize<'a> + std::fmt::Debug {
    type Response: Serialize + for<'a> Deserialize<'a> + std::fmt::Debug;
    type Context<'a>;

    fn process<'a, 'b>(self, context: &'b mut Self::Context<'a>) -> Self::Response;
}
```

This trait tells the server how to process the command. We recommend using an enum to allow for multiple types of message. The `Context<'a>` type allows the server to access state that is external to the server (in-memory or persistent state, such as a vector, hashmap, or persistent/external database). In the `simple` example, for example, there are three types of messages:

```rust
#[derive(Subcommand, Serialize, Deserialize, Debug)]
enum ClientCommand {
    Print { payload: String },
    Add { a: u64, b: u64 },
    Push { x: u64 },
}
```

These variants are self-explanatory.`Print` prints a payload. `Add` adds two numbers, prints them, and returns the result. `Push` pushes a value onto a stack. The stack (which is the `Context<'a>` in this case), along with how each variant is processed and the message response type is specified in the trait implementation:

```rust
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

```

This fully specifies the messages the client can send, how the server processes them, and what kinds of responses the client can expect.

### Server

The `IpcServer` type is generic over the command. Upon specifying the message types as described in the previous section, all you need to do to get a server is

```rust
let socket_path = "ipc-server-path-example.sock"
let server = IpcServer::new(socket_path).unwrap();
```

This server is now initialized. But remember, there is no dedicated server thread. No messages will be received or processed until the `server.handle_new_messages(..)` method is called. Your application must define how to prepare the `Context<'a>`. This will likely be either nothing or some reference to a data structure or database. This poll method will read, process, and respond to all outstanding messages.

### Client

There is no client type. There is simply a `fn client_send<C: IpcServerCommand>(command: &C, socket_path: &str)` that serializes and sends the given command via a `UnixStream` aimed at the socket address provided.

## Notes

The server and client use fixed sized 1024 byte buffers for the messages. You might run into issues if your message size exceeds this.
