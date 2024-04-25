use std::{ops::Range, str::FromStr};

use crate::Ty;

use super::{
    token::{Keyword, Token},
    Value,
};

pub type Result = std::result::Result<Token, TokenizeError>;

#[derive(Debug, Clone, Copy)]
pub enum TokenizeError {
    NonTerminatedStr,
    NonUTF8,
    UnexpectedCharacter,
    InvalidFloatLiteral,
    InvalidIntLiteral,
}

#[derive(Debug, Clone, Copy)]
pub struct TokenIter<'a> {
    bytes: &'a [u8],
    last_index: usize,
    index: usize,
}

impl<'a> TokenIter<'a> {
    pub fn src_pos(&self) -> Range<usize> {
        self.last_index..self.index
    }

    fn next_byte(&mut self) -> Option<u8> {
        let byte = self.bytes.get(self.index)?;
        self.index += 1;
        Some(*byte)
    }

    fn next_byte_if(&mut self, f: impl Fn(u8) -> bool) -> Option<u8> {
        if self.peek_byte().is_some_and(f) {
            return self.next_byte();
        }

        None
    }

    fn peek_byte(&self) -> Option<u8> {
        self.bytes.get(self.index).copied()
    }

    fn next_token(&mut self, byte: u8) -> Result {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' => {
                let mut bytes = vec![byte];
                while let Some(byte) = self.next_byte_if(
                    |byte| matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_'),
                ) {
                    bytes.push(byte);
                }

                // It can only have utf-8 bytes because of the code above.
                let str = unsafe { std::str::from_utf8_unchecked(&bytes) };
                if let Ok(keyword) = Keyword::from_str(str) {
                    return Ok(Token::Keyword(keyword));
                }

                if let Ok(ty) = Ty::from_str(str) {
                    return Ok(Token::Ty(ty));
                }

                Ok(Token::Identifier(str.to_owned()))
            }
            b'"' => {
                let mut bytes = Vec::new();
                while let Some(byte) = self.next_byte() {
                    match byte {
                        b'"' => {
                            let Ok(str) = String::from_utf8(bytes) else {
                                return Err(TokenizeError::NonUTF8);
                            };

                            return Ok(Token::Literal(Value::Str(str)));
                        }
                        byte => bytes.push(byte),
                    }
                }

                Err(TokenizeError::NonTerminatedStr)
            }
            b'0'..=b'9' => {
                let mut bytes = vec![byte];
                let mut dot = false;
                while let Some(byte) = self.next_byte_if(|byte| matches!(byte, b'.' | b'0'..=b'9'))
                {
                    match (byte, dot) {
                        (b'.', false) => dot = true,
                        (b'.', true) => break,
                        _ => {}
                    }

                    bytes.push(byte);
                }

                if dot {
                    // It can only have utf-8 bytes because of the code above.
                    let Ok(float) = unsafe { std::str::from_utf8_unchecked(&bytes) }.parse() else {
                        return Err(TokenizeError::InvalidFloatLiteral);
                    };

                    return Ok(Token::Literal(Value::Float(float)));
                }

                let Ok(int) = unsafe { std::str::from_utf8_unchecked(&bytes) }.parse() else {
                    return Err(TokenizeError::InvalidIntLiteral);
                };
                Ok(Token::Literal(Value::Int(int)))
            }
            b'*' => Ok(Token::Star),
            b',' => Ok(Token::Comma),
            b':' => Ok(Token::Colon),
            b';' => Ok(Token::SemiColon),
            b'@' => Ok(Token::At),
            b'(' => Ok(Token::LeftSmooth),
            b')' => Ok(Token::RightSmooth),
            b'?' => Ok(Token::QuestionMark),
            _ => Err(TokenizeError::UnexpectedCharacter),
        }
    }
}

impl<'a> From<&'a [u8]> for TokenIter<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            last_index: 0,
            index: 0,
        }
    }
}

impl<'a> Iterator for TokenIter<'a> {
    type Item = Result;

    fn next(&mut self) -> Option<Self::Item> {
        while self.peek_byte()?.is_ascii_whitespace() {
            _ = self.next_byte();
        }

        // Skip comments.
        while let Some(b'#') = self.peek_byte() {
            _ = self.next_byte();

            match self.next_byte()? {
                b'!' => loop {
                    if self.next_byte()? != b'!' {
                        continue;
                    }

                    if self.peek_byte()? == b'#' {
                        break;
                    }
                },
                b'\n' => {}
                _ => while self.next_byte()? != b'\n' {},
            }

            // Skip any whitespace after comments.
            while self.peek_byte()?.is_ascii_whitespace() {
                _ = self.next_byte();
            }
        }

        self.last_index = self.index;
        let byte = self.next_byte()?;
        Some(self.next_token(byte))
    }
}