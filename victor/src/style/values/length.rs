use crate::style::errors::{PropertyParseError, PropertyParseErrorKind};
use crate::style::values::{CascadeContext, FromSpecified, Parse, SpecifiedValue};
use cssparser::{Parser, Token};
use std::fmt;
use std::ops;

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub(crate) struct Length {
    pub px: f32,
}

/// <https://drafts.csswg.org/css-values/#percentages>
#[repr(transparent)]
#[derive(Copy, Clone, SpecifiedAsComputed)]
pub(crate) struct Percentage {
    unit_value: f32,
}

/// <https://drafts.csswg.org/css-values/#lengths>
#[derive(Clone, FromVariants)]
pub(in crate::style) enum SpecifiedLength {
    Absolute(Length),
    Em(f32),
}

#[derive(Clone, Parse, FromVariants)]
pub(in crate::style) enum SpecifiedLengthOrPercentage {
    Length(SpecifiedLength),
    Percentage(Percentage),
}

#[derive(Debug, Copy, Clone, FromSpecified, FromVariants)]
pub(crate) enum LengthOrPercentage {
    Length(Length),
    Percentage(Percentage),
}

#[derive(Clone, Parse, FromVariants)]
pub(in crate::style) enum SpecifiedLengthOrPercentageOrAuto {
    Length(SpecifiedLength),
    Percentage(Percentage),
    Auto,
}

#[derive(Debug, Copy, Clone, FromSpecified, FromVariants)]
pub(crate) enum LengthOrPercentageOrAuto {
    Length(Length),
    Percentage(Percentage),
    Auto,
}

#[derive(Copy, Clone, Debug, FromVariants, PartialEq)]
pub(crate) enum LengthOrAuto {
    Length(Length),
    Auto,
}

impl Parse for SpecifiedLength {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>> {
        match parser.next()? {
            Token::Dimension { value, unit, .. } => match_ignore_ascii_case!(unit,
                "px" => Ok(SpecifiedLength::Absolute(Length { px: *value })),
                "em" => Ok(SpecifiedLength::Em(*value)),
                _ => {
                    let u = unit.clone();
                    Err(parser.new_custom_error(PropertyParseErrorKind::UnknownUnit(u)))
                }
            ),
            Token::Number { value, .. } if *value == 0. => {
                Ok(SpecifiedLength::Absolute(Length { px: 0. }))
            }
            token => {
                let t = token.clone();
                Err(parser.new_unexpected_token_error(t))
            }
        }
    }
}

impl SpecifiedValue for Length {
    type SpecifiedValue = SpecifiedLength;
}

impl FromSpecified for Length {
    fn from_specified(s: &SpecifiedLength, context: &CascadeContext) -> Self {
        match s {
            SpecifiedLength::Absolute(px) => *px,
            SpecifiedLength::Em(value) => context.this.font_size().0 * *value,
        }
    }
}

impl Parse for Percentage {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>> {
        Ok(Percentage {
            unit_value: parser.expect_percentage()?,
        })
    }
}

impl Length {
    pub fn zero() -> Self {
        Length { px: 0. }
    }

    pub fn max(self, other: Self) -> Self {
        Length {
            px: self.px.max(other.px),
        }
    }

    pub fn min(self, other: Self) -> Self {
        Length {
            px: self.px.min(other.px),
        }
    }

    pub fn max_assign(&mut self, other: Self) {
        *self = self.max(other)
    }
}

impl ops::Neg for Length {
    type Output = Self;
    fn neg(self) -> Self {
        Length { px: -self.px }
    }
}

impl ops::Add for Length {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Length {
            px: self.px + other.px,
        }
    }
}

impl ops::Sub for Length {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Length {
            px: self.px - other.px,
        }
    }
}

impl ops::AddAssign for Length {
    fn add_assign(&mut self, other: Self) {
        self.px += other.px
    }
}

impl ops::SubAssign for Length {
    fn sub_assign(&mut self, other: Self) {
        self.px -= other.px
    }
}

impl ops::Mul<f32> for Length {
    type Output = Self;

    fn mul(self, other: f32) -> Self {
        Length {
            px: self.px * other,
        }
    }
}

impl ops::Mul<Percentage> for Length {
    type Output = Self;

    fn mul(self, other: Percentage) -> Self {
        self * other.unit_value
    }
}

impl ops::Div<f32> for Length {
    type Output = Self;

    fn div(self, other: f32) -> Self {
        Length {
            px: self.px / other,
        }
    }
}

impl From<Length> for euclid::Length<f32, crate::primitives::CssPx> {
    fn from(l: Length) -> Self {
        euclid::Length::new(l.px)
    }
}

impl fmt::Debug for Length {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.px.fmt(f)?;
        fmt::Write::write_str(f, "px")
    }
}

impl fmt::Debug for Percentage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (self.unit_value * 100.).fmt(f)?;
        fmt::Write::write_str(f, "%")
    }
}

impl LengthOrPercentage {
    pub(crate) fn percentage_relative_to(&self, reference: Length) -> Length {
        match *self {
            LengthOrPercentage::Length(l) => l,
            LengthOrPercentage::Percentage(p) => reference * p,
        }
    }
}

impl From<LengthOrPercentage> for LengthOrPercentageOrAuto {
    fn from(value: LengthOrPercentage) -> Self {
        match value {
            LengthOrPercentage::Length(l) => LengthOrPercentageOrAuto::Length(l),
            LengthOrPercentage::Percentage(p) => LengthOrPercentageOrAuto::Percentage(p),
        }
    }
}

impl LengthOrPercentageOrAuto {
    pub(crate) fn non_auto(&self) -> Option<LengthOrPercentage> {
        match *self {
            LengthOrPercentageOrAuto::Length(l) => Some(LengthOrPercentage::Length(l)),
            LengthOrPercentageOrAuto::Percentage(p) => Some(LengthOrPercentage::Percentage(p)),
            LengthOrPercentageOrAuto::Auto => None,
        }
    }

    pub(crate) fn percentage_relative_to(&self, reference: Length) -> LengthOrAuto {
        match *self {
            LengthOrPercentageOrAuto::Length(l) => LengthOrAuto::Length(l),
            LengthOrPercentageOrAuto::Percentage(p) => LengthOrAuto::Length(reference * p),
            LengthOrPercentageOrAuto::Auto => LengthOrAuto::Auto,
        }
    }
}

impl LengthOrAuto {
    pub(crate) fn auto_is(&self, auto_value: impl FnOnce() -> Length) -> Length {
        match *self {
            LengthOrAuto::Length(l) => l,
            LengthOrAuto::Auto => auto_value(),
        }
    }

    pub(crate) fn map(&self, f: impl FnOnce(Length) -> Length) -> Self {
        match *self {
            LengthOrAuto::Length(l) => LengthOrAuto::Length(f(l)),
            LengthOrAuto::Auto => LengthOrAuto::Auto,
        }
    }
}
