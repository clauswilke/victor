//! Flow layout, also known as block-and-inline layout.

use super::*;
use rayon::prelude::*;
use rayon_croissant::ParallelIteratorExt;

mod construct;
mod float;
mod inline;
mod root;

pub(super) use construct::*;
pub(super) use float::*;
use inline::*;

#[derive(Debug)]
pub(super) struct BlockFormattingContext {
    pub contents: BlockContainer,
    pub contains_floats: ContainsFloats,
}

#[derive(Debug)]
pub(super) enum BlockContainer {
    BlockLevelBoxes(Vec<BlockLevelBox>),
    InlineFormattingContext(InlineFormattingContext),
}

#[derive(Debug)]
pub(super) enum BlockLevelBox {
    SameFormattingContextBlock {
        style: Arc<ComputedValues>,
        contents: BlockContainer,
    },
    OutOfFlowAbsolutelyPositionedBox(AbsolutelyPositionedBox),
    OutOfFlowFloatBox(FloatBox),
    Independent {
        style: Arc<ComputedValues>,
        contents: IndependentFormattingContext,
    },
}

impl BlockFormattingContext {
    pub(super) fn layout(
        &self,
        containing_block: &ContainingBlock,
        tree_rank: usize,
    ) -> (Vec<Fragment>, Vec<AbsolutelyPositionedFragment>, Length) {
        let mut float_list = self.contains_floats.into_list();
        let mut float_context = float_list
            .as_mut()
            .map(|list| FloatContext::root(containing_block.inline_size, list));
        self.contents
            .layout(containing_block, tree_rank, float_context.as_mut())
    }
}

impl BlockContainer {
    pub(super) fn layout(
        &self,
        containing_block: &ContainingBlock,
        tree_rank: usize,
        float_context: Option<&mut FloatContext>,
    ) -> (Vec<Fragment>, Vec<AbsolutelyPositionedFragment>, Length) {
        match self {
            BlockContainer::BlockLevelBoxes(child_boxes) => {
                let (child_fragments, mut absolutely_positioned_fragments, content_block_size) =
                    layout_block_level_children(containing_block, child_boxes, float_context);

                for abspos_fragment in &mut absolutely_positioned_fragments {
                    let child_fragment_rect = match &child_fragments[abspos_fragment.tree_rank] {
                        Fragment::Box(b) => &b.content_rect,
                        Fragment::Anonymous(a) => &a.rect,
                        _ => unreachable!(),
                    };

                    abspos_fragment.tree_rank = tree_rank;

                    if let AbsoluteBoxOffsets::StaticStart { start } =
                        &mut abspos_fragment.inline_start
                    {
                        *start += child_fragment_rect.start_corner.inline;
                    }

                    if let AbsoluteBoxOffsets::StaticStart { start } =
                        &mut abspos_fragment.block_start
                    {
                        *start += child_fragment_rect.start_corner.block;
                    }
                }

                let block_size = containing_block.block_size.auto_is(|| content_block_size);

                (child_fragments, absolutely_positioned_fragments, block_size)
            }
            BlockContainer::InlineFormattingContext(ifc) => ifc.layout(containing_block, tree_rank),
        }
    }
}

fn layout_block_level_children<'a>(
    containing_block: &ContainingBlock,
    child_boxes: &'a [BlockLevelBox],
    float_context: Option<&mut FloatContext>,
) -> (Vec<Fragment>, Vec<AbsolutelyPositionedFragment<'a>>, Length) {
    fn adjust_block_axis(fragment: &mut Fragment, content_block_size: &mut Length) -> Length {
        let (bpm, rect) = match fragment {
            Fragment::Box(fragment) => (
                fragment.padding.block_sum()
                    + fragment.border.block_sum()
                    + fragment.margin.block_sum(),
                &mut fragment.content_rect,
            ),
            Fragment::Anonymous(fragment) => (Length::zero(), &mut fragment.rect),
            _ => unreachable!(),
        };
        // FIXME: margin collapsing
        rect.start_corner.block += *content_block_size;
        let fragment_total_block_size = bpm + rect.size.block;
        *content_block_size += fragment_total_block_size;
        fragment_total_block_size
    }

    let mut content_block_size = Length::zero();
    let mut absolutely_positioned_fragments = vec![];
    let child_fragments = if let Some(float_context) = float_context {
        // FIXME(nox): This should probably use Vec::with_capacity.
        let mut child_fragments = vec![];

        for (tree_rank, box_) in child_boxes.iter().enumerate() {
            let mut fragment = match box_ {
                BlockLevelBox::SameFormattingContextBlock { style, contents } => {
                    in_flow_non_replaced(
                        containing_block,
                        &mut absolutely_positioned_fragments,
                        style,
                        |containing_block, start_corner| {
                            contents.layout(
                                containing_block,
                                tree_rank,
                                Some(
                                    &mut float_context
                                        .child(containing_block.inline_size, start_corner),
                                ),
                            )
                        },
                    )
                }
                BlockLevelBox::Independent { style, contents } => match contents.as_replaced() {
                    Ok(replaced) => {
                        // FIXME
                        match *replaced {}
                    }
                    Err(contents) => in_flow_non_replaced(
                        containing_block,
                        &mut absolutely_positioned_fragments,
                        style,
                        |containing_block, _| contents.layout(containing_block, tree_rank),
                    ),
                },
                BlockLevelBox::OutOfFlowAbsolutelyPositionedBox(box_) => {
                    absolutely_positioned_fragments.push(box_.layout(Vec2::zero(), tree_rank));
                    Fragment::Anonymous(AnonymousFragment::no_op(containing_block.mode))
                }
                BlockLevelBox::OutOfFlowFloatBox(_box_) => {
                    // TODO
                    Fragment::Anonymous(AnonymousFragment::no_op(containing_block.mode))
                }
            };
            float_context
                .advance_block_start(adjust_block_axis(&mut fragment, &mut content_block_size));
            child_fragments.push(fragment);
        }

        child_fragments
    } else {
        let mut child_fragments = child_boxes
            .par_iter()
            .enumerate()
            .mapfold_reduce_into(
                &mut absolutely_positioned_fragments,
                |abspos_fragments, (tree_rank, box_)| {
                    match box_ {
                        BlockLevelBox::SameFormattingContextBlock { style, contents } => {
                            in_flow_non_replaced(
                                containing_block,
                                abspos_fragments,
                                style,
                                |containing_block, _start_corner| {
                                    contents.layout(containing_block, tree_rank, None)
                                },
                            )
                        }
                        BlockLevelBox::Independent { style, contents } => {
                            match contents.as_replaced() {
                                Ok(replaced) => {
                                    // FIXME
                                    match *replaced {}
                                }
                                Err(contents) => in_flow_non_replaced(
                                    containing_block,
                                    abspos_fragments,
                                    style,
                                    |containing_block, _start_corner| {
                                        contents.layout(containing_block, tree_rank)
                                    },
                                ),
                            }
                        }
                        BlockLevelBox::OutOfFlowAbsolutelyPositionedBox(box_) => {
                            abspos_fragments.push(box_.layout(Vec2::zero(), tree_rank));
                            Fragment::Anonymous(AnonymousFragment::no_op(containing_block.mode))
                        }
                        BlockLevelBox::OutOfFlowFloatBox(_box_) => {
                            panic!("encountered a float box in parallel layout")
                        }
                    }
                },
                |left_abspos_fragments, mut right_abspos_fragments| {
                    left_abspos_fragments.append(&mut right_abspos_fragments);
                },
            )
            .collect::<Vec<_>>();

        for fragment in &mut child_fragments {
            adjust_block_axis(fragment, &mut content_block_size);
        }

        child_fragments
    };

    (
        child_fragments,
        absolutely_positioned_fragments,
        content_block_size,
    )
}

/// https://drafts.csswg.org/css2/visudet.html#blockwidth
/// https://drafts.csswg.org/css2/visudet.html#normal-block
fn in_flow_non_replaced<'a>(
    containing_block: &ContainingBlock,
    absolutely_positioned_fragments: &mut Vec<AbsolutelyPositionedFragment<'a>>,
    style: &Arc<ComputedValues>,
    layout_contents: impl FnOnce(
        &ContainingBlock,
        &Vec2<Length>,
    ) -> (Vec<Fragment>, Vec<AbsolutelyPositionedFragment<'a>>, Length),
) -> Fragment {
    let cbis = containing_block.inline_size;
    let padding = style.padding().percentages_relative_to(cbis);
    let border = style.border_width().percentages_relative_to(cbis);
    let mut computed_margin = style.margin().percentages_relative_to(cbis);
    let pb = &padding + &border;
    let box_size = style.box_size();
    let inline_size = box_size.inline.percentage_relative_to(cbis);
    if let LengthOrAuto::Length(is) = inline_size {
        let inline_margins = cbis - is - pb.inline_sum();
        use LengthOrAuto::*;
        match (
            &mut computed_margin.inline_start,
            &mut computed_margin.inline_end,
        ) {
            (s @ &mut Auto, e @ &mut Auto) => {
                *s = Length(inline_margins / 2.);
                *e = Length(inline_margins / 2.);
            }
            (s @ &mut Auto, _) => {
                *s = Length(inline_margins);
            }
            (_, e @ &mut Auto) => {
                *e = Length(inline_margins);
            }
            (_, e @ _) => {
                // Either the inline-end margin is auto,
                // or we’re over-constrained and we do as if it were.
                *e = Length(inline_margins);
            }
        }
    }
    let margin = computed_margin.auto_is(Length::zero);
    let pbm = &pb + &margin;
    let inline_size = inline_size.auto_is(|| cbis - pbm.inline_sum());
    let block_size = match box_size.block {
        LengthOrPercentageOrAuto::Length(l) => LengthOrAuto::Length(l),
        LengthOrPercentageOrAuto::Percentage(p) => containing_block.block_size.map(|cbbs| cbbs * p),
        LengthOrPercentageOrAuto::Auto => LengthOrAuto::Auto,
    };
    let containing_block_for_children = ContainingBlock {
        inline_size,
        block_size,
        mode: style.writing_mode(),
    };
    // https://drafts.csswg.org/css-writing-modes/#orthogonal-flows
    assert_eq!(
        containing_block.mode, containing_block_for_children.mode,
        "Mixed writing modes are not supported yet"
    );
    let start_corner = pbm.start_corner();
    let (mut children, nested_abspos, content_block_size) =
        layout_contents(&containing_block_for_children, &start_corner);
    let relative_adjustement = relative_adjustement(style, inline_size, block_size);
    let block_size = block_size.auto_is(|| content_block_size);
    let content_rect = Rect {
        start_corner: &start_corner + &relative_adjustement,
        size: Vec2 {
            block: block_size,
            inline: inline_size,
        },
    };
    if style.box_.position.is_relatively_positioned() {
        AbsolutelyPositionedFragment::in_positioned_containing_block(
            &nested_abspos,
            &mut children,
            &content_rect.size,
            &padding,
            containing_block_for_children.mode,
        )
    } else {
        absolutely_positioned_fragments.extend(nested_abspos);
    };
    Fragment::Box(BoxFragment {
        style: style.clone(),
        children,
        content_rect,
        padding,
        border,
        margin,
    })
}