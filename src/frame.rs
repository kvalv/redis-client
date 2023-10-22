pub struct Response {
    pub b: Vec<u8>,
    i: usize,
}
impl Response {
    pub fn new() -> Self {
        let mut vec = Vec::new();
        vec.resize(1024, 0);
        Self { b: vec, i: 0 }
    }
    pub fn reset(&mut self) {
        self.i = 0
    }
    pub fn read_next(&mut self) -> Option<String> {
        match self.b[..].windows(2).position(|pair| {
            let x = pair[0];
            let y = pair[1];
            x == b'\r' && y == b'\n'
        }) {
            Some(p) => {
                let rv = self.b[self.i..self.i + p - 1].to_owned();
                let r = String::from_utf8(rv).unwrap();
                self.i += r.len() + 2;
                Some(r)
            }
            None => None,
        }
    }
    pub fn peek(&self, n: usize) -> Option<String> {
        let slice = self.b[self.i..self.i + n].to_owned();
        String::from_utf8(slice.to_vec()).ok()
    }
    pub fn read(&mut self, n: usize) -> Option<String> {
        let res = self.peek(n);
        if res.is_some() {
            self.i += n;
        }
        res
    }
}
