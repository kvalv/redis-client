#![allow(dead_code)]
#![allow(unused_variables)]
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::cmd::{Arg, Cmd};
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
                let res = Some(Self::Data(f.read(count)?.as_bytes().to_vec()));
                f.read(2); // read the next \r\n
                res
            }
            '*' => {
                let count = f.read_next()?.parse::<usize>().ok()?;
                let mut res: Vec<Self> = vec![];
                for i in 0..count {
                    res.push(Self::from_frame(f)?);
                }
                Some(Self::Bulk(res))
            }
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
impl TryFrom<Value> for () {
    type Error = RedisError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Ok(())
    }
}
impl TryFrom<Value> for usize {
    type Error = RedisError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Int(d) => Ok(d.try_into().unwrap()),
            _ => Err(RedisError::ErrIllegalTypeConversion(format!("{:?}", value))),
        }
    }
}

impl TryFrom<Value> for Vec<String> {
    type Error = RedisError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bulk(v) => {
                let mut res = vec![];
                for elem in v {
                    match elem.try_into() {
                        Ok(o) => {
                            res.push(o);
                        }
                        Err(r) => return Err(r),
                    };
                }
                Ok(res)
            }
            _ => Err(RedisError::ErrIllegalTypeConversion(format!("{:?}", value))),
        }
    }
}

impl TryFrom<Value> for i64 {
    type Error = RedisError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Int(d) => Ok(d),
            Value::Status(s) => Err(RedisError::ErrorResponse(s)),
            _ => Err(RedisError::ErrIllegalTypeConversion(format!("{:?}", value))),
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
            _ => {
                let value = Value::from_frame(&mut self.recv).unwrap();
                let out: RedisResult<T> = value.try_into();
                out
            }
        }
    }
    fn exec<T>(&mut self, cmd: Cmd) -> RedisResult<T>
    where
        T: TryFrom<Value, Error = RedisError>,
    {
        let x = &cmd.bytes()[..];
        let length = x.len();
        match self.conn.write(x)? {
            n if n == length => Ok(()),
            _ => Err(RedisError::NoBytesWriten),
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
}

impl Client {
    fn new() -> types::RedisResult<Self> {
        Ok(Client {
            conn: Connection::new()?,
        })
    }
    fn set(&mut self, key: &str, value: &str) -> types::RedisResult<()> {
        self.conn.exec(Cmd::new().arg("SET").arg(key))
    }
    fn del(&mut self, key: &str) -> RedisResult<usize> {
        self.conn.exec(Cmd::new().arg("DEL").arg(key))
    }
    fn get(&mut self, key: &str) -> types::RedisResult<String> {
        self.conn.exec(Cmd::new().arg("GET").arg(key))
    }
    fn decr(&mut self, key: &str) -> types::RedisResult<i64> {
        self.conn.exec(Cmd::new().arg("DECR").arg(key))
    }
    fn incr(&mut self, key: &str) -> types::RedisResult<i64> {
        self.conn.exec(Cmd::new().arg("INCR").arg(key))
    }
    fn incrby(&mut self, key: &str, n: usize) -> RedisResult<i64> {
        self.conn.exec(Cmd::new().arg("INCRBY").arg(key).arg(n))
    }
    fn decrby(&mut self, key: &str, n: usize) -> RedisResult<i64> {
        self.conn.exec(Cmd::new().arg("DECRBY").arg(key).arg(n))
    }
    fn lpush<T: Into<Arg>>(&mut self, key: &str, value: T) -> RedisResult<usize> {
        self.conn.exec(Cmd::new().arg("LPUSH").arg(key).arg(value))
    }
    fn lpop(
        &mut self,
        key: &str,
        count: Option<std::num::NonZeroUsize>,
    ) -> RedisResult<Vec<String>> {
        // cast to something that can be passed as an arg
        let u: usize = match count {
            Some(d) => d.into(),
            None => 1,
        };
        self.conn.exec(Cmd::new().arg("LPOP").arg(key).arg(u))
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
        client.del("number").unwrap(); // cleanup
        assert_eq!(-1, client.decr("number")?);
        assert_eq!(-2, client.decr("number")?);
        assert_eq!(-1, client.incr("number")?);
        assert_eq!(0, client.incr("number")?);
        assert_eq!(1, client.incr("number")?);
        Ok(())
    }

    #[test]
    fn incrby() -> RedisResult<()> {
        let mut client = Client::new()?;
        client.del("incrby").unwrap();
        assert_eq!(10, client.incrby("incrby", 10)?);
        assert_eq!(20, client.incrby("incrby", 10)?);
        assert_eq!(-10, client.decrby("incrby", 30)?);
        Ok(())
    }

    #[test]
    fn lpush() -> RedisResult<()> {
        let mut client = Client::new()?;
        client.del("lpush").unwrap();
        assert_eq!(1, client.lpush("lpush", "a")?);
        assert_eq!(2, client.lpush("lpush", "b")?);
        assert_eq!(3, client.lpush("lpush", "c")?);
        assert_eq!(
            vec!["c", "b", "a"],
            client.lpop("lpush", Some(3.try_into().unwrap()))?
        );
        client.lpush("lpush", "a")?;
        assert_eq!(vec!["a"], client.lpop("lpush", None)?);
        client.lpush("lpush", "a")?;
        Ok(())
    }
}
