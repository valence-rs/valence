use crate::Inventory;

pub trait IntoInventory {
    /// Converts an inventory wrapper back to an abstract inventory
    fn into_inventory(self) -> Inventory;
}
