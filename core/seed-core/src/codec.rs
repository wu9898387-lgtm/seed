#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EncodeError {
    FieldTooLarge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DecodeError {
    UnexpectedEof,
    TrailingBytes,
}

pub(crate) struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
        }
    }

    pub(crate) fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub(crate) fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn fixed(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    pub(crate) fn bytes(&mut self, value: &[u8]) -> Result<(), EncodeError> {
        let len = u32::try_from(value.len()).map_err(|_| EncodeError::FieldTooLarge)?;
        self.u32(len);
        self.fixed(value);
        Ok(())
    }

    pub(crate) fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

pub(crate) struct Decoder<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Decoder<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], DecodeError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(DecodeError::UnexpectedEof)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(DecodeError::UnexpectedEof)?;
        self.offset = end;
        Ok(value)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_be_bytes(self.fixed()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_be_bytes(self.fixed()?))
    }

    pub(crate) fn u64(&mut self) -> Result<u64, DecodeError> {
        Ok(u64::from_be_bytes(self.fixed()?))
    }

    pub(crate) fn i64(&mut self) -> Result<i64, DecodeError> {
        Ok(i64::from_be_bytes(self.fixed()?))
    }

    pub(crate) fn fixed<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    pub(crate) fn bytes(&mut self) -> Result<&'a [u8], DecodeError> {
        let len = self.u32()? as usize;
        self.take(len)
    }

    pub(crate) fn finish(self) -> Result<(), DecodeError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(DecodeError::TrailingBytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_round_trip() {
        let mut encoder = Encoder::with_capacity(40);
        encoder.u8(7);
        encoder.u16(8);
        encoder.u32(9);
        encoder.u64(10);
        encoder.i64(-11);
        encoder.bytes(b"seed").unwrap();

        let encoded = encoder.finish();
        let mut decoder = Decoder::new(&encoded);

        assert_eq!(decoder.u8().unwrap(), 7);
        assert_eq!(decoder.u16().unwrap(), 8);
        assert_eq!(decoder.u32().unwrap(), 9);
        assert_eq!(decoder.u64().unwrap(), 10);
        assert_eq!(decoder.i64().unwrap(), -11);
        assert_eq!(decoder.bytes().unwrap(), b"seed");
        decoder.finish().unwrap();
    }

    #[test]
    fn trailing_bytes_are_rejected() {
        let mut decoder = Decoder::new(&[1, 2]);
        assert_eq!(decoder.u8().unwrap(), 1);
        assert_eq!(decoder.finish(), Err(DecodeError::TrailingBytes));
    }
}
