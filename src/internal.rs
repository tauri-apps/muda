
//!  **DO NOT USE:**. This module is ONLY meant to be used internally.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuItemType {
    Submenu,
    Normal,
    Check,
    Predefined,
}

impl Default for MenuItemType {
    fn default() -> Self {
        Self::Normal
    }
}

/// # Safety
///
/// **DO NOT IMPLEMENT:** This trait is ONLY meant to be implemented internally.
pub unsafe trait MenuEntry {
    fn type_(&self) -> MenuItemType;

    fn as_any(&self) -> &(dyn std::any::Any + 'static);

    fn id(&self) -> u32;
}
