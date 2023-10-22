#![allow(dead_code)]
#![allow(unused_variables)]
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::cmd::Cmd;
use crate::frame::Response;
use crate::types::{self, RedisError, RedisResult};

#[derive(Debug, Clone)]
pub enum Value {
    Nil,
    Int(i64),
    Data(Vec<u8>),
    Bulk(Vec<Value>),
    Status(String),
    Okay,
}

impl Value {
    fn from_frame(f: &mut Response) -> Option<Self> {
        let prefix = f.read(1)?;
        let c = prefix.chars().next().unwrap();
        match c {
            '+' => {
                let content = f.read_next()?;
                if &content == "OK" {
                    Some(Value::Okay)
                } else {
                    Some(Self::Data(content.as_bytes().to_vec()))
                }
            }
            '-' => Some(Self::Status(f.read_next()?)),
            ':' => Some(Self::Int(f.read_next()?.parse().ok()?)),
            '$' => {
                let count = f.read_next()?.parse::<usize>().ok()?;
                Some(Self::Data(f.read(count)?.as_bytes().to_vec()))
            }
            '*' => todo!("Some(Self::Boolean)"),
            '_' => todo!("Some(Self::Null)"),
            '#' => todo!("Some(Self::Boolean)"),
            ',' => todo!("Some(Self::Double)"),
            '(' => todo!("Some(Self::BigNumber)"),
            '!' => todo!("Some(Self::BulkError)"),
            '=' => todo!("Some(Self::VerbatimString)"),
            '%' => todo!("Some(Self::Map)"),
            '~' => todo!("Some(Self::Set)"),
            '>' => todo!("Some(Self::Push)"),
            _ => None,
        }
    }
}

// impl From<Value> for i64 {
//     fn from(v: Value) -> Self {
//         dbg!(v.clone());
//         match v {
//             Value::Int(d) => d,
//             _ => -619,
//         }
//     }
// }
impl TryFrom<Value> for i64 {
    type Error = RedisError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Int(d) => Ok(d),
            Value::Status(s) => Err(RedisError::ErrorResponse(s)),
            _ => Err(RedisError::ErrIllegalTypeConversion),
        }
    }
}
impl TryFrom<Value> for String {
    type Error = RedisError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Data(v) => Ok(String::from_utf8(v).unwrap()),
            _ => panic!("xx"),
        }
    }
}

pub struct Client {
    conn: Connection,
}

struct Connection {
    conn: TcpStream,
    recv: Response,
}
impl Connection {
    fn new() -> RedisResult<Self> {
        let st = TcpStream::connect("127.0.0.1:6379")?;
        st.set_read_timeout(Some(Duration::from_secs(2)))?;
        let mut buf = Vec::new();
        buf.resize(1024, 0);
        Ok(Self {
            conn: st,
            recv: Response::new(),
        })
    }
    fn get_reply<T>(&mut self) -> types::RedisResult<T>
    where
        T: TryFrom<Value, Error = RedisError>,
    {
        self.recv.reset();
        match self.conn.read(&mut self.recv.b)? {
            0 => Err(RedisError::NoBytesWriten),
            // _ => Ok(Value::from_frame(&mut self.recv).unwrap().try_into()?),
            _ => {
                let value = Value::from_frame(&mut self.recv).unwrap();
                let out: RedisResult<T> = value.try_into();
                out
                // Ok(())
            }
        }
    }
    fn exec<T>(&mut self, cmd: Cmd) -> RedisResult<T>
    where
        T: TryFrom<Value, Error = RedisError>,
    {
        let x = &cmd.bytes()[..];
        print!("will send {}", String::from_utf8(x.to_vec()).unwrap());
        match self.conn.write(x)? {
            0 => Err(RedisError::NoBytesWriten),
            _ => Ok(()),
        }?;
        self.get_reply()
    }
    fn send(&mut self, s: String) -> RedisResult<()> {
        self.recv.reset();
        match self.conn.write(s.as_bytes())? {
            0 => Err(RedisError::NoBytesWriten),
            _ => Ok(()),
        }
    }
    fn resp(&mut self, s: String) -> RedisResult<Value> {
        self.send(s)?;
        self.get_reply()
    }
}

impl Client {
    fn new() -> types::RedisResult<Self> {
        Ok(Client {
            conn: Connection::new()?,
        })
    }
    fn set(&mut self, key: &str, value: &str) -> types::RedisResult<()> {
        match self.conn.resp(format!("SET {} {}\r\n", key, value))? {
            Value::Okay => Ok(()),
            _ => Err(RedisError::UnexpectedResponseType),
        }
    }
    fn del(&mut self, key: &str) -> RedisResult<usize> {
        match self.conn.resp(format!("DEL {}\r\n", key))? {
            Value::Int(d) => Ok(d as usize),
            _ => Err(RedisError::UnexpectedResponseType),
        }
    }
    fn get(&mut self, key: &str) -> types::RedisResult<String> {
        match self.conn.resp(format!("GET {}\r\n", key))? {
            Value::Data(v) => Ok(Value::Data(v).try_into()?),
            _ => Err(RedisError::UnexpectedResponseType),
        }
    }
    fn decr(&mut self, key: &str) -> types::RedisResult<i64> {
        match self.conn.resp(format!("DECR {}\r\n", key))? {
            Value::Int(d) => Ok(d),
            _ => Err(RedisError::UnexpectedResponseType),
        }
    }
    fn incr(&mut self, key: &str) -> types::RedisResult<i64> {
        match self.conn.resp(format!("INCR {}\r\n", key))? {
            Value::Int(d) => Ok(d),
            _ => Err(RedisError::UnexpectedResponseType),
        }
    }
    fn incrby(&mut self, key: &str, n: usize) -> RedisResult<i64> {
        self.conn.exec(Cmd::new().arg("INCRBY").arg(key).arg(n))
    }
    fn decrby(&mut self, key: &str, n: usize) -> RedisResult<i64> {
        self.conn.exec(Cmd::new().arg("DECRBY").arg(key).arg(n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_string() -> types::RedisResult<()> {
        let mut client = Client::new()?;
        assert_eq!((), client.set("foo", "bar")?);
        assert_eq!("bar", client.get("foo")?);
        Ok(())
    }

    #[test]
    fn get_number() -> RedisResult<()> {
        let mut client = Client::new()?;
        client.del("abc").unwrap(); // cleanup
        assert_eq!(-1, client.decr("abc")?);
        assert_eq!(-2, client.decr("abc")?);
        assert_eq!(-1, client.incr("abc")?);
        assert_eq!(0, client.incr("abc")?);
        assert_eq!(1, client.incr("abc")?);
        Ok(())
    }

    #[test]
    fn incrby() -> RedisResult<()> {
        let mut client = Client::new()?;
        client.del("abc").unwrap();
        assert_eq!(10, client.incrby("abc", 10)?);
        assert_eq!(20, client.incrby("abc", 10)?);
        assert_eq!(-10, client.decrby("abc", 30)?);
        Ok(())
    }
}
