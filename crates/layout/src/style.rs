//! `LayoutStyle`: the box a node presents to layout, in our own words. Taffy's
//! style traits are implemented on it here, converting per getter, so no
//! taffy type is a field and none appears in a builder.

use taffy::geometry::{Rect as TRect, Size as TSize};
use taffy::style::{
    AlignContent, AlignItems, BlockContainerStyle, BlockItemStyle, BoxGenerationMode, CoreStyle,
    Dimension, FlexDirection, FlexWrap, FlexboxContainerStyle, FlexboxItemStyle, LengthPercentage,
    LengthPercentageAuto, Position as TPosition,
};
use utils::Edges;

// ---------------------------------------------------------------------------
// Values
// ---------------------------------------------------------------------------

/// A length as layout understands it. Built with [`auto`], [`px`] and
/// [`percent`]; there is deliberately no conversion from a bare number.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Val {
    #[default]
    Auto,
    Px(f32),
    /// Of the parent, `0.0..=100.0`.
    Percent(f32),
}

pub const fn auto() -> Val {
    Val::Auto
}

pub const fn px(v: f32) -> Val {
    Val::Px(v)
}

/// `percent(50.0)` is half of the parent.
pub const fn percent(v: f32) -> Val {
    Val::Percent(v)
}

impl Val {
    fn dimension(self) -> Dimension {
        match self {
            Val::Auto => Dimension::auto(),
            Val::Px(v) => Dimension::length(v),
            Val::Percent(p) => Dimension::percent(p / 100.0),
        }
    }

    fn length_percentage_auto(self) -> LengthPercentageAuto {
        match self {
            Val::Auto => LengthPercentageAuto::auto(),
            Val::Px(v) => LengthPercentageAuto::length(v),
            Val::Percent(p) => LengthPercentageAuto::percent(p / 100.0),
        }
    }

    /// Where taffy has no `auto`, `Auto` is zero.
    fn length_percentage(self) -> LengthPercentage {
        match self {
            Val::Auto => LengthPercentage::length(0.0),
            Val::Px(v) => LengthPercentage::length(v),
            Val::Percent(p) => LengthPercentage::percent(p / 100.0),
        }
    }
}

fn edges_lpa(e: Edges<Val>) -> TRect<LengthPercentageAuto> {
    TRect {
        left: e.left.length_percentage_auto(),
        right: e.right.length_percentage_auto(),
        top: e.top.length_percentage_auto(),
        bottom: e.bottom.length_percentage_auto(),
    }
}

fn edges_lp(e: Edges<Val>) -> TRect<LengthPercentage> {
    TRect {
        left: e.left.length_percentage(),
        right: e.right.length_percentage(),
        top: e.top.length_percentage(),
        bottom: e.bottom.length_percentage(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Display {
    #[default]
    Flex,
    Block,
    /// The node and its subtree take no space and get no box.
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    #[default]
    Relative,
    /// Taken out of the parent's flow and placed by `inset`; sized only by
    /// its own style.
    Absolute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    Start,
    Center,
    End,
    #[default]
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Wrap {
    #[default]
    No,
    Yes,
}

impl Align {
    fn items(self) -> AlignItems {
        match self {
            Align::Start => AlignItems::Start,
            Align::Center => AlignItems::Center,
            Align::End => AlignItems::End,
            Align::Stretch => AlignItems::Stretch,
        }
    }
}

impl Justify {
    fn content(self) -> AlignContent {
        match self {
            Justify::Start => AlignContent::Start,
            Justify::Center => AlignContent::Center,
            Justify::End => AlignContent::End,
            Justify::SpaceBetween => AlignContent::SpaceBetween,
            Justify::SpaceAround => AlignContent::SpaceAround,
            Justify::SpaceEvenly => AlignContent::SpaceEvenly,
        }
    }
}

// ---------------------------------------------------------------------------
// LayoutStyle
// ---------------------------------------------------------------------------

/// The box a node presents to layout. Dense: every node has one. Written by
/// reading it, changing it with the verbs, and `set`ting it back.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutStyle {
    pub display: Display,
    pub direction: Direction,
    pub position: Position,
    pub justify: Justify,
    /// How wrapped lines are packed along the cross axis. `Start` packs
    /// them, so a wrapping row of fixed-size children leaves its free space
    /// at the end rather than spreading the lines across it.
    pub align_content: Justify,
    pub align_items: Align,
    /// `None` follows the parent's `align_items`.
    pub align_self: Option<Align>,
    pub wrap: Wrap,

    pub width: Val,
    pub height: Val,
    pub min_width: Val,
    pub min_height: Val,
    pub max_width: Val,
    pub max_height: Val,
    pub flex_basis: Val,
    pub flex_grow: f32,
    pub flex_shrink: f32,

    pub row_gap: Val,
    pub column_gap: Val,
    pub padding: Edges<Val>,
    pub margin: Edges<Val>,
    pub border: Edges<Val>,
    pub inset: Edges<Val>,
}

/// CSS's defaults: a flex row, relative, auto-sized, no spacing, shrinkable.
/// Margin in particular is zero and not `Auto`, because an auto margin on a
/// flex item eats the free space that `justify` would otherwise distribute.
impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            display: Display::Flex,
            direction: Direction::Row,
            position: Position::Relative,
            justify: Justify::Start,
            align_content: Justify::Start,
            align_items: Align::Stretch,
            align_self: None,
            wrap: Wrap::No,
            width: Val::Auto,
            height: Val::Auto,
            min_width: Val::Auto,
            min_height: Val::Auto,
            max_width: Val::Auto,
            max_height: Val::Auto,
            flex_basis: Val::Auto,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            row_gap: Val::Px(0.0),
            column_gap: Val::Px(0.0),
            padding: Edges::all(Val::Px(0.0)),
            margin: Edges::all(Val::Px(0.0)),
            border: Edges::all(Val::Px(0.0)),
            inset: Edges::all(Val::Auto),
        }
    }
}

/// The verbs. All by value, so a style is rewritten in one expression.
impl LayoutStyle {
    pub fn display(mut self, d: Display) -> Self {
        self.display = d;
        self
    }
    pub fn flex(self) -> Self {
        self.display(Display::Flex)
    }
    pub fn block(self) -> Self {
        self.display(Display::Block)
    }
    pub fn hidden(self) -> Self {
        self.display(Display::Hidden)
    }

    pub fn row(mut self) -> Self {
        self.direction = Direction::Row;
        self
    }
    pub fn column(mut self) -> Self {
        self.direction = Direction::Column;
        self
    }

    pub fn absolute(mut self) -> Self {
        self.position = Position::Absolute;
        self
    }
    pub fn relative(mut self) -> Self {
        self.position = Position::Relative;
        self
    }

    pub fn justify(mut self, j: Justify) -> Self {
        self.justify = j;
        self
    }
    pub fn align_content(mut self, j: Justify) -> Self {
        self.align_content = j;
        self
    }
    pub fn align_items(mut self, a: Align) -> Self {
        self.align_items = a;
        self
    }
    pub fn align_self(mut self, a: Align) -> Self {
        self.align_self = Some(a);
        self
    }
    /// Centres children on both axes.
    pub fn center(self) -> Self {
        self.justify(Justify::Center).align_items(Align::Center)
    }
    pub fn wrap(mut self) -> Self {
        self.wrap = Wrap::Yes;
        self
    }

    pub fn width(mut self, v: Val) -> Self {
        self.width = v;
        self
    }
    pub fn height(mut self, v: Val) -> Self {
        self.height = v;
        self
    }
    pub fn size(self, w: Val, h: Val) -> Self {
        self.width(w).height(h)
    }
    /// The whole of the parent on both axes.
    pub fn fill(self) -> Self {
        self.size(percent(100.0), percent(100.0))
    }
    pub fn min_width(mut self, v: Val) -> Self {
        self.min_width = v;
        self
    }
    pub fn min_height(mut self, v: Val) -> Self {
        self.min_height = v;
        self
    }
    pub fn max_width(mut self, v: Val) -> Self {
        self.max_width = v;
        self
    }
    pub fn max_height(mut self, v: Val) -> Self {
        self.max_height = v;
        self
    }
    pub fn flex_basis(mut self, v: Val) -> Self {
        self.flex_basis = v;
        self
    }
    pub fn grow(mut self, g: f32) -> Self {
        self.flex_grow = g;
        self
    }
    pub fn shrink(mut self, s: f32) -> Self {
        self.flex_shrink = s;
        self
    }

    /// The same gap between rows and between columns.
    pub fn gap(mut self, v: Val) -> Self {
        self.row_gap = v;
        self.column_gap = v;
        self
    }
    pub fn row_gap(mut self, v: Val) -> Self {
        self.row_gap = v;
        self
    }
    pub fn column_gap(mut self, v: Val) -> Self {
        self.column_gap = v;
        self
    }
    pub fn padding(mut self, e: Edges<Val>) -> Self {
        self.padding = e;
        self
    }
    pub fn padding_all(self, v: Val) -> Self {
        self.padding(Edges::all(v))
    }
    pub fn margin(mut self, e: Edges<Val>) -> Self {
        self.margin = e;
        self
    }
    pub fn border(mut self, e: Edges<Val>) -> Self {
        self.border = e;
        self
    }
    pub fn inset(mut self, e: Edges<Val>) -> Self {
        self.inset = e;
        self
    }
}

// ---------------------------------------------------------------------------
// Taffy's view of it
// ---------------------------------------------------------------------------

impl CoreStyle for LayoutStyle {
    type CustomIdent = String;

    fn box_generation_mode(&self) -> BoxGenerationMode {
        match self.display {
            Display::Hidden => BoxGenerationMode::None,
            _ => BoxGenerationMode::Normal,
        }
    }
    fn is_block(&self) -> bool {
        self.display == Display::Block
    }
    fn position(&self) -> TPosition {
        match self.position {
            Position::Relative => TPosition::Relative,
            Position::Absolute => TPosition::Absolute,
        }
    }
    fn inset(&self) -> TRect<LengthPercentageAuto> {
        edges_lpa(self.inset)
    }
    fn size(&self) -> TSize<Dimension> {
        TSize {
            width: self.width.dimension(),
            height: self.height.dimension(),
        }
    }
    fn min_size(&self) -> TSize<Dimension> {
        TSize {
            width: self.min_width.dimension(),
            height: self.min_height.dimension(),
        }
    }
    fn max_size(&self) -> TSize<Dimension> {
        TSize {
            width: self.max_width.dimension(),
            height: self.max_height.dimension(),
        }
    }
    fn margin(&self) -> TRect<LengthPercentageAuto> {
        edges_lpa(self.margin)
    }
    fn padding(&self) -> TRect<LengthPercentage> {
        edges_lp(self.padding)
    }
    fn border(&self) -> TRect<LengthPercentage> {
        edges_lp(self.border)
    }
}

impl FlexboxContainerStyle for LayoutStyle {
    fn flex_direction(&self) -> FlexDirection {
        match self.direction {
            Direction::Row => FlexDirection::Row,
            Direction::Column => FlexDirection::Column,
        }
    }
    fn flex_wrap(&self) -> FlexWrap {
        match self.wrap {
            Wrap::No => FlexWrap::NoWrap,
            Wrap::Yes => FlexWrap::Wrap,
        }
    }
    fn gap(&self) -> TSize<LengthPercentage> {
        TSize {
            width: self.column_gap.length_percentage(),
            height: self.row_gap.length_percentage(),
        }
    }
    fn align_items(&self) -> Option<AlignItems> {
        Some(self.align_items.items())
    }
    fn justify_content(&self) -> Option<AlignContent> {
        Some(self.justify.content())
    }
    fn align_content(&self) -> Option<AlignContent> {
        Some(self.align_content.content())
    }
}

impl FlexboxItemStyle for LayoutStyle {
    fn flex_basis(&self) -> Dimension {
        self.flex_basis.dimension()
    }
    fn flex_grow(&self) -> f32 {
        self.flex_grow
    }
    fn flex_shrink(&self) -> f32 {
        self.flex_shrink
    }
    fn align_self(&self) -> Option<AlignItems> {
        self.align_self.map(Align::items)
    }
}

impl BlockContainerStyle for LayoutStyle {}
impl BlockItemStyle for LayoutStyle {}
