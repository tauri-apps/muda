use std::{convert::Infallible, str::FromStr};

/// An unique id that is associated with a menu or a menu item.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct MenuId(pub String);

impl MenuId {
    /// Create a new menu id.
    pub fn new<S: AsRef<str>>(id: S) -> Self {
        Self(id.as_ref().to_string())
    }
}

impl AsRef<str> for MenuId {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl From<String> for MenuId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for MenuId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl FromStr for MenuId {
    type Err = Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

impl PartialEq<&str> for MenuId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for MenuId {
    fn eq(&self, other: &String) -> bool {
        self.0 == *other
    }
}

impl PartialEq<&String> for MenuId {
    fn eq(&self, other: &&String) -> bool {
        self.0 == **other
    }
}

impl PartialEq<&MenuId> for MenuId {
    fn eq(&self, other: &&MenuId) -> bool {
        other.0 == self.0
    }
}

#[cfg(test)]
mod test {
    use crate::MenuId;

    #[test]
    fn is_eq() {
        assert_eq!(MenuId::new("t"), "t",);
        assert_eq!(MenuId::new("t"), String::from("t"));
        assert_eq!(MenuId::new("t"), &String::from("t"));
        assert_eq!(MenuId::new("t"), MenuId::new("t"));
        assert_eq!(MenuId::new("t"), &MenuId::new("t"));
        assert_eq!(&MenuId::new("t"), &MenuId::new("t"));
        assert_eq!(MenuId::new("t").as_ref(), "t");
    }
}
