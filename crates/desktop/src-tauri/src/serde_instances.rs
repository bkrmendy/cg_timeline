use serde::{ser::SerializeStruct, Serialize, Serializer};
use timeline_lib::db::db_ops::{DBError, ShortCommitRecord};
pub struct ShortCommitRecordWrapper(pub ShortCommitRecord);

impl Serialize for ShortCommitRecordWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("ShortCommitRecordWrapper", 3)?;
        s.serialize_field("hash", &self.0.hash)?;
        s.serialize_field("branch", &self.0.branch)?;
        s.serialize_field("message", &self.0.message)?;
        s.end()
    }
}

pub struct DBErrorWrapper(pub DBError);

impl Serialize for DBErrorWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            DBError::Error(ref err) => serializer.serialize_str(err),
            DBError::Consistency(ref err) => serializer.serialize_str(err),
            DBError::Fundamental(ref err) => serializer.serialize_str(err),
        }
    }
}
