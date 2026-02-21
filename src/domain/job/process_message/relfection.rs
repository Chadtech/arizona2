use crate::capability::job::JobCapability;
use crate::capability::message::MessageCapability;
use crate::capability::scene::SceneCapability;
use crate::nice_display::NiceDisplay;

pub enum Error {}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
			// Handle error variants here
		}
    }
}

pub async fn run<W>(worker: &W) -> Result<(), Error> {
    Ok(())
}
