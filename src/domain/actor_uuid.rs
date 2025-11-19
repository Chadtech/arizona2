use crate::domain::person_uuid::PersonUuid;
#[derive(Debug, Clone)]
pub enum ActorUuid {
    AiPerson(PersonUuid),
    RealWorldUser,
}

impl ActorUuid {
    pub fn to_label(&self) -> String {
        match self {
            ActorUuid::AiPerson(person_uuid) => format!("AI Person {}", person_uuid.to_uuid()),
            ActorUuid::RealWorldUser => "Real World User".to_string(),
        }
    }

    pub fn from_person_uuid(person_uuid: PersonUuid) -> Self {
        ActorUuid::AiPerson(person_uuid)
    }
}
