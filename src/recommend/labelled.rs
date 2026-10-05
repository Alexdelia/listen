use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::{declaration::Source, library::tag};

#[derive(Clone, Copy)]
pub(super) struct Labelled(pub Source);

impl Serialize for Labelled {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		let label = tag::plain_label(self.0);
		let mut state = serializer.serialize_struct("Labelled", 2)?;
		state.serialize_field("mbid", &self.0)?;
		state.serialize_field("label", &Some(label).filter(|label| !label.is_empty()))?;
		state.end()
	}
}
