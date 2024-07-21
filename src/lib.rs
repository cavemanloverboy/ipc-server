use mio::net::{UnixListener, UnixStream};
use mio::{Events, Interest, Poll, Token};
use serde::{Deserialize, Serialize};
use std::fs::{remove_file, set_permissions, Permissions};
use std::io::{self, Read};
use std::marker::PhantomData;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Duration;

pub trait IpcServerCommand: Serialize + for<'a> Deserialize<'a> + std::fmt::Debug {
    type Response: Serialize + for<'a> Deserialize<'a> + std::fmt::Debug;
    type Context<'a>;

    fn process<'a, 'b>(self, context: &'b mut Self::Context<'a>) -> Self::Response;
}

pub struct IpcServer<C: IpcServerCommand> {
    listener: UnixListener,
    poll: Poll,
    events: Events,
    _command: PhantomData<C>,
}

impl<C: IpcServerCommand> IpcServer<C> {
    /// Initialize a new IpcServer. Recall that there is no dedicated server
    /// thread. You must call `handle_new_messages` to poll for and process
    /// new messages
    pub fn new(socket_path: &str) -> io::Result<IpcServer<C>> {
        if Path::new(socket_path).exists() {
            remove_file(socket_path)?;
        }

        let mut listener = UnixListener::bind(socket_path)?;
        // Restrict permissions to owner read/write only
        set_permissions(socket_path, Permissions::from_mode(0o600))?;

        let poll = Poll::new()?;
        let events = Events::with_capacity(128);

        poll.registry()
            .register(&mut listener, Token(0), Interest::READABLE)?;

        Ok(IpcServer::<C> {
            listener,
            poll,
            events,
            _command: Default::default(),
        })
    }

    /// Polls for new messages from any clients, and processes and responds.
    pub fn handle_new_messages<'a>(&mut self, mut context: C::Context<'a>) -> io::Result<()> {
        self.poll.poll(&mut self.events, None)?;

        for event in self.events.iter() {
            match event.token() {
                Token(0) => loop {
                    match self.listener.accept() {
                        Ok((mut stream, _)) => {
                            let mut buffer = [0; 1024];
                            match stream.read(&mut buffer) {
                                Ok(bytes_read) => {
                                    let command = bincode::deserialize::<C>(&buffer[..bytes_read])
                                        .map_err(|e| {
                                            io::Error::new(io::ErrorKind::InvalidData, e)
                                        })?;
                                    self.process_command(command, &mut context, &mut stream)?;
                                }
                                Err(err) => {
                                    eprintln!("Failed to read from connection: {}", err);
                                    break;
                                }
                            }
                        }
                        Err(ref err) if would_block(err) => break,
                        Err(err) => {
                            eprintln!("Failed to accept connection: {}", err);
                            break;
                        }
                    }
                },
                _ => unreachable!(),
            }
        }

        Ok(())
    }

    #[inline(always)]
    fn process_command<'a, 'b>(
        &self,
        command: C,
        context: &'b mut C::Context<'a>,
        stream: &mut UnixStream,
    ) -> io::Result<()> {
        let response = command.process(context);
        loop {
            match bincode::serialize_into(&mut *stream, &response)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
            {
                Ok(()) => return Ok(()),
                Err(ref err) if would_block(err) => {
                    // Spin loop is okay here.
                    // IPC server is not intended for large payloads or high volumes.
                    std::hint::spin_loop();
                    continue;
                }
                e => return e,
            }
        }
    }
}

fn would_block(err: &std::io::Error) -> bool {
    err.kind() == std::io::ErrorKind::WouldBlock
}

/// Serialize and write the `command` provided to the `UnixStream` at the
/// `socket_path` provided. If there is an active `IpcServer`, it will receive
/// and process this command upon polling.
pub fn client_send<C: IpcServerCommand>(command: &C, socket_path: &str) {
    let mut stream = UnixStream::connect(socket_path).unwrap();
    bincode::serialize_into(&mut stream, command).unwrap();
    println!("sent command: {:?}", command);

    loop {
        let mut buffer = [0; 1024];
        match stream.read(&mut buffer) {
            Ok(bytes_read) => {
                if let Ok(response) = bincode::deserialize::<C::Response>(&buffer[..bytes_read]) {
                    println!("received response: {:?}", response);
                } else {
                    eprintln!("failed to parse response: {:?}", &buffer[..bytes_read]);
                }
                return;
            }
            Err(ref err) if would_block(&err) => {
                #[allow(deprecated)]
                std::thread::sleep_ms(1);
                continue;
            }
            Err(err) => {
                eprintln!("failed to read response: {} {}", err, err.kind());
                return;
            }
        }
    }
}
