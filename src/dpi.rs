pub trait Pixel: Copy + Into<f64> {
    fn from_f64(f: f64) -> Self;
    fn cast<P: Pixel>(self) -> P {
        P::from_f64(self.into())
    }
}

macro_rules! pixel_int_impl {
    ($($t:ty),*) => {$(
        impl Pixel for $t {
            fn from_f64(f: f64) -> Self {
                f.round() as $t
            }
        }
    )*}
  }

pixel_int_impl!(u8, u16, u32, i8, i16, i32);

impl Pixel for f32 {
    fn from_f64(f: f64) -> Self {
        f as f32
    }
}

impl Pixel for f64 {
    fn from_f64(f: f64) -> Self {
        f
    }
}

macro_rules! from_impls {
    ($t:ident ) => {
        impl<P: Pixel, X: Pixel> From<(X, X)> for $t<P> {
            fn from((x, y): (X, X)) -> Self {
                Self::new(x.cast(), y.cast())
            }
        }

        impl<P: Pixel, X: Pixel> From<$t<P>> for (X, X) {
            fn from(p: $t<P>) -> Self {
                (p.x.cast(), p.y.cast())
            }
        }

        impl<P: Pixel, X: Pixel> From<[X; 2]> for $t<P> {
            fn from([x, y]: [X; 2]) -> Self {
                Self::new(x.cast(), y.cast())
            }
        }

        impl<P: Pixel, X: Pixel> From<$t<P>> for [X; 2] {
            fn from(p: $t<P>) -> Self {
                [p.x.cast(), p.y.cast()]
            }
        }
    };
}

/// Checks that the scale factor is a normal positive `f64`.
///
/// All functions that take a scale factor assert that this will return `true`. If you're sourcing scale factors from
/// anywhere other than tao, it's recommended to validate them using this function before passing them to tao;
/// otherwise, you risk panics.
#[inline]
pub fn validate_scale_factor(scale_factor: f64) -> bool {
    scale_factor.is_sign_positive() && scale_factor.is_normal()
}

/// A position represented in logical pixels.
///
/// The position is stored as floats, so please be careful. Casting floats to integers truncates the
/// fractional part, which can cause noticable issues. To help with that, an `Into<(i32, i32)>`
/// implementation is provided which does the rounding for you.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LogicalPosition<P> {
    pub x: P,
    pub y: P,
}

impl<P> LogicalPosition<P> {
    #[inline]
    pub const fn new(x: P, y: P) -> Self {
        LogicalPosition { x, y }
    }
}

impl<P: Pixel> LogicalPosition<P> {
    #[inline]
    pub fn from_physical<T: Into<PhysicalPosition<X>>, X: Pixel>(
        physical: T,
        scale_factor: f64,
    ) -> Self {
        physical.into().to_logical(scale_factor)
    }

    #[inline]
    pub fn to_physical<X: Pixel>(&self, scale_factor: f64) -> PhysicalPosition<X> {
        assert!(validate_scale_factor(scale_factor));
        let x = self.x.into() * scale_factor;
        let y = self.y.into() * scale_factor;
        PhysicalPosition::new(x, y).cast()
    }

    #[inline]
    pub fn cast<X: Pixel>(&self) -> LogicalPosition<X> {
        LogicalPosition {
            x: self.x.cast(),
            y: self.y.cast(),
        }
    }
}

from_impls!(LogicalPosition);

/// A position represented in physical pixels.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PhysicalPosition<P> {
    pub x: P,
    pub y: P,
}

impl<P> PhysicalPosition<P> {
    #[inline]
    pub const fn new(x: P, y: P) -> Self {
        PhysicalPosition { x, y }
    }
}

impl<P: Pixel> PhysicalPosition<P> {
    #[inline]
    pub fn from_logical<T: Into<LogicalPosition<X>>, X: Pixel>(
        logical: T,
        scale_factor: f64,
    ) -> Self {
        logical.into().to_physical(scale_factor)
    }

    #[inline]
    pub fn to_logical<X: Pixel>(&self, scale_factor: f64) -> LogicalPosition<X> {
        assert!(validate_scale_factor(scale_factor));
        let x = self.x.into() / scale_factor;
        let y = self.y.into() / scale_factor;
        LogicalPosition::new(x, y).cast()
    }

    #[inline]
    pub fn cast<X: Pixel>(&self) -> PhysicalPosition<X> {
        PhysicalPosition {
            x: self.x.cast(),
            y: self.y.cast(),
        }
    }
}

from_impls!(PhysicalPosition);

/// A position that's either physical or logical.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Position {
    Physical(PhysicalPosition<i32>),
    Logical(LogicalPosition<f64>),
}

impl Position {
    pub fn new<S: Into<Position>>(val: S) -> Position {
        val.into()
    }

    pub fn to_logical<P: Pixel>(&self, scale_factor: f64) -> LogicalPosition<P> {
        match *self {
            Position::Physical(val) => val.to_logical(scale_factor),
            Position::Logical(val) => val.cast(),
        }
    }

    pub fn to_physical<P: Pixel>(&self, scale_factor: f64) -> PhysicalPosition<P> {
        match *self {
            Position::Physical(val) => val.cast(),
            Position::Logical(val) => val.to_physical(scale_factor),
        }
    }
}

impl<P: Pixel> From<PhysicalPosition<P>> for Position {
    #[inline]
    fn from(val: PhysicalPosition<P>) -> Position {
        Position::Physical(val.cast())
    }
}

impl<P: Pixel> From<LogicalPosition<P>> for Position {
    #[inline]
    fn from(val: LogicalPosition<P>) -> Position {
        Position::Logical(val.cast())
    }
}
