use std::{fmt::Display, mem};

pub fn variable_length_quantity(input: &[u8]) -> Result<(usize, u32), ParseError> {
    const CONTINUATION_BIT_MASK: u8 = 0x80;

    if input.is_empty() {
        return Err(ParseError::InputEmpty);
    }

    let mut result: u32 = 0;
    for i in 0..4 {
        let Some(byte) = input.get(i) else {
            break;
        };
        let is_continuation_bit_set = byte & CONTINUATION_BIT_MASK > 0;

        result = (result << 7) + (byte & !CONTINUATION_BIT_MASK) as u32;
        if !is_continuation_bit_set {
            return Ok((i + 1, result));
        } else if i == 3 {
            return Err(ParseError::ContinuationIn4thByte);
        }
    }

    Err(ParseError::ContinuationAtEndOfInput)
}

pub fn variable_length_quantity_usize(input: &[u8]) -> Result<(usize, usize), ParseError> {
    let (consumed, value): (_, u32) = variable_length_quantity(input)?;

    const { assert!(mem::size_of::<u32>() <= mem::size_of::<usize>()) };
    Ok((consumed, value as usize))
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    InputEmpty,
    ContinuationIn4thByte,
    ContinuationAtEndOfInput,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InputEmpty => write!(f, "input empty"),
            // TODO: add bytes?
            ParseError::ContinuationIn4thByte => write!(
                f,
                "received continuation in 4th byte, max. size of variable length quantity is 4 bytes"
            ),
            ParseError::ContinuationAtEndOfInput => write!(
                f,
                "received continuation, but there is no more input left to process"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::variable_length_quantity::{ParseError, variable_length_quantity};

    #[test]
    fn empty_input() {
        assert_eq!(variable_length_quantity(&[]), Err(ParseError::InputEmpty));
    }

    #[test]
    fn too_long_4th_byte_has_continuation() {
        assert_eq!(
            variable_length_quantity(&[0xF0, 0xF1, 0xF2, 0xF3]),
            Err(ParseError::ContinuationIn4thByte)
        );
    }

    #[test]
    fn too_long_last_byte_in_input_has_continuation() {
        assert_eq!(
            variable_length_quantity(&[0xF0]),
            Err(ParseError::ContinuationAtEndOfInput)
        );
    }

    #[test]
    fn from_variable_length_quantity() {
        let x: Vec<(&[u8], u32)> = vec![
            (&[0x0], 0x0),
            (&[0b00000100], 4),
            (&[0b01011000], 88),
            (&[0x40], 0x40),
            (&[0x7F], 0x7F),
            (&[0x81, 0x00], 0x80),
            (&[0xC0, 0x00], 0x002000),
            (&[0xFF, 0x7F], 0x00003FFF),
            (&[0x81, 0x80, 0x00], 0x00004000),
            (&[0xC0, 0x80, 0x00], 0x00100000),
            (&[0xFF, 0xFF, 0x7F], 0x001FFFFF),
            (&[0x81, 0x80, 0x80, 0x00], 0x00200000),
            (&[0xC0, 0x80, 0x80, 0x00], 0x08000000),
            (&[0xFF, 0xFF, 0xFF, 0x7F], 0x0FFFFFFF),
        ];

        for (i, (input, result)) in x.iter().enumerate() {
            assert_eq!(
                variable_length_quantity(input),
                Ok((input.len(), *result)),
                "test failed:{i}\n{:#?} != {:b}",
                input.iter().map(|x| format!("{:b}", x)).collect::<Vec<_>>(),
                result
            );
        }
    }
}
