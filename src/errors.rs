use postgres;
use std::io;
use yaml_rust;

quick_error! {
    #[derive(Debug)]
    pub enum EBError {
        BuildError(err: String) { from() }
        ConnectionError(err: postgres::error::ConnectError) { from() }
        PostgresError(err: postgres::error::Error) { from() }
        IoError(err: io::Error) { from() }
        YamlError(err: yaml_rust::ScanError) { from() }
        Error(message: &'static str) { description(message) display("Error: {}", message) from() }
    }
}

pub type EBResult<T> = Result<T, EBError>;
