pub(crate) use crate::style::values::Length;

pub(crate) mod physical {
    #[derive(Debug, Clone)]
    pub(crate) struct Vec2<T> {
        pub x: T,
        pub y: T,
    }

    #[derive(Debug, Clone)]
    pub(crate) struct Rect<T> {
        pub top_left: Vec2<T>,
        pub size: Vec2<T>,
    }

    #[derive(Debug, Clone)]
    pub(crate) struct Sides<T> {
        pub top: T,
        pub left: T,
        pub bottom: T,
        pub right: T,
    }
}

pub(crate) mod flow_relative {
    #[derive(Debug, Clone)]
    pub(crate) struct Vec2<T> {
        pub inline: T,
        pub block: T,
    }

    #[derive(Debug, Clone)]
    pub(crate) struct Rect<T> {
        pub start_corner: Vec2<T>,
        pub size: Vec2<T>,
    }

    #[derive(Debug, Clone)]
    pub(crate) struct Sides<T> {
        pub inline_start: T,
        pub inline_end: T,
        pub block_start: T,
        pub block_end: T,
    }
}

use crate::style::values::{Direction, WritingMode};
use std::ops::{Add, Sub};

impl<T: Clone> physical::Vec2<T> {
    pub fn size_to_flow_relative(&self, mode: (WritingMode, Direction)) -> flow_relative::Vec2<T> {
        // https://drafts.csswg.org/css-writing-modes/#logical-to-physical
        let (i, b) = if let (WritingMode::HorizontalTb, _) = mode {
            (&self.x, &self.y)
        } else {
            (&self.y, &self.x)
        };
        flow_relative::Vec2 {
            inline: i.clone(),
            block: b.clone(),
        }
    }
}

impl<T: Clone> flow_relative::Vec2<T> {
    pub fn size_to_physical(&self, mode: (WritingMode, Direction)) -> physical::Vec2<T> {
        // https://drafts.csswg.org/css-writing-modes/#logical-to-physical
        let (x, y) = if let (WritingMode::HorizontalTb, _) = mode {
            (&self.inline, &self.block)
        } else {
            (&self.block, &self.inline)
        };
        physical::Vec2 {
            x: x.clone(),
            y: y.clone(),
        }
    }
}

impl<T: Clone> physical::Sides<T> {
    pub fn to_flow_relative(&self, mode: (WritingMode, Direction)) -> flow_relative::Sides<T> {
        use Direction::*;
        use WritingMode::*;

        // https://drafts.csswg.org/css-writing-modes/#logical-to-physical
        let (bs, be) = match mode.0 {
            HorizontalTb => (&self.top, &self.bottom),
            VerticalRl | SidewaysRl => (&self.right, &self.left),
            VerticalLr | SidewaysLr => (&self.left, &self.right),
        };
        let (is, ie) = match mode {
            (HorizontalTb, Ltr) => (&self.left, &self.right),
            (HorizontalTb, Rtl) => (&self.right, &self.left),
            (VerticalRl, Ltr) | (SidewaysRl, Ltr) | (VerticalLr, Ltr) | (SidewaysLr, Rtl) => {
                (&self.top, &self.bottom)
            }
            (VerticalRl, Rtl) | (SidewaysRl, Rtl) | (VerticalLr, Rtl) | (SidewaysLr, Ltr) => {
                (&self.bottom, &self.top)
            }
        };
        flow_relative::Sides {
            inline_start: is.clone(),
            inline_end: ie.clone(),
            block_start: bs.clone(),
            block_end: be.clone(),
        }
    }
}

impl<T> flow_relative::Sides<T> {
    pub fn map<U>(&self, f: impl Fn(&T) -> U) -> flow_relative::Sides<U> {
        flow_relative::Sides {
            inline_start: f(&self.inline_start),
            inline_end: f(&self.inline_end),
            block_start: f(&self.block_start),
            block_end: f(&self.block_end),
        }
    }

    pub fn map_inline_and_block_axes<U>(
        &self,
        inline_f: impl Fn(&T) -> U,
        block_f: impl Fn(&T) -> U,
    ) -> flow_relative::Sides<U> {
        flow_relative::Sides {
            inline_start: inline_f(&self.inline_start),
            inline_end: inline_f(&self.inline_end),
            block_start: block_f(&self.block_start),
            block_end: block_f(&self.block_end),
        }
    }

    pub fn inline_sum(&self) -> T::Output
    where
        T: Add + Copy,
    {
        self.inline_start + self.inline_end
    }

    pub fn block_sum(&self) -> T::Output
    where
        T: Add + Copy,
    {
        self.block_start + self.block_end
    }

    pub fn start_corner(&self) -> flow_relative::Vec2<T>
    where
        T: Clone,
    {
        flow_relative::Vec2 {
            inline: self.inline_start.clone(),
            block: self.block_start.clone(),
        }
    }
}

impl<T, U> Add<&'_ flow_relative::Sides<U>> for &'_ flow_relative::Sides<T>
where
    T: Add<U> + Copy,
    U: Copy,
{
    type Output = flow_relative::Sides<T::Output>;

    fn add(self, other: &'_ flow_relative::Sides<U>) -> Self::Output {
        flow_relative::Sides {
            inline_start: self.inline_start + other.inline_start,
            inline_end: self.inline_end + other.inline_end,
            block_start: self.block_start + other.block_start,
            block_end: self.block_end + other.block_end,
        }
    }
}

impl<T> flow_relative::Rect<T> {
    pub fn inflate(&self, sides: &flow_relative::Sides<T>) -> Self
    where
        T: Add<Output = T> + Copy,
        T: Sub<Output = T> + Copy,
    {
        flow_relative::Rect {
            start_corner: flow_relative::Vec2 {
                inline: self.start_corner.inline - sides.inline_start,
                block: self.start_corner.block - sides.block_start,
            },
            size: flow_relative::Vec2 {
                inline: self.size.inline + sides.inline_sum(),
                block: self.size.block + sides.block_sum(),
            },
        }
    }

    pub fn to_physical(
        &self,
        mode: (WritingMode, Direction),
        // Will be needed for other writing modes
        // FIXME: what if the containing block has a different mode?
        // https://drafts.csswg.org/css-writing-modes/#orthogonal-flows
        _containing_block: &physical::Rect<T>,
    ) -> physical::Rect<T>
    where
        T: Clone,
    {
        // Top-left corner
        let (tl_x, tl_y) = if let (WritingMode::HorizontalTb, Direction::Ltr) = mode {
            (&self.start_corner.inline, &self.start_corner.block)
        } else {
            unimplemented!()
        };
        physical::Rect {
            top_left: physical::Vec2 {
                x: tl_x.clone(),
                y: tl_y.clone(),
            },
            size: self.size.size_to_physical(mode),
        }
    }
}
