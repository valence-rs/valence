use uuid::Uuid;

use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct LoginHelloC2s<'a> {
    pub username: &'a str, // TODO: bound this
    pub profile_id: Option<Uuid>,
}
