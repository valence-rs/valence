use std::borrow::Cow;

use uuid::Uuid;

use crate::types::Property;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct LoginSuccessS2c<'a> {
    pub uuid: Uuid,
    pub username: &'a str, // TODO: bound this.
    pub properties: Cow<'a, [Property]>,
}
