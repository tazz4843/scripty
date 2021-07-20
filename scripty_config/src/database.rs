/// A enum that specifies the type of database connection to use.
pub enum DatabaseConnection {
    /// Use a TCP socket to connect to the database.
    /// The first item is the host, and the second is the port.
    TcpSocket(String, u16),
    /// Use a Unix socket to connect to the database.
    /// The only item is the socket path.
    UnixSocket(String),
}
