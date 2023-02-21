use std::borrow::Cow;

use uuid::Uuid;

use crate::types::Property;
use crate::username::Username;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct LoginSuccessS2c<'a> {
    pub uuid: Uuid,
    pub username: Username<&'a str>,
    pub properties: Cow<'a, [Property]>,
}
