use crate::util::traits::AsBytes;
use std::fmt::Display;
use bytemuck::NoUninit;
use num_traits::PrimInt;

/// CellCoord (Cell coordinate) represents the coordinate of a cell in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellCoord<T = usize>
    where T: PrimInt
{
    pub x: T,
    pub y: T,
}

impl<T: PrimInt> CellCoord<T> {
    /// Create a new cell coordinate.
    pub fn new(x: T, y: T) -> Self {
        CellCoord { x, y }
    }
}

impl<T> Display for CellCoord<T>
    where T: PrimInt + std::fmt::Debug
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// CellValue (Cell value) represents the value or status of a cell.
/// It should be either `Dead` (0_u8) or `Alive` (255_u8).
/// ## Examples
/// Create a new `Dead` cell, and change it to `Alive`.
/// ``` ignore
/// let mut cell = CellValue::Dead;
/// cell = CellValue::Alive;
/// assert_eq!(cell, CellValue::Alive); // The cell is `Alive` now
///
/// match cell {
///     CellValue::Dead => println!("It is a Dead cell"),
///     CellValue::Alive => println!("It is an Alive cell"),
/// }
/// ```
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, NoUninit)]
#[repr(u8)]
pub enum CellValue {
    #[default]
    Dead = 0,
    Alive = 255,
}

impl CellValue {
    /// Flip the cell. If it is currently `Alive`, flipping it will make it `Dead`, and vice versa.
    /// ## Examples
    /// ``` ignore
    /// let mut cell = CellValue::Alive;
    /// cell.flip();
    /// assert_eq!(cell, CellValue::Dead); // The cell is `Dead` now
    /// ```
    pub fn flip(&mut self) {
        *self = match self {
            CellValue::Dead => Self::Alive,
            CellValue::Alive => Self::Dead,
        }
    }

    /// Create a new flipped cell.
    /// ## Examples
    /// ``` ignore
    /// let cell = CellValue::Alive;
    /// let new_cell = cell.into_flipped();
    /// assert_eq!(new_cell, CellValue::Dead); // The `new_cell` is `Dead`
    /// assert_eq!(cell, CellValue::Alive); // The `cell` remains `Alive`
    /// ```
    pub fn into_flipped(self) -> Self {
        match self {
            CellValue::Dead => Self::Alive,
            CellValue::Alive => Self::Dead,
        }
    }

    /// Check if the cell is `Dead`.
    pub fn is_dead(&self) -> bool {
        *self == CellValue::Dead
    }

    /// Check if the cell is `Alive`.
    pub fn is_alive(&self) -> bool {
        *self == CellValue::Alive
    }

    /// Cast a single `CellValue` to u8 (byte).
    /// ## Examples
    /// ``` ignore
    /// let byte = CellValue::Alive.as_u8();
    /// assert_eq!(byte, 255_u8);
    /// ```
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

impl<T: PrimInt> From<T> for CellValue {
    /// Convert any valid integer (0 or 255) to `CellValue`.
    /// ## Examples
    /// ``` ignore
    /// let int: u32 = 255;
    /// let cell = CellValue::from(int);
    /// let cell: CellValue = int.into(); // This does the same
    /// ```
    fn from(value: T) -> Self {
        let value = value.to_u8().expect("CellValue should be either 0 or 255");
        match value {
            0 => CellValue::Dead,
            255 => CellValue::Alive,
            _ => panic!("CellValue should be either 0 or 255"),
        }
    }
}

impl From<CellValue> for u8 {
    /// Convert `CellValue` to u8 (byte).
    /// ## Examples
    /// ``` ignore
    /// let cell = CellValue::Alive;
    /// let byte: u8 = cell.into();
    /// ```
    fn from(value: CellValue) -> Self {
        value.as_u8()
    }
}

impl AsBytes for [CellValue] {
    /// Cast a `CellValue` slice to byte slice (&[u8]).
    /// ## Examples
    /// ``` ignore
    /// let world = vec![CellValue::Alive; 10];
    /// for byte in world.as_bytes() {
    ///     print!("{} ", byte);
    ///     assert_eq!(*byte, 255_u8);
    /// }
    /// ```
    fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self)
    }
}

impl Display for CellValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
