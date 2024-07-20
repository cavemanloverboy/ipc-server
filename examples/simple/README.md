# `simple` example

A cli application with a server and client mode.

## Usage

First spin up a server via

```bash
cargo run --example simple -- server
```

Then, send message to the server via one of the following commands.

### Print

Print your input string via

```bash
cargo run --example simple -- client print "cavemanloverboy was here"
```

### Add

Add two numbers and get the result via

```bash
cargo run --example simple -- client add 5 3
```

### Push

Push a `u64` to the stack that the server maintains via

```bash
cargo run --example simple -- client push 69
```
