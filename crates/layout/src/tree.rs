//! `LayoutTree`: the view taffy walks. Holds the tree for reading and the
//! module's columns as proxies, and gives them back on drop. Taffy's node id
//! is our node slot.

use app::{App, Comps, CompsMut, NodeId};
use taffy::CacheTree;
use taffy::compute::{
    compute_block_layout, compute_cached_layout, compute_flexbox_layout, compute_hidden_layout,
    compute_leaf_layout,
};
use taffy::geometry::{Point as TPoint, Size as TSize};
use taffy::tree::{
    Layout as TLayout, LayoutBlockContainer, LayoutFlexboxContainer, LayoutInput, LayoutOutput,
    LayoutPartialTree, NodeId as TaffyId, RoundTree, RunMode, TraversePartialTree, TraverseTree,
};
use utils::{Point, Rect, Size};

use crate::style::{Display, LayoutStyle};
use crate::{Layout, LayoutCache, Measure, Unrounded};

pub(crate) struct LayoutTree<'a> {
    app: &'a App,
    /// The live id at each node slot, so taffy's id resolves to one without
    /// reaching into the arena.
    ids: Vec<Option<NodeId>>,
    style: Comps<LayoutStyle>,
    measure: Comps<Measure>,
    layout: CompsMut<Layout>,
    unrounded: CompsMut<Unrounded>,
    cache: CompsMut<LayoutCache>,
}

impl<'a> LayoutTree<'a> {
    pub fn new(app: &'a mut App) -> Self {
        let layout = app
            .components_mut::<Layout>()
            .expect("no Layout holder is alive across a Tick");
        let unrounded = app
            .components_mut::<Unrounded>()
            .expect("layout's private column is never lent out");
        let cache = app
            .components_mut::<LayoutCache>()
            .expect("no LayoutCache holder is alive across a Tick");
        // The proxies do not borrow the app, so what is left to hold is a
        // shared view of the tree.
        let app: &'a App = app;
        let style = app
            .components::<LayoutStyle>()
            .expect("no LayoutStyle writer is alive across a Tick");
        let measure = app
            .components::<Measure>()
            .expect("no Measure writer is alive across a Tick");
        let mut ids = Vec::new();
        for (id, _) in style.iter() {
            let row = id.component_index() as usize;
            if ids.len() <= row {
                ids.resize(row + 1, None);
            }
            ids[row] = Some(id);
        }
        Self {
            app,
            ids,
            style,
            measure,
            layout,
            unrounded,
            cache,
        }
    }

    fn id(&self, node: TaffyId) -> NodeId {
        self.ids[usize::from(node)].expect("taffy only names slots the tree gave it")
    }

    fn children(&self, node: TaffyId) -> &'a [NodeId] {
        self.app.children(self.id(node)).unwrap_or(&[])
    }

    fn style(&self, node: TaffyId) -> &LayoutStyle {
        self.style
            .get(self.id(node))
            .expect("every live node has a style")
    }

    /// Turns each node's rounded, parent-relative box into an absolute one.
    /// Preorder from `id`, whose parent's absolute origin is `origin`.
    pub fn absolutize(&mut self, id: NodeId, origin: Point) {
        let Some(layout) = self.layout.get_mut(id) else {
            return;
        };
        let rect = &mut layout.rect;
        rect.origin = Point::new(origin.x() + rect.x(), origin.y() + rect.y());
        let origin = rect.origin;
        let app = self.app;
        for &child in app.children(id).unwrap_or(&[]) {
            self.absolutize(child, origin);
        }
    }
}

fn taffy_id(id: &NodeId) -> TaffyId {
    TaffyId::from(id.component_index())
}

/// The children of a slot as taffy ids, without allocating.
pub(crate) struct Children<'b>(std::slice::Iter<'b, NodeId>);

impl Iterator for Children<'_> {
    type Item = TaffyId;
    fn next(&mut self) -> Option<TaffyId> {
        self.0.next().map(taffy_id)
    }
}

impl TraversePartialTree for LayoutTree<'_> {
    type ChildIter<'b>
        = Children<'b>
    where
        Self: 'b;

    fn child_ids(&self, parent: TaffyId) -> Self::ChildIter<'_> {
        Children(self.children(parent).iter())
    }

    fn child_count(&self, parent: TaffyId) -> usize {
        self.children(parent).len()
    }

    fn get_child_id(&self, parent: TaffyId, index: usize) -> TaffyId {
        taffy_id(&self.children(parent)[index])
    }
}

impl TraverseTree for LayoutTree<'_> {}

impl LayoutPartialTree for LayoutTree<'_> {
    type CoreContainerStyle<'b>
        = &'b LayoutStyle
    where
        Self: 'b;
    type CustomIdent = String;

    fn get_core_container_style(&self, node: TaffyId) -> &LayoutStyle {
        self.style(node)
    }

    fn set_unrounded_layout(&mut self, node: TaffyId, layout: &TLayout) {
        let id = self.id(node);
        if let Some(unrounded) = self.unrounded.get_mut(id) {
            unrounded.0 = *layout;
        }
    }

    fn compute_child_layout(&mut self, node: TaffyId, inputs: LayoutInput) -> LayoutOutput {
        if inputs.run_mode == RunMode::PerformHiddenLayout {
            return compute_hidden_layout(self, node);
        }
        compute_cached_layout(self, node, inputs, |tree, node, inputs| {
            let has_children = !tree.children(node).is_empty();
            match (tree.style(node).display, has_children) {
                (Display::Hidden, _) => compute_hidden_layout(tree, node),
                (Display::Block, true) => compute_block_layout(tree, node, inputs, None),
                (Display::Flex, true) => compute_flexbox_layout(tree, node, inputs),
                (_, false) => {
                    let id = tree.id(node);
                    let style = tree.style(node);
                    let measured = tree.measure.get(id).and_then(|m| m.0);
                    compute_leaf_layout(
                        inputs,
                        style,
                        |_, _| 0.0,
                        |_, _| match measured {
                            Some(s) => TSize {
                                width: s.width(),
                                height: s.height(),
                            },
                            None => TSize::ZERO,
                        },
                    )
                }
            }
        })
    }
}

impl LayoutFlexboxContainer for LayoutTree<'_> {
    type FlexboxContainerStyle<'b>
        = &'b LayoutStyle
    where
        Self: 'b;
    type FlexboxItemStyle<'b>
        = &'b LayoutStyle
    where
        Self: 'b;

    fn get_flexbox_container_style(&self, node: TaffyId) -> &LayoutStyle {
        self.style(node)
    }
    fn get_flexbox_child_style(&self, child: TaffyId) -> &LayoutStyle {
        self.style(child)
    }
}

impl LayoutBlockContainer for LayoutTree<'_> {
    type BlockContainerStyle<'b>
        = &'b LayoutStyle
    where
        Self: 'b;
    type BlockItemStyle<'b>
        = &'b LayoutStyle
    where
        Self: 'b;

    fn get_block_container_style(&self, node: TaffyId) -> &LayoutStyle {
        self.style(node)
    }
    fn get_block_child_style(&self, child: TaffyId) -> &LayoutStyle {
        self.style(child)
    }
}

impl CacheTree for LayoutTree<'_> {
    fn cache_get(&self, node: TaffyId, input: &LayoutInput) -> Option<LayoutOutput> {
        self.cache.get(self.id(node))?.0.get(input)
    }
    fn cache_store(&mut self, node: TaffyId, input: &LayoutInput, output: LayoutOutput) {
        let id = self.id(node);
        if let Some(cache) = self.cache.get_mut(id) {
            cache.0.store(input, output);
        }
    }
    fn cache_clear(&mut self, node: TaffyId) {
        let id = self.id(node);
        if let Some(cache) = self.cache.get_mut(id) {
            cache.0.clear();
        }
    }
}

impl RoundTree for LayoutTree<'_> {
    fn get_unrounded_layout(&self, node: TaffyId) -> TLayout {
        self.unrounded
            .get(self.id(node))
            .map(|u| u.0)
            .unwrap_or_default()
    }

    /// Writes the rounded box, still relative to the parent; `absolutize`
    /// fixes that up after the whole tree is rounded.
    fn set_final_layout(&mut self, node: TaffyId, layout: &TLayout) {
        let id = self.id(node);
        let TPoint { x, y } = layout.location;
        if let Some(slot) = self.layout.get_mut(id) {
            *slot = Layout {
                rect: Rect {
                    origin: Point::new(x, y),
                    size: Size::new(layout.size.width, layout.size.height),
                },
                padding: utils::edges(
                    layout.padding.top,
                    layout.padding.right,
                    layout.padding.bottom,
                    layout.padding.left,
                ),
                border: utils::edges(
                    layout.border.top,
                    layout.border.right,
                    layout.border.bottom,
                    layout.border.left,
                ),
            };
        }
    }
}
