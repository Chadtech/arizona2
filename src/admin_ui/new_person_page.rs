use crate::admin_ui::s;
use crate::capability::person::{NewPerson, PersonCapability};
use crate::capability::person_identity::{NewPersonIdentity, PersonIdentityCapability};
use crate::domain::person_identity_uuid::PersonIdentityUuid;
use crate::domain::person_name::PersonName;
use crate::domain::person_uuid::PersonUuid;
use crate::worker::Worker;
use iced::{clipboard, widget as w, Element, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    name_field: String,
    identity_field: w::text_editor::Content,
    status: Status,
    lookup_name_field: String,
    lookup_status: LookupStatus,
}

enum Status {
    Ready,
    CreatingPerson,
    CreatingIdentity,
    Done,
    ErrorCreatingPerson(String),
    ErrorCreatingIdentity(String),
}

enum LookupStatus {
    Ready,
    Loading,
    Loaded {
        person_uuid: PersonUuid,
        identity: Option<String>,
        is_hibernating: bool,
        hibernation_status: HibernationStatus,
    },
    Error(String),
}

enum HibernationStatus {
    Ready,
    Updating,
    Error(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Storage {
    #[serde(default)]
    identity_field: String,
    #[serde(default)]
    name_field: String,
    #[serde(default)]
    lookup_name_field: String,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            identity_field: String::new(),
            name_field: String::new(),
            lookup_name_field: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    IdentityFieldChanged(w::text_editor::Action),
    NameFieldChanged(String),
    ClickedCreatePerson,
    PersonCreated(Result<PersonUuid, String>),
    IdentityCreated(Result<PersonIdentityUuid, String>),
    LookupNameChanged(String),
    ClickedLoadIdentity,
    ClickedCopyIdentity(String),
    LoadedIdentity(Result<(PersonUuid, Option<String>, bool), String>),
    ClickedSetHibernation {
        person_uuid: PersonUuid,
        is_hibernating: bool,
    },
    SetHibernationUpdated {
        is_hibernating: bool,
        result: Result<(), String>,
    },
}

impl Model {
    pub fn new(storage: &Storage) -> Self {
        Self {
            identity_field: w::text_editor::Content::with_text(&storage.identity_field),
            name_field: storage.name_field.clone(),
            status: Status::Ready,
            lookup_name_field: storage.lookup_name_field.clone(),
            lookup_status: LookupStatus::Ready,
        }
    }
    pub fn to_storage(&self) -> Storage {
        Storage {
            identity_field: self.identity_field.text(),
            name_field: self.name_field.clone(),
            lookup_name_field: self.lookup_name_field.clone(),
        }
    }

    pub fn update(&mut self, worker: Arc<Worker>, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::IdentityFieldChanged(action) => {
                self.identity_field.perform(action);
                Task::none()
            }
            Msg::NameFieldChanged(value) => {
                self.name_field = value;
                Task::none()
            }
            Msg::LookupNameChanged(value) => {
                self.lookup_name_field = value;
                self.lookup_status = LookupStatus::Ready;
                Task::none()
            }
            Msg::ClickedCreatePerson => match self.status {
                Status::Ready => {
                    self.status = Status::CreatingPerson;

                    let new_person = NewPerson {
                        person_name: PersonName::from_string(self.name_field.clone()),
                        person_uuid: PersonUuid::new(),
                    };

                    Task::perform(
                        async move { create_new_person(&worker, new_person).await },
                        Msg::PersonCreated,
                    )
                }
                _ => Task::none(),
            },
            Msg::PersonCreated(result) => match result {
                Ok(_) => {
                    self.status = Status::CreatingIdentity;

                    let new_identity = NewPersonIdentity {
                        person_identity_uuid: PersonIdentityUuid::new(),
                        identity: self.identity_field.text(),
                        person_name: self.name_field.clone(),
                    };

                    Task::perform(
                        async move { create_new_identity(&worker, new_identity).await },
                        Msg::IdentityCreated,
                    )
                }
                Err(err) => {
                    self.status = Status::ErrorCreatingPerson(err);
                    Task::none()
                }
            },
            Msg::IdentityCreated(result) => {
                self.status = match result {
                    Ok(_) => Status::Done,
                    Err(err) => Status::ErrorCreatingIdentity(err),
                };
                Task::none()
            }
            Msg::ClickedLoadIdentity => {
                self.lookup_status = LookupStatus::Loading;
                let person_name = self.lookup_name_field.clone();
                Task::perform(
                    async move { load_identity(&worker, person_name).await },
                    Msg::LoadedIdentity,
                )
            }
            Msg::ClickedCopyIdentity(identity) => clipboard::write(identity),
            Msg::LoadedIdentity(result) => {
                self.lookup_status = match result {
                    Ok((person_uuid, identity, is_hibernating)) => LookupStatus::Loaded {
                        person_uuid,
                        identity,
                        is_hibernating,
                        hibernation_status: HibernationStatus::Ready,
                    },
                    Err(err) => LookupStatus::Error(err),
                };
                Task::none()
            }
            Msg::ClickedSetHibernation {
                person_uuid,
                is_hibernating,
            } => {
                if let LookupStatus::Loaded {
                    hibernation_status,
                    ..
                } = &mut self.lookup_status
                {
                    *hibernation_status = HibernationStatus::Updating;
                }

                Task::perform(
                    async move {
                        worker
                            .set_person_hibernating(&person_uuid, is_hibernating)
                            .await
                    },
                    move |result| Msg::SetHibernationUpdated {
                        is_hibernating,
                        result,
                    },
                )
            }
            Msg::SetHibernationUpdated {
                is_hibernating,
                result,
            } => {
                if let LookupStatus::Loaded {
                    is_hibernating: current,
                    hibernation_status,
                    ..
                } = &mut self.lookup_status
                {
                    match result {
                        Ok(()) => {
                            *current = is_hibernating;
                            *hibernation_status = HibernationStatus::Ready;
                        }
                        Err(err) => {
                            *hibernation_status = HibernationStatus::Error(err);
                        }
                    }
                }
                Task::none()
            }
        }
    }
    pub fn view(&self) -> Element<'_, Msg> {
        let create_section = w::column![
            w::text("Create Person"),
            w::text("Person Name"),
            w::text_input("", &self.name_field).on_input(Msg::NameFieldChanged),
            w::text("Identity"),
            w::text_editor(&self.identity_field)
                .on_action(Msg::IdentityFieldChanged)
                .height(iced::Length::Fixed(220.0)),
            w::button("Create Person").on_press(Msg::ClickedCreatePerson),
            status_view(&self.status),
        ]
        .spacing(s::S2);

        let lookup_section = w::column![
            w::text("Lookup Person Identity"),
            w::row![
                w::text_input("Person name", &self.lookup_name_field)
                    .on_input(Msg::LookupNameChanged)
                    .on_submit(Msg::ClickedLoadIdentity),
                w::button("Load").on_press(Msg::ClickedLoadIdentity),
            ]
            .spacing(s::S1),
            lookup_status_view(&self.lookup_status),
        ]
        .spacing(s::S2);

        w::column![create_section, lookup_section]
            .spacing(s::S4)
            .into()
    }
}

fn status_view(status: &Status) -> Element<'_, Msg> {
    match status {
        Status::Ready => w::text("Ready").into(),
        Status::CreatingPerson { .. } => w::text("Creating person...").into(),
        Status::CreatingIdentity => w::text("Creating identity...").into(),
        Status::Done => w::text("Done!").into(),
        Status::ErrorCreatingPerson(err) => {
            w::text(format!("Error creating person: {}", err)).into()
        }
        Status::ErrorCreatingIdentity(err) => {
            w::text(format!("Error creating identity: {}", err)).into()
        }
    }
}

fn lookup_status_view(status: &LookupStatus) -> Element<'_, Msg> {
    match status {
        LookupStatus::Ready => w::text("Ready").into(),
        LookupStatus::Loading => w::text("Loading...").into(),
        LookupStatus::Loaded {
            person_uuid,
            identity,
            is_hibernating,
            hibernation_status,
        } => {
            let identity_text = match identity {
                Some(text) => text.as_str(),
                None => "No identity found",
            };
            let copy_button: Element<'_, Msg> = match identity {
                Some(text) => w::button("Copy identity")
                    .on_press(Msg::ClickedCopyIdentity(text.clone()))
                    .into(),
                None => w::text("").into(),
            };
            let hibernation_state_text = if *is_hibernating {
                "Hibernation: On"
            } else {
                "Hibernation: Off"
            };

            let hibernation_status_view: Element<'_, Msg> = match hibernation_status {
                HibernationStatus::Ready => w::text("").into(),
                HibernationStatus::Updating => w::text("Updating hibernation...").into(),
                HibernationStatus::Error(err) => {
                    w::text(format!("Error updating hibernation: {}", err)).into()
                }
            };

            let is_updating = match hibernation_status {
                HibernationStatus::Updating => true,
                _ => false,
            };

            let hibernate_button: Element<'_, Msg> = if *is_hibernating || is_updating {
                w::button("Put into hibernation").into()
            } else {
                w::button("Put into hibernation")
                    .on_press(Msg::ClickedSetHibernation {
                        person_uuid: person_uuid.clone(),
                        is_hibernating: true,
                    })
                    .into()
            };

            let wake_button: Element<'_, Msg> = if !*is_hibernating || is_updating {
                w::button("Take out of hibernation").into()
            } else {
                w::button("Take out of hibernation")
                    .on_press(Msg::ClickedSetHibernation {
                        person_uuid: person_uuid.clone(),
                        is_hibernating: false,
                    })
                    .into()
            };

            w::column![
                w::text(format!("Person UUID: {}", person_uuid.to_uuid())),
                w::text(identity_text),
                copy_button,
                w::text(hibernation_state_text),
                w::row![hibernate_button, wake_button].spacing(s::S1),
                hibernation_status_view,
            ]
            .spacing(s::S1)
            .into()
        }
        LookupStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
    }
}

async fn create_new_person(worker: &Worker, new_person: NewPerson) -> Result<PersonUuid, String> {
    worker.create_person(new_person).await
}

async fn create_new_identity(
    worker: &Worker,
    new_identity: NewPersonIdentity,
) -> Result<PersonIdentityUuid, String> {
    worker.create_person_identity(new_identity).await
}

async fn load_identity(
    worker: &Worker,
    person_name: String,
) -> Result<(PersonUuid, Option<String>, bool), String> {
    if person_name.trim().is_empty() {
        return Err("Person name cannot be empty".to_string());
    }

    let person_uuid = worker
        .get_person_uuid_by_name(PersonName::from_string(person_name))
        .await?;
    let identity = worker.get_person_identity(&person_uuid).await?;
    let is_hibernating = worker.is_person_hibernating(&person_uuid).await?;
    Ok((person_uuid, identity, is_hibernating))
}
