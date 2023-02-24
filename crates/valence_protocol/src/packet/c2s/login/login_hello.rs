use uuid::Uuid;

use crate::username::Username;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct LoginHelloC2s<'a> {
    pub username: Username<&'a str>,
    pub profile_id: Option<Uuid>,
}
