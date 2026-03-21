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
    /// use midi_struck::note::{Note, PitchClass, HalfTone};
    ///
    /// assert_eq!(
    ///     Note::from_parts(PitchClass::C, 4, None).index(),
    ///     60
    /// );
    /// assert_eq!(
    ///     Note::from_parts(PitchClass::A, 8, Some(HalfTone::Sharp)).index(),
    ///     118
    /// );
    /// ```
    pub fn from_parts(pitch_class: PitchClass, octave: u8, half_tone: Option<HalfTone>) -> Self {
        let half_tone_offset = half_tone.map(|v| v.index_offset()).unwrap_or(0);
        let index =
            ((octave + 1) * 12 + pitch_class.index_offset()).wrapping_add_signed(half_tone_offset);

        Self { index }
    }

    /// # Examples
    ///
    /// ```
    /// use midi_struck::note::{Note, PitchClass, HalfTone};
    ///
    /// let a0 = Note::new(21);
    /// assert_eq!(a0.octave(), 0);
    /// assert_eq!(a0.pitch_class_half_tone(), (PitchClass::A, None));
    ///
    /// let f9s = Note::new(126);
    /// assert_eq!(f9s.octave(), 9);
    /// assert_eq!(
    ///     f9s.pitch_class_half_tone(),
    ///     (PitchClass::F, Some(HalfTone::Sharp))
    /// );
    /// ```
    pub fn index(&self) -> u8 {
        self.index
    }

    pub fn octave(&self) -> u8 {
        (self.index / 12).saturating_sub(1)
    }

    pub const fn pitch_class_half_tone(&self) -> (PitchClass, Option<HalfTone>) {
        match self.index % 12 {
            0 => (PitchClass::C, None),
            1 => (PitchClass::C, Some(HalfTone::Sharp)),
            2 => (PitchClass::D, None),
            3 => (PitchClass::D, Some(HalfTone::Sharp)),
            4 => (PitchClass::E, None),
            5 => (PitchClass::F, None),
            6 => (PitchClass::F, Some(HalfTone::Sharp)),
            7 => (PitchClass::G, None),
            8 => (PitchClass::G, Some(HalfTone::Sharp)),
            9 => (PitchClass::A, None),
            10 => (PitchClass::A, Some(HalfTone::Sharp)),
            _ => (PitchClass::B, None),
        }
    }
}

impl Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let octave = self.octave();
        let (pitch_class, half_tone) = self.pitch_class_half_tone();

        write!(f, "{pitch_class}{octave}")?;
        if let Some(half_tone) = half_tone {
            write!(f, "{half_tone}")?;
        };
        write!(f, " ({})", self.index)?;

        Ok(())
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
    D,
    E,
    F,
    G,
    A,
    B,
}

impl PitchClass {
    /// # Examples
    ///
    /// ```
    /// use midi_struck::note::PitchClass;
    ///
    /// assert_eq!(PitchClass::C.index_offset(), 0);
    /// assert_eq!(PitchClass::D.index_offset(), 2);
    /// assert_eq!(PitchClass::E.index_offset(), 4);
    /// assert_eq!(PitchClass::F.index_offset(), 5);
    /// assert_eq!(PitchClass::G.index_offset(), 7);
    /// assert_eq!(PitchClass::A.index_offset(), 9);
    /// assert_eq!(PitchClass::B.index_offset(), 11);
    /// ```
    pub const fn index_offset(&self) -> u8 {
        match self {
            PitchClass::C => 0,
            PitchClass::D => 2,
            PitchClass::E => 4,
            PitchClass::F => 5,
            PitchClass::G => 7,
            PitchClass::A => 9,
            PitchClass::B => 11,
        }
    }
}

impl Display for PitchClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// TODO: I think there was a proper name for this... accent? accident?
// TODO: The above comment should still be relevant for strcore, but I'm pretty sure midi doesn't
// have a concept of sharp or flat. So remove?
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum HalfTone {
    Flat,
    Sharp,
}

impl HalfTone {
    /// # Examples
    ///
    /// ```
    /// use midi_struck::note::HalfTone;
    ///
    /// assert_eq!(HalfTone::Flat.index_offset(), -1);
    /// assert_eq!(HalfTone::Sharp.index_offset(), 1);
    /// ```
    pub const fn index_offset(&self) -> i8 {
        match self {
            HalfTone::Flat => -1,
            HalfTone::Sharp => 1,
        }
    }
}

impl Display for HalfTone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let symbol = match self {
            HalfTone::Flat => "♭",
            HalfTone::Sharp => "♯",
        };

        write!(f, "{symbol}",)
    }
}

#[cfg(test)]
mod tests {
    use crate::note::{HalfTone, Note, PitchClass};

    #[test]
    fn from_index_a0() {
        assert_eq!(Note::from(21), Note::from_parts(PitchClass::A, 0, None));
    }

    #[test]
    fn from_index_c5() {
        assert_eq!(Note::from(60), Note::from_parts(PitchClass::C, 4, None));
    }

    #[test]
    fn from_index_c5_sharp() {
        assert_eq!(
            Note::from(61),
            Note::from_parts(PitchClass::C, 4, Some(HalfTone::Sharp))
        );
    }

    #[test]
    fn from_index_d5() {
        assert_eq!(Note::from(62), Note::from_parts(PitchClass::D, 4, None));
    }

    #[test]
    fn from_index_d5_sharp() {
        assert_eq!(
            Note::from(63),
            Note::from_parts(PitchClass::D, 4, Some(HalfTone::Sharp))
        );
    }

    #[test]
    fn from_index_e5() {
        assert_eq!(Note::from(64), Note::from_parts(PitchClass::E, 4, None));
    }

    #[test]
    fn from_index_f5() {
        assert_eq!(Note::from(65), Note::from_parts(PitchClass::F, 4, None));
    }

    #[test]
    fn from_index_f5_sharp() {
        assert_eq!(
            Note::from(66),
            Note::from_parts(PitchClass::F, 4, Some(HalfTone::Sharp))
        );
    }

    #[test]
    fn from_index_g5() {
        assert_eq!(Note::from(67), Note::from_parts(PitchClass::G, 4, None));
    }

    #[test]
    fn from_index_g5_sharp() {
        assert_eq!(
            Note::from(68),
            Note::from_parts(PitchClass::G, 4, Some(HalfTone::Sharp))
        );
    }

    #[test]
    fn from_index_a5() {
        assert_eq!(Note::from(69), Note::from_parts(PitchClass::A, 4, None));
    }

    #[test]
    fn from_index_a5_sharp() {
        assert_eq!(
            Note::from(70),
            Note::from_parts(PitchClass::A, 4, Some(HalfTone::Sharp))
        );
    }

    #[test]
    fn from_index_b5() {
        assert_eq!(Note::from(71), Note::from_parts(PitchClass::B, 4, None));
    }
}
