use serde::Serialize;

struct DomainSubObject {
    id: u64,
    label: String,
}

struct DomainObject {
    field1: String,
    field2: Vec<DomainSubObject>,
}

#[derive(Serialize)]
struct DomainSubObjectFe {
    id: u64,
    name: String,
}

#[derive(Serialize)]
struct DomainObjectFe {
    title: String,
    items: Vec<DomainSubObjectFe>,
}

impl From<DomainSubObject> for DomainSubObjectFe {
    fn from(value: DomainSubObject) -> Self {
        Self {
            id: value.id,
            name: value.label,
        }
    }
}

impl From<DomainObject> for DomainObjectFe {
    fn from(value: DomainObject) -> Self {
        Self {
            title: value.field1,
            items: value.field2.into_iter().map(Into::into).collect(),
        }
    }
}
