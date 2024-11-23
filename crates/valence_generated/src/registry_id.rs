#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegistryId(i32);

impl RegistryId {
    pub fn new(id: i32) -> Self {
        Self(id)
    }

    pub fn id(&self) -> i32 {
        self.0
    }
}

impl From<RegistryId> for i32 {
    fn from(id: RegistryId) -> i32 {
        id.id()
    }
}

impl From<i32> for RegistryId {
    fn from(id: i32) -> RegistryId {
        RegistryId::new(id)
    }
}
