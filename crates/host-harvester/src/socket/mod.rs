mod client;
mod operation;
mod packet;
mod server;

pub use client::SocketReader;
pub use server::SocketServer;

pub const DEFAULT_SOCKET: &str = "/run/host-harvester/socket";
