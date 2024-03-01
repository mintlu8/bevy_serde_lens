
pub enum Value {
    Readable(serde_value::Value),
    Bytes(Vec<u8>),
}

impl serde::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        match self {
            Value::Readable(value) => value.serialize(serializer),
            Value::Bytes(bytes) => bytes.serialize(serializer),
        }
    }
}