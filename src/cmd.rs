pub struct Cmd {
    args: Vec<Arg>,
}

pub struct Arg(String);

impl From<&str> for Arg {
    fn from(value: &str) -> Self {
        Arg(value.to_string())
    }
}
impl From<usize> for Arg {
    fn from(value: usize) -> Self {
        Arg(value.to_string())
    }
}

impl Cmd {
    pub fn new() -> Self {
        Self { args: vec![] }
    }
    pub fn arg<T>(mut self, a: T) -> Self
    where
        T: Into<Arg>,
    {
        self.args.push(a.into());
        self
    }
    pub fn bytes(self) -> Vec<u8> {
        // for each param, append \r\n
        let mut acc = vec![];
        self.args.iter().for_each(|a| {
            acc.extend_from_slice(a.0.as_bytes());
            acc.push(b'\r');
            acc.push(b'\n');
        });
        acc
    }
}
