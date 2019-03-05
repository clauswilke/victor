pub(crate) use self::definitions::ComputedValues;
pub(super) use self::definitions::ComputedValuesForLateCascade;
use self::definitions::LonghandId;
pub(super) use self::definitions::{property_data_by_name, LonghandDeclaration};
use crate::geom::{flow_relative, physical};
use crate::style::errors::PropertyParseError;
use crate::style::values::{self, CssWideKeyword, Direction, WritingMode};
use cssparser::{Color, RGBA};
use std::rc::Rc;

#[macro_use]
mod macros;

mod definitions;

impl ComputedValues {
    pub(crate) fn initial() -> Rc<Self> {
        Self::new(None, None)
    }

    pub(crate) fn anonymous_inheriting_from(parent_style: &Self) -> Rc<Self> {
        Self::new(Some(parent_style), None)
    }

    pub(super) fn post_cascade_fixups(&mut self) {
        let b = Rc::make_mut(&mut self.border);
        b.border_top_width.fixup(b.border_top_style);
        b.border_left_width.fixup(b.border_left_style);
        b.border_bottom_width.fixup(b.border_bottom_style);
        b.border_right_width.fixup(b.border_right_style);
    }

    pub(crate) fn writing_mode(&self) -> (WritingMode, Direction) {
        // FIXME: For now, this is the only supported mode
        (WritingMode::HorizontalTb, Direction::Ltr)
    }

    pub(crate) fn box_size(&self) -> flow_relative::Vec2<values::LengthOrPercentageOrAuto> {
        physical::Vec2 {
            x: self.box_.width,
            y: self.box_.height,
        }
        .size_to_flow_relative(self.writing_mode())
    }

    pub(crate) fn padding(&self) -> flow_relative::Sides<values::LengthOrPercentage> {
        physical::Sides {
            top: self.padding.padding_top,
            left: self.padding.padding_left,
            bottom: self.padding.padding_bottom,
            right: self.padding.padding_right,
        }
        .to_flow_relative(self.writing_mode())
    }

    pub(crate) fn border_width(&self) -> flow_relative::Sides<values::LengthOrPercentage> {
        physical::Sides {
            top: self.border.border_top_width.0,
            left: self.border.border_left_width.0,
            bottom: self.border.border_bottom_width.0,
            right: self.border.border_right_width.0,
        }
        .to_flow_relative(self.writing_mode())
    }

    pub(crate) fn margin(&self) -> flow_relative::Sides<values::LengthOrPercentageOrAuto> {
        physical::Sides {
            top: self.margin.margin_top,
            left: self.margin.margin_left,
            bottom: self.margin.margin_bottom,
            right: self.margin.margin_right,
        }
        .to_flow_relative(self.writing_mode())
    }

    pub(crate) fn to_rgba(&self, color: Color) -> RGBA {
        match color {
            Color::RGBA(rgba) => rgba,
            Color::CurrentColor => self.color.color,
        }
    }
}

#[derive(Copy, Clone)]
pub(super) struct Early;

#[derive(Copy, Clone)]
pub(super) struct Late;

pub(super) trait Phase: Copy {
    fn any(self, p: Phases) -> bool;
}

impl Phase for Early {
    fn any(self, p: Phases) -> bool {
        p.any_early
    }
}

impl Phase for Late {
    fn any(self, p: Phases) -> bool {
        p.any_late
    }
}

#[derive(Default, Copy, Clone)]
pub(super) struct Phases {
    pub any_early: bool,
    pub any_late: bool,
}

impl Phases {
    pub fn any(self) -> bool {
        self.any_early || self.any_late
    }
}

impl std::ops::BitOrAssign for Phases {
    fn bitor_assign(&mut self, other: Self) {
        self.any_early |= other.any_early;
        self.any_late |= other.any_late;
    }
}

type FnParseProperty = for<'i, 't> fn(
    &mut cssparser::Parser<'i, 't>,
    &mut Vec<LonghandDeclaration>,
) -> Result<Phases, PropertyParseError<'i>>;

pub struct PropertyData {
    pub(in crate::style) longhands: &'static [LonghandId],
    pub(in crate::style) parse: FnParseProperty,
}

trait ValueOrInitial<T> {
    fn into<F>(self, id: LonghandId, constructor: F) -> LonghandDeclaration
    where
        F: Fn(T) -> LonghandDeclaration;
}

impl<T> ValueOrInitial<T> for T {
    fn into<F>(self, _id: LonghandId, constructor: F) -> LonghandDeclaration
    where
        F: Fn(T) -> LonghandDeclaration,
    {
        constructor(self)
    }
}

impl<T> ValueOrInitial<T> for Option<T> {
    fn into<F>(self, id: LonghandId, constructor: F) -> LonghandDeclaration
    where
        F: Fn(T) -> LonghandDeclaration,
    {
        match self {
            Some(value) => constructor(value),
            None => LonghandDeclaration::CssWide(id, CssWideKeyword::Initial),
        }
    }
}
