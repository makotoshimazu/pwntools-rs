use std::ops::{AddAssign, Mul};

#[derive(Debug, Clone, Default)]
pub struct Payload {
    data: Vec<u8>,
}

#[derive(Debug)]
pub struct P64(pub u64);

impl AddAssign<P64> for Payload {
    fn add_assign(&mut self, rhs: P64) {
        self.data.extend_from_slice(&rhs.0.to_le_bytes())
    }
}

impl AddAssign<&[u8]> for Payload {
    fn add_assign(&mut self, rhs: &[u8]) {
        self.data.extend_from_slice(rhs)
    }
}

impl AddAssign<Vec<u8>> for Payload {
    fn add_assign(&mut self, rhs: Vec<u8>) {
        self.data.extend_from_slice(&rhs)
    }
}

impl Mul<usize> for P64 {
    type Output = Vec<u8>;

    fn mul(self, rhs: usize) -> Self::Output {
        self.0.to_le_bytes().repeat(rhs)
    }
}

impl Payload {
    pub fn ljust(&mut self, size: usize, value: u8) {
        self.data.resize(size, value)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}
