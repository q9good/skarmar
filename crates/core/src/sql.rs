use crate::store::Result;

#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Text(String),
    Integer(i64),
}

impl From<&String> for Value {
    fn from(value: &String) -> Self {
        Self::Text(value.clone())
    }
}
impl From<&&str> for Value {
    fn from(value: &&str) -> Self {
        Self::Text((*value).into())
    }
}
impl From<&Option<String>> for Value {
    fn from(value: &Option<String>) -> Self {
        value.as_ref().map(Self::from).unwrap_or(Self::Null)
    }
}
impl From<&i64> for Value {
    fn from(value: &i64) -> Self {
        Self::Integer(*value)
    }
}

/// SQL operations used by the shared store. Only TEXT columns are queried today.
/// Values are always bound; callers never concatenate request data into SQL.
pub trait Sql {
    fn execute(&self, sql: &str, params: &[Value]) -> Result<()>;
    fn execute_batch(&self, sql: &str) -> Result<()>;
    fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Vec<String>>>;
}

macro_rules! params {
    ($($value:expr),* $(,)?) => { &[$(crate::sql::Value::from(&$value)),*] };
}
pub(crate) use params;
