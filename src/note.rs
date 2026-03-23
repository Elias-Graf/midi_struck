use std::fmt::Display;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Note {
    index: u8,
}

impl Note {
    pub fn new(index: u8) -> Self {
        Self { index }
    }

    /// # Examples
    ///
    /// ```
    /// use midi_struck::note::{Note, PitchClass};
    ///
    /// assert_eq!(
    ///     Note::from_parts(PitchClass::C, 4).index(),
    ///     60
    /// );
    /// assert_eq!(
    ///     Note::from_parts(PitchClass::ASharp, 8).index(),
    ///     118
    /// );
    /// ```
    pub fn from_parts(pitch_class: PitchClass, octave: u8) -> Self {
        let index = (octave + 1) * 12 + pitch_class.index_offset();

        Self { index }
    }

    /// # Examples
    ///
    /// ```
    /// use midi_struck::note::{Note, PitchClass};
    ///
    /// let a0 = Note::new(21);
    /// assert_eq!(a0.octave(), 0);
    /// assert_eq!(a0.pitch_class(), PitchClass::A);
    ///
    /// let f9s = Note::new(126);
    /// assert_eq!(f9s.octave(), 9);
    /// assert_eq!(f9s.pitch_class(), PitchClass::FSharp);
    /// ```
    pub fn index(&self) -> u8 {
        self.index
    }

    pub fn octave(&self) -> u8 {
        (self.index / 12).saturating_sub(1)
    }

    pub const fn pitch_class(&self) -> PitchClass {
        match self.index % 12 {
            0 => PitchClass::C,
            1 => PitchClass::CSharp,
            2 => PitchClass::D,
            3 => PitchClass::DSharp,
            4 => PitchClass::E,
            5 => PitchClass::F,
            6 => PitchClass::FSharp,
            7 => PitchClass::G,
            8 => PitchClass::GSharp,
            9 => PitchClass::A,
            10 => PitchClass::ASharp,
            _ => PitchClass::B,
        }
    }
}

impl Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let octave = self.octave();
        let pitch_class = self.pitch_class();

        write!(f, "{pitch_class}{octave} ({})", self.index)
    }
}

impl From<u8> for Note {
    fn from(value: u8) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum PitchClass {
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
    A,
    ASharp,
    B,
}

impl PitchClass {
    /// # Examples
    ///
    /// ```
    /// use midi_struck::note::PitchClass;
    ///
    /// assert_eq!(PitchClass::C.index_offset(), 0);
    /// assert_eq!(PitchClass::CSharp.index_offset(), 1);
    /// assert_eq!(PitchClass::D.index_offset(), 2);
    /// assert_eq!(PitchClass::DSharp.index_offset(), 3);
    /// assert_eq!(PitchClass::E.index_offset(), 4);
    /// assert_eq!(PitchClass::F.index_offset(), 5);
    /// assert_eq!(PitchClass::FSharp.index_offset(), 6);
    /// assert_eq!(PitchClass::G.index_offset(), 7);
    /// assert_eq!(PitchClass::GSharp.index_offset(), 8);
    /// assert_eq!(PitchClass::A.index_offset(), 9);
    /// assert_eq!(PitchClass::ASharp.index_offset(), 10);
    /// assert_eq!(PitchClass::B.index_offset(), 11);
    /// ```
    pub const fn index_offset(&self) -> u8 {
        match self {
            PitchClass::C => 0,
            PitchClass::CSharp => 1,
            PitchClass::D => 2,
            PitchClass::DSharp => 3,
            PitchClass::E => 4,
            PitchClass::F => 5,
            PitchClass::FSharp => 6,
            PitchClass::G => 7,
            PitchClass::GSharp => 8,
            PitchClass::A => 9,
            PitchClass::ASharp => 10,
            PitchClass::B => 11,
        }
    }
}

impl Display for PitchClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PitchClass::CSharp => write!(f, "C♯"),
            PitchClass::DSharp => write!(f, "D♯"),
            PitchClass::FSharp => write!(f, "F♯"),
            PitchClass::GSharp => write!(f, "G♯"),
            PitchClass::ASharp => write!(f, "A♯"),
            other => write!(f, "{other:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::note::{Note, PitchClass};

    #[test]
    fn from_index_a0() {
        assert_eq!(Note::from(21), Note::from_parts(PitchClass::A, 0));
    }

    #[test]
    fn from_index_c5() {
        assert_eq!(Note::from(60), Note::from_parts(PitchClass::C, 4));
    }

    #[test]
    fn from_index_c5_sharp() {
        assert_eq!(Note::from(61), Note::from_parts(PitchClass::CSharp, 4));
    }

    #[test]
    fn from_index_d5() {
        assert_eq!(Note::from(62), Note::from_parts(PitchClass::D, 4));
    }

    #[test]
    fn from_index_d5_sharp() {
        assert_eq!(Note::from(63), Note::from_parts(PitchClass::DSharp, 4));
    }

    #[test]
    fn from_index_e5() {
        assert_eq!(Note::from(64), Note::from_parts(PitchClass::E, 4));
    }

    #[test]
    fn from_index_f5() {
        assert_eq!(Note::from(65), Note::from_parts(PitchClass::F, 4));
    }

    #[test]
    fn from_index_f5_sharp() {
        assert_eq!(Note::from(66), Note::from_parts(PitchClass::FSharp, 4));
    }

    #[test]
    fn from_index_g5() {
        assert_eq!(Note::from(67), Note::from_parts(PitchClass::G, 4));
    }

    #[test]
    fn from_index_g5_sharp() {
        assert_eq!(Note::from(68), Note::from_parts(PitchClass::GSharp, 4));
    }

    #[test]
    fn from_index_a5() {
        assert_eq!(Note::from(69), Note::from_parts(PitchClass::A, 4));
    }

    #[test]
    fn from_index_a5_sharp() {
        assert_eq!(Note::from(70), Note::from_parts(PitchClass::ASharp, 4));
    }

    #[test]
    fn from_index_b5() {
        assert_eq!(Note::from(71), Note::from_parts(PitchClass::B, 4));
    }
}
