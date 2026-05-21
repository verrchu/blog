use serde_json::{Value, json};

struct DomainObject {
    field1: String,
    field2: u64,
}

impl DomainObject {
    fn to_fe_json(&self) -> Value {
        json!({
            "displayName": self.field1,
            "count": self.field2,
        })
    }

    fn to_db_json(&self) -> Value {
        json!({
            "field1": self.field1,
            "field2": self.field2,
        })
    }
}
